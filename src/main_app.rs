use models::*;
use crate::*;
use crate::chats_view::*;
use crate::messages_view::*;
use std::{
	vec::Vec,
	io::Stdout,
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
	hint_msg: String,
	right_offset: i32, // cursor offset from the right side of the input string
	input_left_start: i32, // what index of the input string appears at the start of the input box
	last_selected: Option<usize>,
	last_commands: Vec<String>,
	selected_box: DisplayBox,
	quit_app: bool,
	chats_view: ChatsView,
	messages_view: MessagesView,
}

impl MainApp {
	pub fn new() -> MainApp {
		MainApp {
			input_str: String::from(""),
			hint_msg: String::from("type :h to get help :)"),
			right_offset: 0,
			input_left_start: 0,
			last_selected: None,
			last_commands: Vec::new(),
			selected_box: DisplayBox::Chats,
			quit_app: false,
			chats_view: ChatsView::new(),
			messages_view: MessagesView::new(),
		}
	}

	pub fn main_loop(&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), io::Error> {

		let set = SETTINGS.read().unwrap();
		let server = format!("ws{}://{}:{}", if set.secure { "s" } else { "" }, set.host, set.socket_port);
		drop(set);

		spawn(move || {
			let (mut socket, _) =
				tungstenite::connect(url::Url::parse(server.as_str()).unwrap()).expect("Can't connect to websocket :(");

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
								let text_json: serde_json::Map<String, serde_json::Value> = json["text"].as_object().unwrap().to_owned();

								if let Ok(mut set) = SETTINGS.write() {
									set.new_text = Some(text_json);
								}
							},
							&_ => (),
						}
					},
					_ => (),
				}
			}
		});

		let _ = crossterm::terminal::enable_raw_mode();

		// draw, get input, redraw with new state, get input, etc.
		while !self.quit_app {
			self.draw(term)?;

			let _ = self.get_input();
		}

		let _ = crossterm::terminal::disable_raw_mode(); // i just be ignoring results tho

		Ok(())
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
						let help_span = vec![Spans::from(vec![Span::styled(self.hint_msg.as_str(), Style::default().fg(set.colorscheme.hints_box))])];
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
				let new_text = if let Ok(set) = SETTINGS.read() {
					set.new_text.is_some()
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
							KeyCode::Tab => if self.input_str.len() > 0 { 
								self.input_str.push_str("	"); 
							},
							// easy way to cancel what you're typing
							KeyCode::Esc => {
								self.input_str = "".to_string();
								self.hint_msg = "Command cancelled".to_string();
							},
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
				'k' | 'j' => self.scroll(ch == 'j', distance),
				// will add more later
				_ => return,
			}
		}
	}

	fn handle_full_input(&mut self) {
		let mut splits = self.input_str.split(' ').collect::<Vec<&str>>();
		let cmd = splits.drain(0..1).as_slice()[0];
		match cmd {
			":q" | ":Q" => self.quit_app = true,
			":c" | ":C" => {
				if splits.len() > 0 {
					let index = splits[0].parse::<usize>();
					match index {
						Ok(idx) => self.load_in_conversation(idx),
						Err(_) => self.hint_msg = format!("Cannot convert {} to an int", splits[0]),
					}
				} else {
					self.hint_msg = "Please insert an index".to_string();
				}
			},
			":h" | ":H" => { 
				self.selected_box = DisplayBox::Help;
			},
			":s" | ":S" => {
				let cmd = splits.join("%20"); // rust why :(
				self.send_text(Some(cmd), None);
			}
			x => self.hint_msg = format!("Command {} not recognized", x),
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
				self.hint_msg = "Sorry, I haven't implemented scrolling for this box yet :/".to_string();
			},
		}
	}

	fn load_in_conversation(&mut self, idx: usize) {
		// ensure that it's in range
		if idx < self.chats_view.chats.len() {
			self.chats_view.load_in_conversation(idx);
			let id = self.chats_view.chats[idx].chat_identifier.as_str().to_string();

			self.messages_view.load_in_conversation(id);

			self.last_selected = Some(idx);
		} else {
			self.hint_msg = format!("{} is out of range for the chats", idx);
		}
	}

	fn load_in_text(&mut self) {
		if let Ok(set) = SETTINGS.read() {
			let text = match &set.new_text {
				Some(t) => Message::from_json(&t),
				None => {
					self.hint_msg = "You got a new text but we can't parse it, sorry...".to_string();
					return;
				},
			};

			self.chats_view.new_text(&text);

			let id = match &text.chat_identifier {
				Some(c) => c,
				None => {
					self.hint_msg = "You got a new text but it has no chat_identifier... sorry".to_string();
					return;
				},
			};

			if let Some(idx) = self.last_selected {
				if *id == self.chats_view.chats[idx].chat_identifier {
					self.messages_view.new_text(&text);
				}
			}
		}

		if let Ok(mut set) = SETTINGS.write() {
			set.new_text = None;
		}
	}

	fn send_text(&mut self, text: Option<String>, files: Option<String>) {
		if let Some(sel) = self.last_selected {
			let in_files = if let Some(fil) = files { vec![fil] } else { Vec::new() };
			let id = self.chats_view.chats[sel].chat_identifier.to_string();

			let sent = APICLIENT.read().unwrap()
				.send_text(text, None, id, Some(in_files), None);

			self.hint_msg = (if sent { "text sent :)" } else { "text not sent :(" }).to_string();
		}
	}
}

enum DisplayBox {
	Chats,
	Messages,
	Help,
	Compose,
}
