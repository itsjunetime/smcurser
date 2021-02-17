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

pub struct InputView {
	pub input: String,
	pub bounds: (u16, u16),
	pub right_offset: u16,
	pub last_width: u16,
	pub last_commands: Vec<String>,
	pub tabbed_up: Option<u16>,
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
		}
	}

	pub fn draw_view(&mut self, frame: &mut Frame<CrosstermBackend<io::Stdout>>, rect: Rect) {
		let (title, colorscheme) = if let Ok(set) = SETTINGS.read() {
			(set.input_title.to_owned(), Colorscheme::from(&set.colorscheme))
		} else {
			("| input here :) |".to_owned(), Colorscheme::from("forest"))
		};

		if self.last_width != frame.size().width {
			self.last_width = frame.size().width;
			self.bounds.1 = self.input.len() as u16 - self.right_offset;
			self.bounds.0 = std::cmp::max(self.bounds.1 as i32 - self.last_width as i32 - 2, 0) as u16;
		}

		if let Ok(mut state) = STATE.write() {
			state.hint_msg = format!("lw: {}, b1: {}, b0: {}, len: {}, ro: {}",
				self.last_width, self.bounds.1, self.bounds.0, self.input.len(), self.right_offset);
		}

		let render_string = 
			&self.input[self.bounds.0 as usize..std::cmp::min(self.input.len(), self.bounds.1 as usize)];

		let input_span = vec![Spans::from(vec![Span::raw(render_string)])];

		let input_widget = Paragraph::new(input_span)
			.block(
				Block::default()
					.title(title)
					.borders(Borders::ALL)
					.border_type(BorderType::Rounded)
					.border_style(Style::default().fg(colorscheme.unselected_box))
			);
		frame.render_widget(input_widget, rect);

		let cursor_x = std::cmp::min(
			self.last_width - 2,
			self.input.len() as u16 - self.right_offset - self.bounds.0 + 1
		);

		frame.set_cursor(cursor_x, frame.size().height - 3);
	}

	pub fn append_char(&mut self, ch: char) {
		self.input.insert(self.input.len() - self.right_offset as usize, ch);

		self.scroll(true, 0);
	}

	pub fn handle_escape(&mut self) {
		self.input = "".to_owned();
		self.scroll(false, 0);
	}

	pub fn handle_backspace(&mut self) {
		let index = self.input.len() as i32 - self.right_offset as i32 - 1;
		if index > -1 {
			self.input.remove(index as usize);
		}

		self.scroll(false, 0);
	}

	pub fn handle_tab(&mut self) {
		if self.input.len() > 0 &&
			&self.input[..3] != ":f " &&
			&self.input[..3] != ":F " {

			self.input.push_str("	");
		} else {
			self.handle_tab_completion();
		}
	}

	pub fn get_typed_attachments(&self, input: String) -> Vec<String> {
		let bad_chars = [' ', '\t', '"', '\\'];

		let mut files: Vec<String> = Vec::new();
		let mut in_quotes = false;
		let mut escaped = false;
		let mut curr_string: String = "".to_owned();

		for c in input.chars() {
			if !bad_chars.contains(&c) || escaped || (in_quotes && c != '"') {
				curr_string.push(c);
				escaped = false;
			} else {
				if c == '\\' {
					escaped = true;
				} else if c == '"' {
					if in_quotes {
						files.push(curr_string);
						curr_string = "".to_owned();
					}
					in_quotes = !in_quotes;
				} else if c == ' ' || c == '\t' {
					if curr_string.len() > 0 && !in_quotes {
						files.push(curr_string);
						curr_string = "".to_owned();
					}
				}
			}
		}

		if curr_string.len() > 0 {
			files.push(curr_string);
		}

		return files;
	}

	pub fn handle_tab_completion(&mut self) {
		// So this is my messy attempt at tab completion. It actually works ok-ish
		// It doesn't work on Windows rn (I think) since it sees directory separators
		// as '/' instead of '\'.

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
		let incomplete = incomplete_opt.last().unwrap();

		// separate it by "/", join all but last since that is probably
		// the file that hasn't fully been input yet
		let mut top_dirs = incomplete.split(dir_char).collect::<Vec<&str>>();
		let first_file = top_dirs.drain(top_dirs.len() - 1..top_dirs.len())
			.collect::<Vec<&str>>()
			.join("");

		// TODO: Add support for Windows with its weird \ instead of /

		// Here we iterate over the parent directories and make sure they weren't
		// escaping a "/" with a "\" in the file that wasn't fully input yet
		let mut to_drop = 0;

		for c in top_dirs.iter().rev() {
			if c.len() > 0 && c.chars().last().unwrap() == '\\' {
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
				for item in items {
					let path = item.unwrap().path();

					// tmp_path = the file or dir name (including dot
					// between name and extension or trailing slash for directory
					let tmp_path = format!("{}{}{}",
						if let Some(fs) = path.file_stem() {
							fs.to_str().unwrap()
						} else { "" },
						if let Some(ex) = path.extension() {
							format!(".{}", ex.to_str().unwrap())
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
						break;
					}
				}
			},
		}
	}

	pub fn scroll(&mut self, right: bool, distance: u16) {
		if right {
			self.right_offset = std::cmp::max(0, self.right_offset as i32 - distance as i32) as u16;
		} else {
			self.right_offset = std::cmp::min(self.input.len() as u16, self.right_offset + distance);
		}

		// ugh. complex logic. Just suffice it to say this handles setting all these parameters to
		// the correct values for the input field to be pretty
		if self.input.len() as u16 - self.right_offset >= self.last_width - 2 
			&& self.bounds.1 == self.input.len() as u16 - self.right_offset - 1 {

			self.bounds.1 = self.input.len() as u16 - self.right_offset;
			self.bounds.0 = std::cmp::max(self.bounds.1 as i32 - (self.last_width as i32 - 3), 0) as u16;

		} else if self.input.len() as u16 - self.right_offset <= self.bounds.0 {

			self.bounds.0 = self.input.len() as u16 - self.right_offset;
			self.bounds.1 = std::cmp::min(self.bounds.0 + self.last_width - 3, self.input.len() as u16);

		} else if self.last_width - 2 > self.input.len() as u16 {
			self.bounds.1 = self.input.len() as u16;
		}
	}

	pub fn change_command(&mut self, up: bool) {
		if up {
			if self.last_commands.len() > 0 && self.tabbed_up.is_none() {
				self.tabbed_up = Some(0);
				self.input = self.last_commands[0].as_str().to_owned();
			} else if self.last_commands.len() as u16 > self.tabbed_up.unwrap() + 1 {
				self.tabbed_up = Some(self.tabbed_up.unwrap() + 1);
				self.input = self.last_commands[self.tabbed_up.unwrap() as usize]
					.as_str().to_owned();
			}
		} else {
			if let Some(tab) = self.tabbed_up {
				if tab == 0 {
					self.input = "".to_owned();
					self.tabbed_up = None;
				} else {
					self.input = self.last_commands[tab as usize - 1]
						.as_str().to_owned();
					self.tabbed_up = Some(tab - 1);
				}
			}
		}
	}
}
