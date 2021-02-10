use models::*;
use crate::*;
use crate::chats_view::*;
use crate::messages_view::*;
use std::{
	vec::Vec,
	io::{Stdout, Error, ErrorKind},
	thread::spawn,
};
use core::time::Duration;
use tui::{
	layout::{Constraint, Direction, Layout},
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph, Wrap, BorderType},
	style::Style,
};
use crossterm::event::{read, Event, KeyCode, KeyModifiers, poll};

pub struct MainApp {
	input_str: String,
	right_offset: i32, // cursor offset from the right side of the input string
	input_left_start: i32, // what index of the input string appears at the start of the input box
	last_selected: Option<usize>,
	last_commands: Vec<String>,
	tabbed_up: Option<u16>,
	selected_box: DisplayBox,
	quit_app: bool,
	chats_view: ChatsView,
	messages_view: MessagesView,
}

impl MainApp {
	pub fn new() -> MainApp {
		MainApp {
			input_str: String::from(""),
			right_offset: 0,
			input_left_start: 0,
			last_selected: None,
			last_commands: Vec::new(),
			tabbed_up: None,
			selected_box: DisplayBox::Chats,
			quit_app: false,
			chats_view: ChatsView::new(),
			messages_view: MessagesView::new(),
		}
	}

	pub fn main_loop(&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Error> {

		// just to make sure
		let good = APICLIENT.read().unwrap().check_auth();

		if !good {
			println!("Failed to authenticate. Check your password and/or hostname");
			return Err(Error::new(ErrorKind::Other, "Failed to authenticate"));
		}

		self.setup_socket();

		let _ = crossterm::terminal::enable_raw_mode();

		// draw, get input, redraw with new state, get input, etc.
		while !self.quit_app {
			self.draw(term)?;

			let _ = self.get_input();
		}

		let _ = crossterm::terminal::disable_raw_mode(); // i just be ignoring results tho

		Ok(())
	}

	fn setup_socket(&mut self) {
		let set = SETTINGS.read().unwrap();
		let host = set.host.as_str().to_owned();
		let port = set.socket_port;
		let sec = set.secure;
		drop(set);

		spawn(move || {
			let config = Some(tungstenite::protocol::WebSocketConfig {
				max_send_queue: None,
				max_message_size: None,
				max_frame_size: None,
				accept_unmasked_frames: false
			});

			let connector = native_tls::TlsConnector::builder()
				.danger_accept_invalid_certs(true)
				.danger_accept_invalid_hostnames(true)
				.build()
				.unwrap();

			let stream = std::net::TcpStream::connect(format!("{}:{}", host, port)).unwrap();
			let tls_stream = connector.connect(&host, stream).unwrap();
			let parsed_url = url::Url::parse(
				&format!("ws{}://{}:{}", if sec { "s" } else { "" }, host, port)
			).unwrap();

			let sock_res = tungstenite::client::client_with_config(
				parsed_url,
				tls_stream,
				config
			);

			match sock_res {
				Ok((mut socket, _)) => {
					loop {
						let msg = socket.read_message().expect("Error reading websocket message");
						match msg {
							tungstenite::Message::Text(val) => {
								let mut splits = val.splitn(2, ':');
								let prefix = splits.next().unwrap();
								let content = splits.next().unwrap();

								match prefix {
									"text" => {
										let json: serde_json::Value = serde_json::from_str(&content).unwrap();
										let text_json: serde_json::Map<String, serde_json::Value> =
											json["text"].as_object().unwrap().to_owned();

										if let Ok(mut state) = STATE.write() {
											state.new_text = Some(text_json);
										}
									},
									&_ => (),
								}
							},
							_ => (),
						}
					}
				},
				Err(x) => {
					if let Ok(mut state) = STATE.write() {
						state.hint_msg = format!("Error: Failed to connect to websocket: {} New messages will not show.", x);
					}
				}
			};

		});
	}

	pub fn draw(&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), io::Error> {
		// gotta make sure we can actually access the settings
		if let Ok(set) = SETTINGS.read() {

			// this just draws the actual tui display.
			term.draw(|f| {
				let size = f.size();

				match self.selected_box {
					DisplayBox::Help => {
						// if we're showing the help box, just draw the help box and nothing else
						let text: Vec<Spans> = HELP_MSG.iter().map(|m| Spans::from(vec![Span::raw(*m)])).collect();
						let help_msg_widget = Paragraph::new(text)
							.block(Block::default().title(set.help_title.as_str()).borders(Borders::ALL))
							.wrap(Wrap { trim: true });

						f.render_widget(help_msg_widget, size);
					},
					DisplayBox::Chats | DisplayBox::Messages | DisplayBox::Compose => {
						// set up layouts
						let main_layout = Layout::default()
							.direction(Direction::Vertical)
							.constraints(
								[
									Constraint::Min(5),
									Constraint::Length(3),
									Constraint::Length(1),
								].as_ref()
							).split(size);

						let content_layout = Layout::default()
							.direction(Direction::Horizontal)
							.constraints(
								[
									Constraint::Percentage(30),
									Constraint::Percentage(70),
								].as_ref()
							)
							.split(main_layout[0]);

						let chats_selected = if let DisplayBox::Chats = self.selected_box { true } else { false };

						self.chats_view.draw_view(f, content_layout[0], chats_selected);

						self.messages_view.draw_view(f, content_layout[1], !chats_selected);

						// create a span for the input box and add the border
						let input_span = vec![Spans::from(vec![Span::raw(self.input_str.as_str())])];
						let input_widget = Paragraph::new(input_span)
							.block(
								Block::default()
									.title(set.input_title.as_str())
									.borders(Borders::ALL)
									.border_type(BorderType::Rounded)
									.border_style(Style::default().fg(set.colorscheme.unselected_box))
							);
						f.render_widget(input_widget, main_layout[1]);

						f.set_cursor(self.input_str.len() as u16 + 1 - self.right_offset as u16, size.height - 3);

						// create a span for the help box add the help string
						let hint_msg = if let Ok(state) = STATE.read() {
							state.hint_msg.as_str().to_string()
						} else {
							"type :h to get help :)".to_string()
						};
						let help_span = vec![Spans::from(vec![Span::styled(hint_msg, Style::default().fg(set.colorscheme.hints_box))])];
						let help_widget = Paragraph::new(help_span);
						f.render_widget(help_widget, main_layout[2]);
					}
				}
			})?;
		}

		Ok(())
	}

	fn get_input(&mut self) -> crossterm::Result<()> {
		// we have to loop this so that if it gets a character/input we don't want,
		// we can just grab the next character/input instead.

		let mut distance = "".to_string();

		loop {
			if !poll(Duration::from_millis(20)).unwrap() {
				let new_text = if let Ok(state) = STATE.read() {
					state.new_text.is_some()
				} else {
					false
				};

				if new_text {
					self.load_in_text();
					break;
				}
			} else {
				match read()? {
					Event::Key(event) => {
						match event.code {
							KeyCode::Backspace => {
								if self.input_str.len() > 0 {
									let index = self.input_str.len() as i32 - self.right_offset - 1;
									if index > -1 { self.input_str.remove(index as usize); }
								}
							},
							KeyCode::Enter => if self.input_str.len() > 0 { self.handle_full_input() },
							// left and right move the cursor if there's input in the box, else they
							// just switch which box is selected
							KeyCode::Left | KeyCode::Right => {
								if self.input_str.len() > 0 {
									if let KeyCode::Left = event.code {
										self.right_offset = std::cmp::min(self.input_str.len() as i32, self.right_offset + 1);
									} else {
										self.right_offset = std::cmp::max(0, self.right_offset - 1);
									}
								} else {
									self.switch_selected_box();
								}
							},
							// will add tab completion for file selection later
							KeyCode::Tab => if self.input_str.len() > 0 &&
								&self.input_str[..3] != ":f " &&
								&self.input_str[..3] != ":F " {

								self.input_str.push_str("	");

							} else {
								self.handle_tab_completion();
							},
							// easy way to cancel what you're typing
							KeyCode::Esc => {
								self.input_str = "".to_string();
								if let Ok(mut state) = STATE.write() {
									state.hint_msg = "Command cancelled".to_string();
								}
							},
							KeyCode::Up => {
								if self.last_commands.len() > 0 && self.tabbed_up.is_none() {
									self.tabbed_up = Some(0);
									self.input_str = self.last_commands[0].as_str().to_owned();
								} else if self.last_commands.len() as u16 > self.tabbed_up.unwrap() + 1 {
									self.tabbed_up = Some(self.tabbed_up.unwrap() + 1);
									self.input_str = self.last_commands[self.tabbed_up.unwrap() as usize]
										.as_str().to_owned();
								}
							},
							KeyCode::Down => {
								if let Some(tab) = self.tabbed_up {
									if tab == 0 {
										self.input_str = "".to_owned();
										self.tabbed_up = None;
									} else {
										self.input_str = self.last_commands[tab as usize - 1]
											.as_str().to_owned();
										self.tabbed_up = Some(tab - 1);
									}
								}
							}
							// ctrl+c gets hijacked by crossterm, so I wanted to manually add in a way
							// for people to invoke it to exit if that's what they're used to.
							KeyCode::Char(c) => {
								if event.modifiers == KeyModifiers::CONTROL && c == 'c' {
									self.quit_app = true;
								} else if c.is_digit(10) && self.input_str.len() == 0 {

									// test for digits to allow for vim-like scrolling, multiple lines
									// at once.
									distance = format!("{}{}", distance, c);
									continue;

								} else {
									let dist: u16 = if distance.len() == 0 {
										1
									} else {
										distance.parse().unwrap_or_else(|_| 1 )
									};

									self.handle_input_char(c, dist);
								}
							}
							_ => continue,
						};
						break
					},
					_ => continue,
				}
			}
		}

		Ok(())
	}

	fn handle_input_char(&mut self, ch: char, distance: u16) {
		if self.input_str.len() > 0 || ch == ':' {
			self.input_str.insert(self.input_str.len() - self.right_offset as usize, ch);
		} else {
			match ch {
				'h' | 'l' => self.switch_selected_box(),
				// quit out of help display if it is showing
				'q' | 'Q' => if let DisplayBox::Help = self.selected_box {
					self.selected_box = DisplayBox::Chats;
				},
				// scroll up or down in the selected box
				'k' | 'j' => self.scroll(ch == 'k', distance),
				// will add more later
				_ => return,
			}
		}
	}

	fn handle_full_input(&mut self) {
		self.last_commands.insert(0, self.input_str.as_str().to_owned());

		let mut splits = self.input_str.split(' ').collect::<Vec<&str>>();
		let cmd = splits.drain(0..1).as_slice()[0];
		match cmd {
			":q" | ":Q" => self.quit_app = true,
			":c" | ":C" => {
				if splits.len() > 0 {
					let index = splits[0].parse::<usize>();
					match index {
						Ok(idx) => self.load_in_conversation(idx),
						Err(_) => {
							if let Ok(mut state) = STATE.write() {
								state.hint_msg = format!("Cannot convert {} to an int", splits[0]);
							}
						}
					}
				} else if let Ok(mut state) = STATE.write() {
					state.hint_msg = "Please insert an index".to_string();
				}
			},
			":h" | ":H" => self.selected_box = DisplayBox::Help,
			":s" | ":S" => {
				let cmd = splits.join(" ");
				self.send_text(Some(cmd), None);
			},
			":r" | ":R" => self.chats_view.reload_chats(),
			":b" | ":B" => {
				let ops = splits.iter().map(|o| o.to_string()).collect::<Vec<String>>();
				self.bind_var(ops);
			},
			":a" | ":A" => {
				if splits.len() > 0 {
					let index = splits[0].parse::<usize>();
					match index {
						Ok(idx) => self.messages_view.open_attachment(idx),
						Err(_) => {
							if let Ok(mut state) = STATE.write() {
								state.hint_msg = format!("Cannot convert {} to an int", splits[0]);
							}
						}
					}
				} else if let Ok(mut state) = STATE.write() {
					state.hint_msg = "Please insert an index".to_string();
				}
			},
			":f" | ":F" => self.send_attachments(splits),
			x => {
				if let Ok(mut state) = STATE.write() {
					state.hint_msg = format!("Command {} not recognized", x);
				}
			}
		};

		self.input_str = "".to_string();
		self.right_offset = 0;
	}

	fn switch_selected_box(&mut self) {
		if let DisplayBox::Chats = self.selected_box {
			self.selected_box = DisplayBox::Messages;
		} else if let DisplayBox::Messages = self.selected_box {
			self.selected_box = DisplayBox::Chats;
		}
	}

	fn scroll(&mut self, up: bool, distance: u16) {
		match self.selected_box {
			DisplayBox::Chats => self.chats_view.scroll(up, distance),
			DisplayBox::Messages => self.messages_view.scroll(up, distance),
			_ => {
				if let Ok(mut state) = STATE.write() {
					state.hint_msg = "Sorry, I haven't implemented scrolling for this box yet :/".to_string();
				}
			},
		}
	}

	fn load_in_conversation(&mut self, idx: usize) {
		// ensure that it's in range
		if idx < self.chats_view.chats.len() {
			self.chats_view.load_in_conversation(idx);
			let id = self.chats_view.chats[idx].chat_identifier.as_str().to_string();

			self.messages_view.load_in_conversation(&id);

			self.last_selected = Some(idx);

			if let Ok(mut state) = STATE.write() {
				state.current_chat = Some(id);
				state.hint_msg = "loaded in chat :)".to_string();
			}
		} else if let Ok(mut state) = STATE.write() {
			state.hint_msg = format!("{} is out of range for the chats", idx);
		}
	}

	fn load_in_text(&mut self) {
		let text_opt = if let Ok(state) = STATE.read() {
			if let Some(text_map) = &state.new_text {
				Some(Message::from_json(&text_map))
			} else { None }
		} else { None };

		if let Some(text) = text_opt {
			// new_text returns the previous index of the conversation in which the new text was
			// sent. We can use it to determine how to shift self.last_selected
			let past = self.chats_view.new_text(&text);

			if let Some(idx) = past {
				if let Some(ls) = self.last_selected {
					if idx == ls {
						self.last_selected = Some(0);
						self.messages_view.new_text(text);
					} else if idx > ls {
						self.last_selected = Some(ls + 1);
					}
				}
			}
		}

		if let Ok(mut state) = STATE.write() {
			state.new_text = None;
		}
	}

	fn send_text(&self, text: Option<String>, files: Option<Vec<String>>) {
		if let Some(sel) = self.last_selected {
			let in_files = if let Some(fil) = files {
				fil
			} else {
				Vec::new()
			};

			let id = self.chats_view.chats[sel]
				.chat_identifier
				.to_string();

			let sent = APICLIENT.read().unwrap()
				.send_text(text, None, id, Some(in_files), None);

			if let Ok(mut state) = STATE.write() {
				state.hint_msg = (if sent {
					"text sent :)"
				} else {
					"text not sent :("
				}).to_string();
			}
		}
	}

	fn send_attachments(&self, files: Vec<&str>) {
		let orig = files.join(" ");

		let files_to_send = self.get_typed_attachments(orig);

		self.send_text(None, Some(files_to_send));
	}

	fn get_typed_attachments(&self, input: String) -> Vec<String> {
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

	fn bind_var(&mut self, ops: Vec<String>) {
		if ops.len() < 2 {
			if let Ok(mut state) = STATE.write() {
				state.hint_msg = "Please enter at least a variable name and value".to_string();
			}
			return;
		}

		let mut new_ops = ops;
		let val = new_ops.split_off(1);
		new_ops.push(val.join(" "));

		if let Ok(mut set) = SETTINGS.write() {
			set.parse_args(new_ops, true);
		}
	}

	fn handle_tab_completion(&mut self) {
		// So this is my messy attempt at tab completion. It actually works ok-ish
		// It doesn't work on Windows rn (I think) since it sees directory separators
		// as '/' instead of '\'.

		let mut splits = self.input_str.split(" ").collect::<Vec<&str>>();
		splits.remove(0);
		let input = splits.join(" ");

		// this gets a list of the currently input attachments,
		// with support for escaping spaces with backslashes and quotes
		let incomplete_opt = self.get_typed_attachments(input);

		// if there are no attachments input, just exit
		if incomplete_opt.len() == 0 {
			return;
		}

		// get the path for the attachment that hasn't fully been input yet
		let incomplete = incomplete_opt.last().unwrap();

		// separate it by "/", join all but last since that is probably
		// the file that hasn't fully been input yet
		let mut top_dirs = incomplete.split("/").collect::<Vec<&str>>();
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
			if to_drop > 0 { "/" } else { "" },
			first_file
		);

		// dir = the whole parent directory for the file they were entering
		let dir = top_dirs.join("/");
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
							"/"
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
						let full_path = format!("{}/{}", dir, path_str);

						self.input_str.truncate(self.input_str.len() - incomplete.len());
						self.input_str = format!("{}{}", self.input_str, full_path);
						break;
					}
				}
			},
		}
	}
}

enum DisplayBox {
	Chats,
	Messages,
	Help,
	Compose,
}
