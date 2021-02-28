use crate::{
	*,
	colorscheme::*,
};
use tui::{
	layout::Rect,
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph, BorderType},
	style::Style,
	terminal::Frame,
};
use std::vec::Vec;
use crossterm::event::KeyCode;

pub struct InputView {
	pub input: String, // the text that is input to this view
	pub bounds: (u16, u16), // the substring of the input that is shown
	pub right_offset: u16, // the cursor's offset from the right side of the input
	pub last_width: u16, // last width that the view recorded. Since input views are always one line, height changes don't affect them.
	pub last_commands: Vec<String>, // last commands that were input, so that you can tab up through them
	pub tabbed_up: Option<u16>, // how far tabbed up through the most recent commands you are
	pub custom_title: Option<String>, // custom title to display with this
}

impl InputView {
	pub fn new() -> InputView {
		InputView {
			input: "".to_owned(),
			bounds: (0, 0),
			right_offset: 0,
			last_width: 0,
			last_commands: Vec::new(),
			tabbed_up: None,
			custom_title: None,
		}
	}

	pub fn draw_view(&mut self, frame: &mut Frame<CrosstermBackend<io::Stdout>>, rect: Rect, selected: bool) {
		// get the colorscheme
		let (mut title, colorscheme) = if let Ok(set) = SETTINGS.read() {
			(set.input_title.to_owned(), Colorscheme::from(&set.colorscheme))
		} else {
			("| input here :) |".to_owned(), Colorscheme::from("forest"))
		};

		// and the title
		if let Some(custom) = &self.custom_title {
			title = custom.to_owned();
		}

		// if it's not the same width, the terminal has been resized. Reset some stuff so that
		// everything doesn't spazz out when you try to draw it.
		if self.last_width != frame.size().width {
			self.last_width = frame.size().width;
			self.bounds.1 = self.input.len() as u16 - self.right_offset;
			self.bounds.0 = std::cmp::max(self.bounds.1 as i32 - self.last_width as i32 - 2, 0) as u16;
		}

		// get the substring that will be rendered
		let render_string =
			&self.input[self.bounds.0 as usize..std::cmp::min(self.input.len(), self.bounds.1 as usize)];

		let input_span = vec![Spans::from(vec![Span::raw(render_string)])];

		let input_widget = Paragraph::new(input_span)
			.block(
				Block::default()
					.title(title)
					.borders(Borders::ALL)
					.border_type(BorderType::Rounded)
					.border_style(Style::default().fg(
						if selected {
							colorscheme.selected_box
						} else {
							colorscheme.unselected_box
						}
					))
			);
		frame.render_widget(input_widget, rect);

		// not perfect logic but works with the flow of drawing in SMCurser to make everything look nice
		if self.input.len() > 0 {
			let cursor_x = std::cmp::min(
				self.last_width - 2,
				self.input.len() as u16 - self.right_offset - self.bounds.0 + 1
			);

			frame.set_cursor(rect.x + cursor_x, rect.y + 1);
		}
	}

	pub fn route_keycode(&mut self, code: KeyCode) {
		// just decide to which function the specified keycode should go
		match code {
			KeyCode::Backspace => self.handle_backspace(),
			KeyCode::Esc => self.handle_escape(),
			KeyCode::Tab => self.handle_tab(),
			_ => (),
		}
	}

	pub fn append_char(&mut self, ch: char) {
		// input it at the specified place
		self.input.insert(self.input.len() - self.right_offset as usize, ch);

		// scroll 0. This makes sure that the string will display nicely when redrawn
		self.scroll(true, 0);
	}

	pub fn handle_escape(&mut self) {
		self.input = "".to_owned();

		// once again, makes sure that the input will display nicely when redrawn
		self.scroll(false, 0);
	}

	pub fn handle_backspace(&mut self) {
		let index = self.input.len() as i32 - self.right_offset as i32 - 1;
		if index > -1 {
			self.input.remove(index as usize);
		}

		// same
		self.scroll(false, 0);
	}

	pub fn handle_tab(&mut self) {
		// if the  first 3 characters are `:f ` or `:F `, then they're pressing tab to get file
		// path completion. Handle that separately.
		if self.input.len() > 0 &&
			&self.input[..3].to_lowercase() != ":f " {

			self.input.push_str("	");
		} else {
			self.handle_tab_completion();
		}
	}

	pub fn get_typed_attachments(&self, input: String) -> Vec<String> {
		// parse the string that is input and get the list of attachments that they have currently
		// typed out the paths of. We have to use special parsing for this so that people can
		// escape spaces with backslashes and quotes
		let bad_chars = [' ', '\t', '"', '\\'];
		let win_bad_chars = [' ', '\t'];

		let mut files: Vec<String> = Vec::new();
		let mut in_quotes = false;
		let mut escaped = false;
		let mut curr_string: String = "".to_owned();

		// go through each character one by one
		for c in input.chars() {

			// first, check if this character should be inserted as-is. If it's a regular
			// character, not in the `bad_chars` array, it's good to go. Also, if this character is
			// escaped with a backslash, it's good. It's also good if the current string is quoted
			// and they're not trying to end the quotation
			if !bad_chars.contains(&c) || escaped || (in_quotes && c != '"') {

				// have to do special parsing for windows here, since their path delimiters are
				// backslashes, as opposed to forward slashes.
				if cfg!(windows) && escaped && !win_bad_chars.contains(&c) {
					curr_string.push('\\');
				}

				// push it onto the list!
				curr_string.push(c);
				escaped = false;
			} else {
				// if it's backslash, just let the next character in as part of the path, no matter
				// what it is.
				if c == '\\' {
					escaped = true;
				} else if c == '"' {
					// if they're trying to end the quotes, then they're starting to list a new
					// file. push the current file and reset the current string
					if in_quotes {
						files.push(curr_string);
						curr_string = "".to_owned();
					}

					// and invert in_quotes no matter what
					in_quotes = !in_quotes;
				} else if c == ' ' || c == '\t' {
					// if you get here, it's whitespace which is not escaped. They're ending one
					// file entry and starting another; however, we have to make sure they've
					// actually input part of a file before pushing it to the list
					if curr_string.len() > 0 {
						files.push(curr_string);
						curr_string = "".to_owned();
					}
				}
			}
		}

		// push the current string where it's at
		if curr_string.len() > 0 {
			files.push(curr_string);
		}

		return files;
	}

	pub fn handle_tab_completion(&mut self) {
		// So this is my messy attempt at tab completion. It actually works ok-ish
		// I think it works on Windows but I can't say for certain

		let mut splits = self.input.split(" ").collect::<Vec<&str>>();
		splits.remove(0);
		let input = splits.join(" ");

		// this gets a list of the currently input attachments,
		// with support for escaping spaces with backslashes and quotes
		let incomplete_opt = self.get_typed_attachments(input);

		// if there are no attachments input, just exit
		if incomplete_opt.len() == 0 {
			return;
		}

		let dir_char = if cfg!(windows) {
			"\\"
		} else {
			"/"
		};

		// get the path for the attachment that hasn't fully been input yet
		let incomplete = incomplete_opt.last()
			.expect("Couldn't get last character of incomplete_opt");

		// separate it by "/", join all but last since that is probably
		// the file that hasn't fully been input yet
		let mut top_dirs = incomplete.split(dir_char).collect::<Vec<&str>>();
		let first_file = top_dirs.drain(top_dirs.len() - 1..top_dirs.len())
			.collect::<Vec<&str>>()
			.join("");

		// Here we iterate over the parent directories and make sure they weren't
		// escaping a "/" with a "\" in the file that wasn't fully input yet
		let mut to_drop = 0;

		for c in top_dirs.iter().rev() {
			if c.len() > 0 && c.chars().last().unwrap_or('-') == '\\' {
				to_drop += 1;
			} else {
				break;
			}
		}

		// Set poss_files to the beginning of the file that they
		// may have been trying to escape
		let poss_files = if to_drop > 0 {
			top_dirs.drain(top_dirs.len() - to_drop..top_dirs.len())
				.collect::<Vec<&str>>()
				.join("")
		} else {
			"".to_owned()
		};

		// Set file to the whole untyped file name, including the possibly escaped sections
		let file = format!("{}{}{}",
			poss_files,
			if to_drop > 0 { dir_char } else { "" },
			first_file
		);

		// dir = the whole parent directory for the file they were entering
		let dir = top_dirs.join(dir_char);
		let dir_contents = std::fs::read_dir(&dir);

		match dir_contents {
			Err(_) => return,
			Ok(items) => {
				for it in items {
					if let Ok(item) = it {
						let path = item.path();

						// tmp_path = the file or dir name (including dot
						// between name and extension or trailing slash for directory
						let tmp_path = format!("{}{}{}",
							if let Some(fs) = path.file_stem() {
								fs.to_str().unwrap_or("")
							} else { "" },
							if let Some(ex) = path.extension() {
								format!(".{}", ex.to_str().unwrap_or(""))
							} else { "".to_owned() },
							if path.is_dir() {
								dir_char
							} else { "" }
						);

						let path_str = tmp_path.as_str();

						// if the file that is currently being iterated over is the same length or
						// shorter than what they've input, don't even try to match it
						if path_str.len() <= file.len() {
							continue
						}

						// If it's a possibility for the file they were trying to input, auto-fill the
						// input string with the whole file path
						if path_str[..file.len()] == file {
							let full_path = format!("{}{}{}", dir, dir_char, path_str);

							self.input.truncate(self.input.len() - incomplete.len());
							self.input = format!("{}{}", self.input, full_path);
							
							self.last_width = 0;
							break;
						}
					}
				}
			},
		}
	}

	pub fn scroll(&mut self, right: bool, distance: u16) {
		// this is the actual scrolling part
		if right {
			self.right_offset = std::cmp::max(0, self.right_offset as i32 - distance as i32) as u16;
		} else {
			self.right_offset = std::cmp::min(self.input.len() as u16, self.right_offset + distance);
		}

		// and this is the part that handles setting other variables to make sure it displays
		// nicely on the next redraw. Just suffice it to say this handles setting all these parameters to
		// the correct values for the input field to be pretty

		let greater_than_view = self.input.len() as u16 - self.right_offset >= self.last_width - 2;
		let bound_at_end = self.bounds.1 == self.input.len() as u16 - self.right_offset - 1;

		let cursor_in_middle = self.input.len() as u16 - self.right_offset <= self.bounds.0;

		let less_than_view = self.last_width - 2 > self.input.len() as u16;

		if greater_than_view && bound_at_end {

			// set it so that the cursor will be at the farthest right end of the drawn input view
			self.bounds.1 = self.input.len() as u16 - self.right_offset;
			self.bounds.0 = std::cmp::max(self.bounds.1 as i32 - (self.last_width as i32 - 3), 0) as u16;

		} else if cursor_in_middle {

			// sets the cursor to the leftmost end of the drawn input view
			self.bounds.0 = self.input.len() as u16 - self.right_offset;
			self.bounds.1 = std::cmp::min(self.bounds.0 + self.last_width - 3, self.input.len() as u16);

		} else if less_than_view {
			// just sets the bounds to the full string, basically, since its length is less than
			// the length of the view that it will be drawn in.
			self.bounds.1 = self.input.len() as u16;
		}
	}

	pub fn change_command(&mut self, up: bool) {
		// this handles tabbing up through recent commands
		if up {
			// if tabbing up, to older commands
			match self.tabbed_up {
				None => if self.last_commands.len() > 0 {
					// if we haven't tabbed up at all, set it to 0 and grab the command
					self.tabbed_up = Some(0);
					self.input = self.last_commands[0].as_str().to_owned();
				},
				Some(tu) => if self.last_commands.len() as u16 > tu + 1 {
					// if we tabbed up and we can still do so more, do so.
					self.tabbed_up = Some(tu + 1);
					self.input = self.last_commands[tu as usize]
						.as_str().to_owned();
				}
			}
		} else {
			// if tabbing down, to more recent commands
			if let Some(tab) = self.tabbed_up {
				// only do something if we've already tabbed up somewhat
				if tab == 0 {
					// if it's 0, reset the input to nothing.
					self.input = "".to_owned();
					self.tabbed_up = None;
				} else {
					// else just go one further down the list
					self.input = self.last_commands[tab as usize - 1]
						.as_str().to_owned();
					self.tabbed_up = Some(tab - 1);
				}
			}
		}
	}
}
