use models::*;
use crate::{
	*,
	chats_view::*,
	messages_view::*,
	input_view::*,
	colorscheme::*,
	utilities::*,
	state::*,
};
use std::{
	vec::Vec,
	io::{Stdout, Error, ErrorKind},
	thread::{spawn, sleep},
	net::TcpStream,
	mem::swap,
	cmp::{min, max},
};
use core::time::Duration;
use tui::{
	layout::{Constraint, Direction, Layout},
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph, Wrap, BorderType},
	style::Style,
};
use crossterm::event::{read, Event, KeyCode, KeyModifiers, poll};
use unicode_segmentation::UnicodeSegmentation;

pub struct MainApp {
	selected_chat: Option<usize>, // index of currently selected conversation in the chats array within the chats view
	selected_box: DisplayBox, // messages view, chats view, compose address, etc
	quit_app: bool, // when this is set true, everything quits.
	redraw_all: bool, // this is set to true with ":r". Allows for redrawing in case there's some weird graphical corruption
	help_scroll: u16, // how far the help display is scrolled down
	chats_view: ChatsView, // the different views
	messages_view: MessagesView,
	input_view: InputView,
	address_view: InputView,
	compose_body_view: InputView,
}

impl MainApp {
	pub fn new() -> MainApp {
		let mut address_view = InputView::new();
		let mut compose_body_view = InputView::new();

		if let Ok(set) = SETTINGS.read() {
			address_view.custom_title = Some(set.to_title.to_owned());
			compose_body_view.custom_title = Some(set.compose_title.to_owned());
		}

		MainApp {
			selected_chat: None,
			selected_box: DisplayBox::Chats,
			quit_app: false,
			redraw_all: false,
			help_scroll: 0,
			chats_view: ChatsView::new(),
			messages_view: MessagesView::new(),
			input_view: InputView::new(),
			address_view: address_view,
			compose_body_view: compose_body_view,
		}
	}

	pub fn main_loop(&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Error> {

		// authenticate with the host right off the bat, just so things don't time out later
		let auth = APICLIENT.authenticate();

		let didnt_auth = match auth {
			Err(_) => true,
			Ok(b) => !b
		};

		if didnt_auth {

			// make nice error string to print out if authentication failed
			let err_str = if let Err(e) = auth {
				e.to_string()
			} else { "".to_owned() };

			let err = "\x1b[31;1mERROR:\x1b[0m";

			let address = if let Ok(set) = SETTINGS.read() {
				format!("http{}://{}:{}",
					if set.secure { "s" } else { "" },
					set.host,
					set.server_port
				)
			} else {
				"".to_owned()
			};

			// inform user that the authentication failedj
			eprintln!("{} Failed to authenticate with {} - {}", err, address, err_str);
			eprintln!("Trying fallback host...");
			if let Ok(mut set) = SETTINGS.write() {
				set.host = set.fallback_host.to_owned();
			}

			// try the authentication again, but now using the fallback host
			if !APICLIENT.check_auth() {
				eprintln!("{} Failed to authenticate with both hosts. Please check your configuration.", err);
				return Err(Error::new(ErrorKind::Other, "Failed to authenticate"));
			}
		}

		// set up the connection with the websocket
		self.setup_socket();

		// necessary to not print every character the user inputs
		let _ = crossterm::terminal::enable_raw_mode();

		// clears the screen
		print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

		// draw, get input, redraw with new state, get input, etc.
		while !self.quit_app {
			self.draw(term)?;

			let _ = self.get_input(&term);

			if self.redraw_all {
				// term.resize forces everything to redraw
				term.resize(term.size()?)?;
				self.redraw_all = false;
			}
		}

		// make the terminal echo everything input again
		let _ = crossterm::terminal::disable_raw_mode(); // i just be ignoring results tho

		Ok(())
	}

	fn setup_socket(&mut self) {
		// would do an if let Ok() but that would be ugly. this is easier and should never panic
		let set = SETTINGS.read().expect("Couldn't read settings");
		let host = set.host.as_str().to_owned();
		let port = set.socket_port;
		let sec = set.secure;
		drop(set);

		// need to run websocket in a new thread so that it can do things while main thread is
		// waiting for input
		spawn(move || {
			// need to make this a template so that it can be re-created when the websocket
			// disconnects
			let sock_res_template = | | {
				let config = Some(tungstenite::protocol::WebSocketConfig {
					max_send_queue: None,
					max_message_size: None,
					max_frame_size: None,
					accept_unmasked_frames: false
				});

				// need this custom connector so that it connects with SMServer's self-signed cert
				let connector = native_tls::TlsConnector::builder()
					.danger_accept_invalid_certs(true)
					.danger_accept_invalid_hostnames(true)
					.build()
					.expect("Couldn't build tls connector");

				// ngl I don't understand this part perfectly but it was online and it works
				let mut stream_try = TcpStream::connect(format!("{}:{}", host, port));
				let url = format!("ws{}://{}:{}", if sec { "s" } else { "" }, host, port);
				let parsed_url = url::Url::parse(&url)
					.expect(&format!("Failed to parse websocket URL: '{}'", url));

				let mut tls_stream: Option<native_tls::TlsStream<TcpStream>> = None;

				// gotta do a loop so that it keeps trying to re-connect if it fails initially
				while tls_stream.is_none() {
					match stream_try {
						Ok(stream) => {
							tls_stream = Some(connector.connect(&host, stream)
								.expect("Couldn't connect to valid TLS Stream"));
							break;
						},
						Err(_) => {
							if let Ok(mut state) = STATE.write() {
								state.hint_msg = "Websocket disconnected; retrying...".to_owned();
								state.websocket_state = WebSocketState::Connecting;
							}
							sleep(Duration::from_secs(2));
							stream_try = TcpStream::connect(format!("{}:{}", host, port));
						}
					}
				};

				tungstenite::client::client_with_config(
					parsed_url,
					tls_stream.expect("Valid TLS Stream became invalid"),
					config
				)
			};

			let mut sock_res = sock_res_template();

			// loop until app dies
			loop {
				match sock_res {
					Ok((mut socket, _)) => {
						if let Ok(mut state) = STATE.write() {
							state.websocket_state = WebSocketState::Connected;
						}

						loop {
							// read the incoming message; we don't write to it (yet)
							// so we don't need to poll
							let msg = socket.read_message();
							match msg {
								Ok(tungstenite::Message::Text(val)) => {
									// all messages are in the format of `prefix:content`
									let mut splits = val.splitn(2, ':');
									let prefix = splits.next().unwrap_or("");
									let content = splits.next().unwrap_or("");

									match prefix {
										"text" => {
											// "text" means that I received a new text. parse it
											// and put it in STATE as a new text
											let json: serde_json::Value = serde_json::from_str(&content).unwrap();
											let text_json: serde_json::Map<String, serde_json::Value> =
												json["text"].as_object().unwrap().to_owned();

											if let Ok(mut state) = STATE.write() {
												state.new_text = Some(Message::from_json(&text_json));
											}
										},
										"typing" => {
											// "typing" means that someone is typing in a
											// conversation
											let typing_text = Message::typing(content);

											if let Ok(mut state) = STATE.write() {
												state.new_text = Some(typing_text);
											}
										},
										"idle" => {
											// "idle" means that someone stopped typing in a
											// conversation
											let idle_text = Message::idle(content);

											if let Ok(mut state) = STATE.write() {
												state.new_text = Some(idle_text);
											}
										},
										&_ => (),
									}
								},
								Err(_) => {
									// if err, it disconnected. Sleep, re-try the connection,
									// and continue.
									sleep(Duration::from_secs(2));
									sock_res = sock_res_template();
									break;
								},
								_ => (),
							}
						}
					},
					Err(ref x) => {
						// if it initally connected, but is now somehow disconnected.
						if let Ok(mut state) = STATE.write() {
							state.hint_msg = format!("Error: Failed to connect to websocket: {} New messages will not show.", x);
							state.websocket_state = WebSocketState::Disconnected;
						}
						// sleep and try again.
						sleep(Duration::from_secs(2));
						sock_res = sock_res_template();
						continue;
					}
				};
			}

		});
	}

	pub fn draw(&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Error> {
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
						.block(
							Block::default()
								.title(help_title)
								.borders(Borders::ALL)
								.border_type(BorderType::Rounded)
								.border_style(Style::default().fg(colorscheme.selected_box))
						)
						.wrap(Wrap { trim: true })
						.scroll((self.help_scroll, 0));

					f.render_widget(help_msg_widget, size);
				},
				_ => {
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
						).split(main_layout[0]);

					let chats_selected = self.selected_box == DisplayBox::Chats;
					let input_cursor = self.selected_box != DisplayBox::ComposeAddress &&
						self.selected_box != DisplayBox::ComposeBody;

					// always draw the chats_view and input_view since it doesn't mattter to them
					// if the compose display is currently selected
					self.chats_view.draw_view(f, content_layout[0], chats_selected);
					self.input_view.draw_view(f, main_layout[1], false, input_cursor);

					// if the compose display is up...
					if DisplayBox::ComposeAddress == self.selected_box
						|| DisplayBox::ComposeBody == self.selected_box {

						// set up a new layout
						let message_layout = Layout::default()
							.direction(Direction::Vertical)
							.constraints(
								[
									Constraint::Length(3),
									Constraint::Min(3),
									Constraint::Length(3)
								].as_ref()
							).split(content_layout[1]);

						// draw the address view above the messages view, and the body view under
						// the messages view. The messages view will just be a bit squished.
						let address_cursor = self.selected_box == DisplayBox::ComposeAddress;
						self.address_view.draw_view(f, message_layout[0],
							address_cursor, address_cursor);

						self.messages_view.draw_view(f, message_layout[1], false);
						self.compose_body_view.draw_view(f, message_layout[2],
							!address_cursor, !address_cursor);

					} else {
						// if it's not, just draw the messages view like normal
						self.messages_view.draw_view(f, content_layout[1], !chats_selected);
					}

					// create a span for the help box add the help string
					let hint_msg = if let Ok(state) = STATE.read() {
						state.hint_msg.to_owned()
					} else {
						"type :h to get help :)".to_owned()
					};

					let help_span = vec![Spans::from(vec![Span::styled(hint_msg, Style::default().fg(colorscheme.hints_box))])];
					let help_widget = Paragraph::new(help_span);
					f.render_widget(help_widget, main_layout[2]);
				}
			}
		})?;

		Ok(())
	}

	fn get_input(&mut self, term: &Terminal<CrosstermBackend<Stdout>>) -> crossterm::Result<()> {
		// we have to loop this so that if it gets a character/input we don't want,
		// we can just grab the next character/input instead.
		let mut distance = "".to_string();
		let (width, height) = {
			match term.size() {
				Ok(size) => (size.width, size.height),
				Err(_) => (0, 0),
			}
		};

		loop {
			if !poll(Duration::from_millis(20)).unwrap() {
				// first check if there's actually an unread text
				let has_unread = if let Ok(state) = STATE.read() {
					state.new_text.is_some()
				} else { false };

				if has_unread {
					let none_text = if let Ok(mut state) = STATE.write() {
						// swap the new text out for `None`
						let mut none_text: Option<Message> = None;
						swap(&mut none_text, &mut state.new_text);
						none_text
					} else { None };

					// send the new text to the load in function
					if let Some(txt) = none_text {
						self.load_in_text(txt);
					}
					break;
				}

				// we check at every poll interval to see if the terminal has resized. If it has,
				// we break from getting input so that it can redraw with the new size.
				let (new_width, new_height) = {
					match term.size() {
						Ok(size) => (size.width, size.height),
						Err(_) => (0, 0)
					}
				};

				if new_width != width || new_height != height {
					break;
				}

				// we also check if the websocket has disconnected. This is so that we can show a
				// message to let the user know it's disconnected
				let disc = if let Ok(state) = STATE.read() {
					state.websocket_state != WebSocketState::Connected
				} else { false };

				if disc { break };

			} else {
				match read()? {
					Event::Key(event) => {
						match event.code {
							// each view treats these keycodes the same, so just route it through
							// the correct one.
							KeyCode::Backspace | KeyCode::Tab | KeyCode::Esc => {
								match self.selected_box {
									DisplayBox::ComposeBody => self.compose_body_view.route_keycode(event.code),
									DisplayBox::ComposeAddress => self.address_view.route_keycode(event.code),
									_ => {
										self.input_view.route_keycode(event.code);
										if event.code == KeyCode::Backspace &&
											self.input_view.input.len() == 0 {

											if let Ok(mut state) = STATE.write() {
												state.set_idle_in_current();
											}
										}
									},
								};
							},

							KeyCode::Enter => {
								match self.selected_box {
									DisplayBox::ComposeAddress => {
										// just moves the focus from the compose address box to the
										// body box; loads in the messages to the messages_view
										// just like in the real iMessage app.
										self.selected_box = DisplayBox::ComposeBody;
										self.messages_view.load_in_conversation(&self.address_view.input);
									},
									DisplayBox::ComposeBody => {
										let chat = &self.address_view.input;

										// if you hit enter when you're already in the compose
										// body, just send it.
										self.send_text(Some(chat.to_owned()),
											Some(self.compose_body_view.input.to_owned()), None);

										//self.messages_view.load_in_conversation(chat);
										self.selected_box = DisplayBox::Messages;

										//self.selected_chat = Some(0);

										// this `awaiting_new_convo` thing is a kinda hacky
										// workaround. Basically, I set it to true whenever someone
										// sends a text from this compose menu, and then whenever a
										// new text comes in through the websocket, it checks if
										// awaiting_new_convo is true.
										//
										// If it is true, it automatically loads in the first
										// conversation in the list, and doesn't display the new
										// text that came in (since it will be loaded in with the
										// conversation).
										if let Ok(mut state) = STATE.write() {
											state.awaiting_new_convo = true;
										}
									},
									_ => if self.input_view.input.len() > 0 {
										self.handle_full_input();
									}
								}
							}
							// left and right move the cursor if there's input in the box, else they
							// just switch which box is selected
							KeyCode::Left | KeyCode::Right => {
								let right = event.code == KeyCode::Right;

								// just scroll the cursor in the input view by one. Technically
								// supports scrolling more than one but that's not possible since
								// you can't specify how much to scroll
								match self.selected_box {
									DisplayBox::ComposeAddress => self.address_view.scroll(right, 1),
									DisplayBox::ComposeBody => self.compose_body_view.scroll(right, 1),
									_ => if self.input_view.input.len() > 0 {
										self.input_view.scroll(event.code == KeyCode::Right, 1);
									} else {
										// if there's no input, just switch the selected box
										self.switch_selected_box();
									},
								}
							},
							KeyCode::Up | KeyCode::Down => if self.selected_box != DisplayBox::ComposeBody
								&& self.selected_box != DisplayBox::ComposeAddress {

								// tab up/down to more recent/less recent executed command
								self.input_view.change_command(event.code == KeyCode::Up);
							},
							// ctrl+c gets hijacked by crossterm, so I wanted to manually add in a way
							// for people to invoke it to exit if that's what they're used to.
							KeyCode::Char(c) => {
								if event.modifiers == KeyModifiers::CONTROL && c == 'c' {
									// however, we're gonna use ctrl+c to get out of the compose
									// view if you don't want to go through with the new message
									match self.selected_box {
										DisplayBox::ComposeAddress | DisplayBox::ComposeBody =>
											self.selected_box = DisplayBox::Messages,
										_ => self.quit_app = true,
									}
								} else if c.is_digit(10) && self.input_view.input.len() == 0 &&
									self.selected_box != DisplayBox::ComposeBody &&
									self.selected_box != DisplayBox::ComposeAddress {

									// test for digits to allow for vim-like scrolling, multiple lines
									// at once.
									distance = format!("{}{}", distance, c);
									continue;

								} else {
									let dist: u16 = match distance.len() {
										0 => 1,
										_ => distance.parse().unwrap_or(1)
									};

									// compose boxes just get the input input, they don't handle it
									// specially.
									match self.selected_box {
										DisplayBox::ComposeAddress => self.address_view.append_char(c),
										DisplayBox::ComposeBody => self.compose_body_view.append_char(c),
										_ => self.handle_input_char(c, dist),
									}
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
		// handle single character that is not a control key
		// this is only executed if the selected view is not the compose address view
		// and not the compose body view
		if self.input_view.input.len() > 0 || ch == ':' {
			self.input_view.append_char(ch);

			let graphemes = self.input_view.input
				.graphemes(true)
				.collect::<Vec<&str>>();

			// set the outgoing websocket message in the state if we're writing a message
			if graphemes.len() > 3 {
				let three_chars = graphemes[..3]
					.join("")
					.to_lowercase();

				if three_chars == ":s " {

					if let Ok(mut state) = STATE.write() {
						state.set_typing_in_current();
					}
				}
			}
		} else {
			match ch {
				'h' | 'l' => self.switch_selected_box(),
				// quit out of help display if it is showing
				'q' | 'Q' => if let DisplayBox::Help = self.selected_box {
					self.selected_box = DisplayBox::Chats;
				},
				// scroll up or down in the selected box
				'k' | 'j' => self.scroll(ch == 'k', distance),
				// will add more later maybe
				_ => return,
			}
		}
	}

	fn handle_full_input(&mut self) {
		// add the command that it's handling to the most recent commands so you can tab up to it
		self.input_view.last_commands.insert(0, self.input_view.input.to_owned());

		// cmd is the first bit before a space, e.g. the ':s' in ':s hey friend'
		let mut splits = self.input_view.input.split(' ').collect::<Vec<&str>>();
		let cmd = splits.drain(0..1).as_slice()[0];

		match cmd.to_lowercase().as_str() {
			// quit the app
			":q" => self.quit_app = true,
			// select a chat
			":c" => if splits.len() > 0 {
				let index = splits[0].parse::<usize>();
				match index {
					Ok(idx) => self.load_in_conversation(idx),
					Err(_) => if let Ok(mut state) = STATE.write() {
						state.hint_msg = format!("Cannot convert {} to an int", splits[0]);
					},
				}
			} else if let Ok(mut state) = STATE.write() {
				state.hint_msg = "Please insert an index".to_string();
			},
			// show help
			":h" => self.selected_box = DisplayBox::Help,
			// send a text
			":s" => {
				let cmd = splits.join(" ");
				self.send_text(None, Some(cmd), None);
			},
			// reload the chats and redraw anything in case of graphical inconsistencies
			":r" => {
				self.redraw_all = true;
				self.chats_view.reload_chats();
			},
			// modify settings (b for bind)
			":b" => {
				let ops = splits.iter().map(|o| o.to_string()).collect::<Vec<String>>();
				self.bind_var(ops);
			},
			// open an attachment by index
			":a" => if splits.len() > 0 {
				let index = splits[0].parse::<usize>();
				match index {
					Ok(idx) => self.messages_view.open_attachment(idx),
					Err(_) => if let Ok(mut state) = STATE.write() {
						state.hint_msg = format!("Cannot convert {} to an int", splits[0]);
					}
				}
			} else if let Ok(mut state) = STATE.write() {
				state.hint_msg = "Please insert an index".to_string();
			},
			// send files
			":f" => self.send_attachments(splits),
			// send a tapback
			":t" => {
				let tapback = splits.join("");
				self.send_tapback(&tapback);
			},
			// start a new composition
			":n" => {
				self.selected_box = DisplayBox::ComposeAddress;
				self.messages_view.load_in_conversation("");

				self.compose_body_view.input = "".to_owned();
				self.address_view.input = "".to_owned();

				if let Some(ls) = self.selected_chat {
					self.chats_view.chats[ls].is_selected = false;
					self.selected_chat = None;
				}

				self.chats_view.last_height = 0;
			}
			// delete a text
			":dt" => {
				if let Some(ls) = self.selected_chat {
					let chat = &self.chats_view.chats[ls].chat_identifier;

					if self.messages_view.delete_current_text() {
						self.messages_view.load_in_conversation(chat);
						self.chats_view.reload_chats();
					}
				}
			},
			// delete a conversation
			":dc" => if splits.len() > 0 {
				// this is if they specified a conversation to delete
				let chat = splits[0];

				// get the url to talk to to delete this conversation
				let del_str = if let Ok(set) = SETTINGS.read() {
					set.delete_chat_string(&chat)
				} else { "".to_owned() };

				if del_str.len() > 0 {
					// send the request
					match APICLIENT.get_url_string(&del_str) {
						Err(err) => if let Ok(mut state) = STATE.write() {
							state.hint_msg = format!("Failed to delete conversation: {}", err);
						},
						Ok(_) => {
							if let Ok(mut state) = STATE.write() {
								state.hint_msg = format!("deleted conversation :)");
							}

							// reload chats so that it doesn't show up anymore
							self.chats_view.reload_chats();
						},
					}
				}

			} else if let Some(ls) = self.selected_chat {
				// if they didn't specify a conversation, let them know how to delete this conversation
				let chat = &self.chats_view.chats[ls].chat_identifier;

				if let Ok(mut state) = STATE.write() {
					state.hint_msg = format!("Please enter ':dc {}' if you'd like to delete this conversation", chat);
				}
			},
			// default
			x => {
				if let Ok(mut state) = STATE.write() {
					state.hint_msg = format!("Command {} not recognized", x);
				}
			}
		};

		self.input_view.input = "".to_string();
	}

	fn switch_selected_box(&mut self) {
		// switches only between chats and messages
		if let DisplayBox::Chats = self.selected_box {
			self.selected_box = DisplayBox::Messages;
		} else if let DisplayBox::Messages = self.selected_box {
			self.selected_box = DisplayBox::Chats;
		}
	}

	fn scroll(&mut self, up: bool, distance: u16) {
		// scrolls, depending on what the selected box is
		match self.selected_box {
			DisplayBox::Chats => self.chats_view.scroll(up, distance),
			DisplayBox::Messages => self.messages_view.scroll(up, distance),
			DisplayBox::Help => {
				// these comparisons are to ensure it doesn't scroll too far
				if up {
					self.help_scroll = max(self.help_scroll as i32 - distance as i32, 0) as u16;
				} else {
					self.help_scroll = min(HELP_MSG.len() as u16, self.help_scroll + distance);
				}
			},
			_ => if let Ok(mut state) = STATE.write() {
				// this shouldn't ever be called
				state.hint_msg = "Sorry, I haven't implemented scrolling for this box yet :/".to_string();
			},
		}
	}

	fn load_in_conversation(&mut self, idx: usize) {
		// ensure that it's in range
		if idx < self.chats_view.chats.len() {
			// first tell the chats view to load it in
			self.chats_view.load_in_conversation(idx);
			let id = self.chats_view.chats[idx].chat_identifier.to_owned();

			// then you can send it to the messages view
			self.messages_view.load_in_conversation(&id);

			self.selected_chat = Some(idx);

			if let Ok(mut state) = STATE.write() {
				state.current_chat = Some(id);
				state.hint_msg = "loaded in chat :)".to_string();
			}
		} else if let Ok(mut state) = STATE.write() {
			state.hint_msg = format!("{} is out of range for the chats", idx);
		}
	}

	fn load_in_text(&mut self, text: Message) {
		match text.message_type {
			MessageType::Normal => {
				// new_text returns the previous index of the conversation in which the new text was
				// sent. We can use it to determine how to shift self.selected_chat
				let past = self.chats_view.new_text(&text);
				let name = text.sender.as_ref().unwrap_or(
						&APICLIENT.get_name(text.chat_identifier.as_ref().unwrap())
					)
					.to_owned();

				let text_content = text.text.to_owned();

				// only show notification if it's not from me && they want notifications
				let show_notif = if let Ok(set) = SETTINGS.read() {
					set.notifications && !text.is_from_me
				} else {
					!text.is_from_me
				};

				// load_in will be true if I just composed and sent a conversation. It's a kinda
				// hacky workaround to prevent text duplication when creating a new conversation
				// from SMCurser
				let load_in = if let Ok(state) = STATE.read() {
					state.awaiting_new_convo
				} else { false };

				// If we did just compose and send a converation, load in the top conversation.
				if load_in {
					self.load_in_conversation(0);

					if let Ok(mut state) = STATE.write() {
						state.awaiting_new_convo = false;
					}
				}

				// idx == the previous index of the conversation in which the new text was sent
				// (since the conversation was automatically moved up to the top, index 0) if that
				// conversation did exist before now
				if let Some(idx) = past {
					if let Some(ls) = self.selected_chat {
						// so if that conversation did exist, and we previously had a conversation
						// selected...

						// only load in the new text to the messages_view if it's not a
						// conversation we just created
						if idx == ls && !load_in {

							self.selected_chat = Some(0);
							self.messages_view.new_text(text);

						} else if idx > ls {
							// increase the index of our currently selected chat if the old index
							// is above it, since it moving down to index 0 will offset everything
							// under it

							self.selected_chat = Some(ls + 1);
						}
					}
				}

				if show_notif {
					Utilities::show_notification(&name, &text_content);
				}
			},
			MessageType::Typing | MessageType::Idle => {
				// the new message could also be a typing or idle message

				if let Some(ref id) = text.chat_identifier {
					// need to grab name now 'cause `text` is moved into messages_view
					if text.message_type == MessageType::Typing {
						let name = text.sender.as_ref().unwrap_or(id);
						Utilities::show_notification(&name, &format!("{} is typing...", name));
					}

					// if we have selected a chat...
					if let Some(ls) = self.selected_chat {
						// and it matches the identifier in the new message...
						if id == self.chats_view.chats[ls].chat_identifier.as_str() {
							if text.message_type == MessageType::Idle {
								self.messages_view.set_idle();
							} else {
								self.messages_view.set_typing(text);
							}
						}
					}
				}
			},
		}
	}

	fn send_text(&self, chat_id: Option<String>, text: Option<String>, files: Option<Vec<String>>) {
		// make chat_id an option so that it can be manually specified for when you're making a new
		// conversation, or omitted, when you're sending a text in the current chat
		let chat_option = match chat_id {
			Some(ch) => Some(ch),
			None => match self.selected_chat {
				Some(sel) => Some(self.chats_view.chats[sel]
					.chat_identifier.to_owned()),
				None => None,
			}
		};

		// tell the websocket that I'm not typing anymore
		if let Ok(mut state) = STATE.write() {
			state.set_idle_in_current();
		}

		// only send it if you have a chat
		if let Some(id) = chat_option {
			let sent = APICLIENT.send_text(text, None, id, files, None);

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

		// this retuns a vector of strings, each string specifying the path of a file to be sent
		let files_to_send = self.input_view.get_typed_attachments(orig);

		self.send_text(None, None, Some(files_to_send));
	}

	fn bind_var(&mut self, ops: Vec<String>) {
		// set a variable in settings

		// you have to have the name of the variable to change, and the value to change it to
		if ops.len() < 2 {
			if let Ok(mut state) = STATE.write() {
				state.hint_msg = "Please enter at least a variable name and value".to_string();
			}
			return;
		}

		let mut new_ops = ops;
		// val = all but the first element of `ops`
		let val = new_ops.split_off(1);
		// add back val, but joined with spaces. So that you could do a command like
		// `:b input_title this is a title`
		new_ops.push(val.join(" "));

		if let Ok(mut set) = SETTINGS.write() {
			set.parse_args(new_ops, true, false);
		}
	}

	pub fn send_tapback(&self, tap: &str) {
		let msgs = ["love", "like", "dislike", "laugh", "emphasize", "question"];
		let guid = &self.messages_view.messages[self.messages_view.selected_msg as usize].guid;

		// ensure that the tapback type that they specified is in the options
		if let Some(idx) = msgs.iter().position(|c| *c == tap) {
			// ensure that we've actually selected a conversation
			if !self.chats_view.last_selected.is_none() {

				// get the url and send it!
				let tap_url = SETTINGS.read().unwrap()
					.tapback_send_string(idx as i8, &guid, None);

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
	ComposeAddress,
	ComposeBody,
}
