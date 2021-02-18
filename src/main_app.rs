use models::*;
use crate::{
	*,
	chats_view::*,
	messages_view::*,
	input_view::*,
	colorscheme::*,
};
use std::{
	vec::Vec,
	io::{Stdout, Error, ErrorKind},
	thread::{spawn, sleep},
};
use core::time::Duration;
use tui::{
	layout::{Constraint, Direction, Layout},
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph, Wrap},
	style::Style,
};
use crossterm::event::{read, Event, KeyCode, KeyModifiers, poll};

pub struct MainApp {
	last_selected: Option<usize>,
	selected_box: DisplayBox,
	quit_app: bool,
	redraw_all: bool,
	chats_view: ChatsView,
	messages_view: MessagesView,
	input_view: InputView,
}

impl MainApp {
	pub fn new() -> MainApp {
		MainApp {
			last_selected: None,
			selected_box: DisplayBox::Chats,
			quit_app: false,
			redraw_all: false,
			chats_view: ChatsView::new(),
			messages_view: MessagesView::new(),
			input_view: InputView::new(),
		}
	}

	pub fn main_loop(&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Error> {

		if let Ok(set) = SETTINGS.read() {
			if set.host.len() == 0 {
				eprintln!("You didn't specify a host to connect to. Please either edit your config file ({}) to include a host \
					or pass one in after the \x1b[1m--host\x1b[0m flag", set.config_file);
				return Err(Error::new(ErrorKind::Other, "No specified host"));
			}
		}

		if !APICLIENT.check_auth() {
			eprintln!("Failed to authenticate. Trying fallback host...");
			if let Ok(mut set) = SETTINGS.write() {
				set.host = set.fallback_host.to_owned();
			}

			if !APICLIENT.check_auth() {
				eprintln!("Failed to authenticate with both hosts. Please check your configuration.");
				return Err(Error::new(ErrorKind::Other, "Failed to authenticate"));
			}
		}

		self.setup_socket();

		let _ = crossterm::terminal::enable_raw_mode();

		// draw, get input, redraw with new state, get input, etc.
		while !self.quit_app {
			self.draw(term)?;

			let _ = self.get_input();

			if self.redraw_all {
				term.resize(term.size()?)?;
				self.redraw_all = false;
			}
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
			let sock_res_template = | | {
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

				tungstenite::client::client_with_config(
					parsed_url,
					tls_stream,
					config
				)
			};

			let mut sock_res = sock_res_template();

			loop {
				match sock_res {
					Ok((mut socket, _)) => {
						loop {
							let msg = socket.read_message();
							match msg {
								Ok(tungstenite::Message::Text(val)) => {
									let mut splits = val.splitn(2, ':');
									let prefix = splits.next().unwrap();
									let content = splits.next().unwrap();

									match prefix {
										"text" => {
											let json: serde_json::Value = serde_json::from_str(&content).unwrap();
											let text_json: serde_json::Map<String, serde_json::Value> =
												json["text"].as_object().unwrap().to_owned();

											if let Ok(mut state) = STATE.write() {
												state.new_text = Some(Message::from_json(&text_json));
											}
										},
										"typing" => {
											let typing_text = Message::typing(content);

											if let Ok(mut state) = STATE.write() {
												state.new_text = Some(typing_text);
											}
										},
										"idle" => {
											let idle_text = Message::idle(content);

											if let Ok(mut state) = STATE.write() {
												state.new_text = Some(idle_text);
											}
										},
										&_ => (),
									}
								},
								Err(_) => {
									sleep(Duration::from_secs(2));
									sock_res = sock_res_template();
									break;
								},
								_ => (),
							}
						}
					},
					Err(ref x) => {
						if let Ok(mut state) = STATE.write() {
							state.hint_msg = format!("Error: Failed to connect to websocket: {} New messages will not show.", x);
						}
					}
				};
			}

		});
	}

	pub fn draw(&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), io::Error> {
		// gotta make sure we can actually access the settings
		let (help_title, colorscheme) = if let Ok(set) = SETTINGS.read() {
			(set.help_title.to_owned(), Colorscheme::from(&set.colorscheme))
		} else {
			("| help |".to_owned(), Colorscheme::from("forest"))
		};

		// this just draws the actual tui display.
		term.draw(|f| {
			let size = f.size();

			match self.selected_box {
				DisplayBox::Help => {
					// if we're showing the help box, just draw the help box and nothing else
					let text: Vec<Spans> = HELP_MSG.iter().map(|m| Spans::from(vec![Span::raw(*m)])).collect();
					let help_msg_widget = Paragraph::new(text)
						.block(Block::default().title(help_title).borders(Borders::ALL))
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

					let chats_selected = self.selected_box == DisplayBox::Chats;

					self.chats_view.draw_view(f, content_layout[0], chats_selected);
					self.messages_view.draw_view(f, content_layout[1], !chats_selected);
					self.input_view.draw_view(f, main_layout[1]);

					// create a span for the help box add the help string
					let hint_msg = if let Ok(state) = STATE.read() {
						state.hint_msg.as_str().to_string()
					} else {
						"type :h to get help :)".to_string()
					};

					let help_span = vec![Spans::from(vec![Span::styled(hint_msg, Style::default().fg(colorscheme.hints_box))])];
					let help_widget = Paragraph::new(help_span);
					f.render_widget(help_widget, main_layout[2]);
				}
			}
		})?;

		Ok(())
	}

	fn get_input(&mut self) -> crossterm::Result<()> {
		// we have to loop this so that if it gets a character/input we don't want,
		// we can just grab the next character/input instead.
		let mut distance = "".to_string();

		loop {
			if !poll(Duration::from_millis(20)).unwrap() {
				// first check if there's actually an unread text
				let has_unread = if let Ok(state) = STATE.read() {
					state.new_text.is_some()
				} else { false };

				if has_unread {
					if let Ok(mut state) = STATE.write() {
						// swap the new text out for `None`
						let mut none_text: Option<Message> = None;
						std::mem::swap(&mut none_text, &mut state.new_text);

						// send the new text to the load in function
						if let Some(txt) = none_text {
							self.load_in_text(txt);
						}
						break;
					}
				}
			} else {
				match read()? {
					Event::Key(event) => {
						match event.code {
							KeyCode::Backspace => self.input_view.handle_backspace(),
							KeyCode::Enter => if self.input_view.input.len() > 0 { self.handle_full_input() },
							// left and right move the cursor if there's input in the box, else they
							// just switch which box is selected
							KeyCode::Left | KeyCode::Right => {
								if self.input_view.input.len() > 0 {
									let right = if let KeyCode::Right = event.code { true } else { false };
									let dist: u16 = distance.parse().unwrap_or(1);
									self.input_view.scroll(right, dist);
								} else {
									self.switch_selected_box();
								}
							},
							// will add tab completion for file selection later
							KeyCode::Tab => self.input_view.handle_tab(),
							// easy way to cancel what you're typing
							KeyCode::Esc => {
								self.input_view.handle_escape();
								if let Ok(mut state) = STATE.write() {
									state.hint_msg = "Command cancelled".to_string();
								}
							},
							KeyCode::Up | KeyCode::Down =>
								self.input_view.change_command(event.code == KeyCode::Up),
							// ctrl+c gets hijacked by crossterm, so I wanted to manually add in a way
							// for people to invoke it to exit if that's what they're used to.
							KeyCode::Char(c) => {
								if event.modifiers == KeyModifiers::CONTROL && c == 'c' {
									self.quit_app = true;
								} else if c.is_digit(10) && self.input_view.input.len() == 0 {

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
		if self.input_view.input.len() > 0 || ch == ':' {
			self.input_view.append_char(ch);
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
		self.input_view.last_commands
			.insert(0, self.input_view.input.as_str().to_owned());

		let mut splits = self.input_view.input.split(' ').collect::<Vec<&str>>();
		let cmd = splits.drain(0..1).as_slice()[0];
		match cmd.to_lowercase().as_str() {
			":q" => self.quit_app = true,
			":c" => {
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
			":h" => self.selected_box = DisplayBox::Help,
			":s" => {
				let cmd = splits.join(" ");
				self.send_text(Some(cmd), None);
			},
			":r" => {
				self.redraw_all = true;
				self.chats_view.reload_chats();
			},
			":b" => {
				let ops = splits.iter().map(|o| o.to_string()).collect::<Vec<String>>();
				self.bind_var(ops);
			},
			":a" => {
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
			":f" => self.send_attachments(splits),
			":t" => {
				let tapback = splits.join("");
				self.send_tapback(&tapback);
			},
			":dt" => {
				if let Some(ls) = self.last_selected {
					let chat = &self.chats_view.chats[ls].chat_identifier;

					if self.messages_view.delete_current_text(chat) {
						self.chats_view.reload_chats();
					}
				}
			},
			":dc" => {
				if splits.len() > 0 {
					let chat = splits[0];
					let del_str = if let Ok(set) = SETTINGS.read() {
						set.delete_string(&chat, None)
					} else { "".to_owned() };

					if del_str.len() > 0 {
						match APICLIENT.get_url_string(&del_str) {
							Err(err) => if let Ok(mut state) = STATE.write() {
								state.hint_msg = format!("Failed to delete conversation: {}", err);
							},
							Ok(_) => {
								if let Ok(mut state) = STATE.write() {
									state.hint_msg = format!("deleted conversation :)");
								}

								self.chats_view.reload_chats();
							},
						}
					}

				} else if let Some(ls) = self.last_selected {
					let chat = self.chats_view.chats[ls].chat_identifier.as_str();

					if let Ok(mut state) = STATE.write() {
						state.hint_msg = format!("Please enter ':dc {}' if you'd like to delete this conversation", chat);
					}
				}
			},
			x => {
				if let Ok(mut state) = STATE.write() {
					state.hint_msg = format!("Command {} not recognized", x);
				}
			}
		};

		self.input_view.input = "".to_string();
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

	fn load_in_text(&mut self, text: Message) {
		// new_text returns the previous index of the conversation in which the new text was
		// sent. We can use it to determine how to shift self.last_selected
		match text.message_type {
			MessageType::Normal => {
				let past = self.chats_view.new_text(&text);
				let name = text.sender.as_ref().unwrap_or(
					text.chat_identifier.as_ref().unwrap()).to_owned();
				let text_content = text.text.to_owned();
				let show_notif = !text.is_from_me;

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

				if show_notif {
					Settings::show_notification(&name, &text_content);
				}
			},
			MessageType::Typing | MessageType::Idle => {
				if let Some(ref id) = text.chat_identifier {
					let name = text.sender.as_ref().unwrap_or(
						id).to_owned();

					if let Some(ls) = self.last_selected {
						if id == self.chats_view.chats[ls].chat_identifier.as_str() {
							if let MessageType::Idle = text.message_type {
								self.messages_view.set_idle();
							} else {
								self.messages_view.set_typing(text);
							}
						}
					}

					Settings::show_notification(&name, &format!("{} is typing...", name));
				}
			},
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

			let sent = APICLIENT.send_text(text, None, id, Some(in_files), None);

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

		let files_to_send = self.input_view.get_typed_attachments(orig);

		self.send_text(None, Some(files_to_send));
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

	pub fn send_tapback(&self, tap: &str) {
		let msgs = ["love", "like", "dislike", "laugh", "emphasize", "question"];
		let guid = &self.messages_view.messages[self.messages_view.selected_msg as usize].guid;

		if let Some(idx) = msgs.iter().position(|c| *c == tap) {
			if let Some(ls) = self.chats_view.last_selected {

				let chat = &self.chats_view.chats[ls].chat_identifier;

				let tap_url = SETTINGS.read().unwrap()
					.tapback_send_string(idx as i8, &guid, &chat, None);

				match APICLIENT.get_url_string(&tap_url) {
					Err(err) => if let Ok(mut state) = STATE.write() {
						state.hint_msg = format!("Could not send tapback: {}", err);
					},
					Ok(_) => if let Ok(mut state) = STATE.write() {
						state.hint_msg = "Sent tapback :)".to_owned();
					}
				}
			}
		} else if let Ok(mut state) = STATE.write() {
			state.hint_msg = format!("Did not recognize tapback option {}; possible options are: {}", tap, msgs.join(", "));
		}
	}
}

#[derive(PartialEq)]
enum DisplayBox {
	Chats,
	Messages,
	Help,
	Compose,
}
