use sdk::models::*;
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
	io::{Stdout, Error},
	mem::swap,
	cmp::{min, max},
	sync::mpsc
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
use sdk::commands::APICommand;
use tokio::sync::RwLock;

pub struct MainApp {
	// index of currently selected conversation in the chats array
	// in the chats view
	selected_chat: Option<usize>,
	// messages view, chats view, compose address, etc
	selected_box: DisplayBox,
	// when this is set true, everything quits.
	quit_app: bool,
	// Allows for redrawing in case there's some weird graphical corruption
	redraw_all: bool,
	// how far the help display is scrolled down
	help_scroll: u16,
	client: Arc<RwLock<sdk::APIClient>>,
	chats_view: ChatsView, // the different views
	msgs_view: MessagesView,
	input_view: InputView,
	address_view: InputView,
	compose_body_view: InputView,
}

impl MainApp {
	pub async fn new() -> anyhow::Result<MainApp> {
		let mut address_view = InputView::new();
		let mut compose_body_view = InputView::new();
		let mut config = sdk::SDKConfig::default();

		if let Ok(set) = SETTINGS.read() {
			address_view.custom_title = Some(set.to_title.to_owned());
			compose_body_view.custom_title = Some(set.compose_title.to_owned());

			if let Some(ref url) = set.remote_url {
				if !set.secure {
					eprintln!("\x1b[31;1mERROR:\x1b[0m \
						If you use a remote connection, it must be secure");
					return Err(sdk::error::SDKError::ConfigBlocked.into());
				}

				let id = match set.remote_id {
					Some(ref id) => id,
					None => {
						eprintln!("\x1b[31;1mERROR:\x1b[0m \
							If you input a remote address, \
							please input a remote id");
						return Err(sdk::error::SDKError::ConfigBlocked.into());
					}
				};

				let scheme = if url.starts_with("wss://") || url.starts_with("ws://")
					|| url.starts_with("http://") || url.starts_with("https://") {
					""
				} else {
					"wss://"
				};

				let conn_url = format!("{}{}/connect?id={}&key={}&sock_type=client",
					scheme, url, id, set.password);

				config = config.with_sock_url(conn_url)
					.with_rest(false);
			} else {
				let rest_url = format!("http{}://{}:{}/",
					if set.secure { "s" } else { "" },
					set.rest_host,
					set.rest_port,
				);

				let sock_url = format!("ws{}://{}:{}/",
					if set.secure { "s" } else { "" },
					set.rest_host,
					set.socket_port
				);

				config = config.with_rest_url(rest_url)
					.with_sock_url(sock_url)
					.with_rest(true);
			}

			config = config.with_password(set.password.to_owned())
				.with_timeout(set.timeout as usize)
				.with_secure(set.secure);
		}

		let (sender, receiver) = mpsc::sync_channel(0);

		MainApp::spawn_receiver(receiver);

		let client = sdk::APIClient::new(config, sender).await?;

		let client_arc = Arc::new(RwLock::new(client));

		let chats_view = match ChatsView::new(client_arc.clone()).await {
			Ok(chv) => chv,
			Err(err) => return Err(err)
		};

		let msgs_view = MessagesView::new(client_arc.clone());

		Ok(MainApp {
			selected_chat: None,
			selected_box: DisplayBox::Chats,
			quit_app: false,
			redraw_all: false,
			help_scroll: 0,
			input_view: InputView::new(),
			client: client_arc,
			chats_view,
			msgs_view,
			address_view,
			compose_body_view,
		})
	}

	pub fn spawn_receiver(
		receiver: mpsc::Receiver<sdk::socket::SocketResponse>
	) {
		tokio::spawn(async move {
			while let Ok(msg) = receiver.recv() {
				match msg.command {
					APICommand::Typing => {
						let typ = msg.typing_data()
							.expect("Cannot turn SocketResponse \
								into TypingNotification");

						let new_text = match typ.active {
							true => Message::typing(&typ.chat),
							_ => Message::idle(&typ.chat)
						};

						if let Ok(mut state) = STATE.write() {
							state.new_text = Some(new_text);
						}
					},
					APICommand::NewMessage => {
						let text = msg.new_message_data()
							.expect("Cannot turn SocketResponse \
								into NewMessageNotification");

						if let Ok(mut state) = STATE.write() {
							state.new_text = Some(text.message);
						}
					},
					APICommand::BatteryStatus => {
						let data = msg.battery_status_data()
							.expect("Cannot turn SocketResponse \
								into TypingNotification");

						let status = if data.charging {
							match data.percentage.round() as u8 {
								100 => BatteryStatus::Full,
								0 => BatteryStatus::Dead,
								x => BatteryStatus::Charging(x)
							}
						} else {
							BatteryStatus::Unplugged(data.percentage.round() as u8)
						};

						if let Ok(mut state) = STATE.write() {
							state.battery_status = status;
						}
					},
					_ => (),
				}
			}
		});
	}

	pub async fn main_loop(
		&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>
	) -> anyhow::Result<()> {

		// authenticate with the host right off the bat,
		// just so things don't time out later
		let mut api = self.client.write().await;

		if api.uses_rest {
			match api.authenticate().await {
				Err(err) => return Err(err),
				Ok(auth) => if !auth {
					return Err(sdk::error::SDKError::UnAuthenticated.into());
				}
			}
		}

		drop(api);

		// necessary to not print every character the user inputs
		if let Err(err) = crossterm::terminal::enable_raw_mode() {
			return Err(err.into());
		}

		// clears the screen
		print!("\x1b[2J\x1b[1;1H");

		// draw, get input, redraw with new state, get input, etc.
		while !self.quit_app {
			self.draw(term)?;

			let _ = self.get_input(&term).await;

			if self.redraw_all {
				// term.resize forces everything to redraw
				term.resize(term.size()?)?;
				self.redraw_all = false;
			}
		}

		// make the terminal echo everything input again
		match crossterm::terminal::disable_raw_mode() {
			Ok(_) => Ok(()),
			Err(err) => Err(err.into()),
		}
	}

	pub fn draw(
		&mut self, term: &mut Terminal<CrosstermBackend<Stdout>>
	) -> Result<(), Error> {
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
					// if we're showing the help box,
					// just draw the help box and nothing else
					let text: Vec<Spans> = HELP_MSG.iter()
						.map(|m| Spans::from(vec![Span::raw(*m)])).collect();
					let help_msg_widget = Paragraph::new(text)
						.block(
							Block::default()
								.title(help_title)
								.borders(Borders::ALL)
								.border_type(BorderType::Rounded)
								.border_style(Style::default()
									.fg(colorscheme.selected_box))
						)
						.wrap(Wrap { trim: true })
						.scroll((self.help_scroll, 0));

					f.render_widget(help_msg_widget, size);
				},
				_ => {
					// we have to get this string first so that we know how long it is
					// to make it left aligned
					let battery_msg = if let Ok(state) = STATE.read() {
						state.battery_string()
					} else {
						"0%, dead".to_owned()
					};

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

					let bottom_layout = Layout::default()
						.direction(Direction::Horizontal)
						.constraints(
							[
								Constraint::Min(1),
								Constraint::Length(battery_msg.len() as u16 + 1),
							].as_ref()
						).split(main_layout[2]);

					let chats_selected = self.selected_box == DisplayBox::Chats;
					let input_cursor = self.selected_box !=
						DisplayBox::ComposeAddress &&
						self.selected_box != DisplayBox::ComposeBody;

					// always draw the chats_view and input_view since it doesn't
					// mattter to them if the compose display is
					// currently selected
					self.chats_view.draw_view(
						f,
						content_layout[0],
						chats_selected
					);
					self.input_view.draw_view(
						f,
						main_layout[1],
						false,
						input_cursor
					);

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

						// draw the address view above the messages view,
						// and the body view under the messages view.
						// The messages view will just be a bit squished.
						let address_cursor =
							self.selected_box == DisplayBox::ComposeAddress;
						self.address_view.draw_view(f, message_layout[0],
							address_cursor, address_cursor);

						self.msgs_view.draw_view(
							f, message_layout[1], false
						);

						self.compose_body_view.draw_view(f, message_layout[2],
							!address_cursor, !address_cursor);

					} else {
						// if it's not, just draw the messages view like normal
						self.msgs_view.draw_view(
							f,
							content_layout[1],
							!chats_selected
						);
					}

					// create a span for the help box add the help string
					let hint_msg = if let Ok(state) = STATE.read() {
						state.hint_msg.to_owned()
					} else {
						"type :h to get help :)".to_owned()
					};

					let help_span = vec![
						Spans::from(vec![
							Span::styled(
								hint_msg,
								Style::default()
									.fg(colorscheme.hints_box)
							)
						])
					];

					let help_widget = Paragraph::new(help_span);
					f.render_widget(help_widget, bottom_layout[0]);

					// and show the battery percentage and status in the
					// bottom right corner
					let batt_span = vec![
						Spans::from(vec![
							Span::styled(
								battery_msg,
								Style::default()
									.fg(colorscheme.text_color)
							)
						])
					];

					let batt_widget = Paragraph::new(batt_span);
					f.render_widget(batt_widget, bottom_layout[1]);
				}
			}
		})?;

		Ok(())
	}

	async fn get_input(
		&mut self, term: &Terminal<CrosstermBackend<Stdout>>
	) -> crossterm::Result<()> {
		// we have to loop this so that if it gets a character/input
		// we don't want, we can just grab the next character/input instead.
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
						self.load_in_text(txt).await;
					}
					break;
				}

				let has_chats = if let Ok(state) = STATE.read() {
					state.new_chats.is_some()
				} else { false };

				if has_chats {
					let chats = if let Ok(mut state) = STATE.write() {
						let mut none_chats = None;
						swap(&mut none_chats, &mut state.new_chats);
						none_chats
					} else { None };

					if let Some(res) = chats {
						match res {
							Ok(chs) => self.loaded_in_chats(chs).await,
							Err(err) => hint!("failed to load in chats: {}", err),
						}
					}

					self.chats_view.await_state = AwaitState::Not;

					break;
				}

				let has_msgs = if let Ok(state) = STATE.read() {
					state.new_msgs.is_some()
				} else { false };

				if has_msgs {
					let msgs = if let Ok(mut state) = STATE.write() {
						let mut none_msgs = None;
						swap(&mut none_msgs, &mut state.new_msgs);
						none_msgs
					} else { None };

					if let Some(res) = msgs {
						match res {
							Ok(ms) => if ms.is_empty() {
								hint!("you have loaded in all the messages for this conversation");
							} else {
								self.loaded_in_messages(ms).await;
							},
							Err(err) => hint!("failed to load in messages: {}", err),
						}
					}

					self.msgs_view.await_state = AwaitState::Not;

					break;
				}

				// we check at every poll interval to see if the terminal has
				// resized. If it has, we break from getting input so that it can
				// redraw with the new size.
				let (new_width, new_height) = {
					match term.size() {
						Ok(size) => (size.width, size.height),
						Err(_) => (0, 0)
					}
				};

				if new_width != width || new_height != height {
					break;
				}

				// we also check if the websocket has disconnected. This is so
				// that we can show a message to let the user know it's
				// disconnected.
				let disc = if let Ok(state) = STATE.read() {
					state.websocket_state != WebSocketState::Connected
				} else { false };

				if disc { break };

			} else {
				let (code, modifiers) = match read()? {
					Event::Key(event) => (event.code, event.modifiers),
					_ => continue
				};

				match code {
					// each view treats these keycodes the same, so just
					// route it through the correct one.
					KeyCode::Backspace | KeyCode::Tab | KeyCode::Esc => {
						match self.selected_box {
							DisplayBox::ComposeBody =>
								self.compose_body_view.route_keycode(code),

							DisplayBox::ComposeAddress =>
								self.address_view.route_keycode(code),

							_ => {
								self.input_view.route_keycode(code);
								if code == KeyCode::Backspace &&
									self.input_view.input.is_empty() {
									self.send_typing_in_current(false).await;
								}
							},
						};
					},

					KeyCode::Enter => {
						match self.selected_box {
							DisplayBox::ComposeAddress => {
								// just moves the focus from the compose
								// address box to the body box; loads in
								// the messages to the msgs_view
								// just like in the real iMessage app.
								self.selected_box =
									DisplayBox::ComposeBody;

								let _ = self.msgs_view
									.load_in_conversation(
										&self.address_view.input
									).await;
							},
							DisplayBox::ComposeBody => {
								let chat = &self.address_view.input;

								// if you hit enter when you're already
								// in the compose
								// body, just send it.
								self.send_text(
									Some(chat.to_owned()),
									Some(self.compose_body_view.input
										.to_owned()),
									None
								).await;

								self.selected_box = DisplayBox::Messages;

								// this `awaiting_new_convo` thing is a kinda
								// hacky workaround. Basically, I set it to true
								// whenever someone sends a text from this
								// compose menu, and then whenever a new text
								// comes in through the websocket, it checks if
								// awaiting_new_convo is true.
								//
								// If it is true, it automatically loads in the
								// first conversation in the list, and doesn't
								// display the new text that came in
								// (since it will be loaded in with the
								// conversation).
								if let Ok(mut state) = STATE.write() {
									state.awaiting_new_convo = true;
								}
							},
							_ => if !self.input_view.input.is_empty() {
								self.handle_full_input().await;
							}
						}
					}
					// left and right move the cursor if there's input in the
					// box, else they just switch which box is selected
					KeyCode::Left | KeyCode::Right => {
						let right = code == KeyCode::Right;

						// just scroll the cursor in the input view by one.
						// Technically supports scrolling more than one
						// but that's not possible since the user can't specify
						// how much to scroll
						match self.selected_box {
							DisplayBox::ComposeAddress =>
								self.address_view.scroll(right, 1),
							DisplayBox::ComposeBody =>
								self.compose_body_view.scroll(right, 1),
							_ => if !self.input_view.input.is_empty() {
								self.input_view.scroll(
									code == KeyCode::Right, 1);
							} else {
								// if there's no input,
								// just switch the selected box
								self.switch_selected_box();
							},
						}
					},
					KeyCode::Up | KeyCode::Down =>
						if self.selected_box != DisplayBox::ComposeBody
						&& self.selected_box != DisplayBox::ComposeAddress {

						// tab up/down to more recent/less
						// recent executed command
						self.input_view.change_command(code == KeyCode::Up);
					},
					// ctrl+c gets hijacked by crossterm, so I wanted to manually
					// add in a way for people to invoke it to exit if that's
					// what they're used to.
					KeyCode::Char(c) => {
						if modifiers == KeyModifiers::CONTROL && c == 'c' {
							// however, we're gonna use ctrl+c to get out of the
							// compose view if you don't want to go through with
							// the new message
							match self.selected_box {
								DisplayBox::ComposeAddress |
								DisplayBox::ComposeBody =>
									self.selected_box = DisplayBox::Messages,
								_ => self.quit_app = true,
							}
						} else if c.is_digit(10) &&
							self.input_view.input.is_empty() &&
							self.selected_box != DisplayBox::ComposeBody &&
							self.selected_box != DisplayBox::ComposeAddress {

							// test for digits to allow for vim-like scrolling,
							// multiple lines at once.
							distance = format!("{}{}", distance, c);
							continue;

						} else {
							let dist: u16 = match distance.len() {
								0 => 1,
								_ => distance.parse().unwrap_or(1)
							};

							// compose boxes just get the input input,
							// they don't handle it specially.
							match self.selected_box {
								DisplayBox::ComposeAddress =>
									self.address_view.append_char(c),
								DisplayBox::ComposeBody =>
									self.compose_body_view.append_char(c),
								_ => self.handle_input_char(c, dist).await,
							}
						}
					}
					_ => continue,
				};
				break
			}
		}

		Ok(())
	}

	async fn handle_input_char(&mut self, ch: char, distance: u16) {
		// handle single character that is not a control key
		// this is only executed if the selected view is not the
		// compose address view and not the compose body view
		if !self.input_view.input.is_empty() || ch == ':' {
			self.input_view.append_char(ch);

			let graphemes = self.input_view.input
				.graphemes(true)
				.collect::<Vec<&str>>();

			// set the outgoing websocket message in the state if we're
			// writing a message
			if graphemes.len() > 3 {
				let three_chars = graphemes[..3]
					.join("")
					.to_lowercase();

				if three_chars == ":s " {
					self.send_typing_in_current(true).await;
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
				'k' | 'j' => self.scroll(ch == 'k', distance).await,
				// will add more later maybe
				_ => return,
			}
		}
	}

	async fn send_typing_in_current(&self, active: bool) {
		let res = if let Ok(state) = STATE.read() {
			if let Some(ref chat) = state.current_chat {
				let mut api = self.client.write().await;
				api.send_typing(&chat, active).await
			} else {
				Ok(())
			}
		} else {
			Ok(())
		};

		if res.is_err() {
			hint!("unable to send typing notifications to host; \
				you may want to check your connection");
		}
	}

	async fn handle_full_input(&mut self) {
		// add the command that it's handling to the most recent commands
		// so you can tab up to it
		self.input_view.last_commands
			.insert(
				0, self.input_view.input.to_owned()
			);

		// cmd is the first bit before a space, e.g. the ':s' in ':s hey friend'
		let mut splits = self.input_view.input.split(' ').collect::<Vec<&str>>();
		let cmd = splits.drain(0..1).as_slice()[0];

		match cmd.to_lowercase().as_str() {
			// quit the app
			":q" => self.quit_app = true,
			// select a chat
			":c" => if !splits.is_empty() && !splits[0].is_empty() {
				let index = splits[0].parse::<usize>();
				match index {
					Ok(idx) => self.load_in_conversation(idx).await,
					Err(_) => hint!("Cannot convert '{}' to an int", splits[0]),
				}
			} else {
				hint!("Please insert an index");
			},
			// show help
			":h" => self.selected_box = DisplayBox::Help,
			// send a text
			":s" => {
				let cmd = splits.join(" ");
				self.send_text(None, Some(cmd), None).await;
			},
			// reload the chats and redraw anything in case of
			// graphical inconsistencies
			":r" => {
				self.redraw_all = true;
				self.chats_view.reload_chats().await;
			},
			// modify settings (b for bind)
			":b" => {
				let ops = splits.iter()
					.map(|o| o.to_string())
					.collect::<Vec<String>>();
				self.bind_var(ops);
			},
			// open an attachment by index
			":a" => if !splits.is_empty() {
				let index = splits[0].parse::<usize>();
				match index {
					Ok(idx) => self.msgs_view.open_attachment(idx),
					Err(_) => hint!("Cannot convert {} to an int", splits[0]),
				}
			} else {
				hint!("Please inset an index");
			},
			// send files
			":f" => self.send_attachments(splits).await,
			// send a tapback
			":t" => {
				let tapback = splits.join("");
				self.send_tapback(&tapback).await;
			},
			// start a new composition
			":n" => {
				self.selected_box = DisplayBox::ComposeAddress;
				let _ = self.msgs_view.load_in_conversation("").await;

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
					if self.msgs_view.delete_current_text().await {
						let chat = &self.chats_view.chats[ls]
							.chat_identifier
							.to_owned();

						let load = self.msgs_view.load_in_conversation(&chat);
						let reload = self.chats_view.reload_chats();

						tokio::join!(load, reload);
					}
				}
			},
			// delete a conversation
			":dc" => if !splits.is_empty() {
				// this is if they specified a conversation to delete
				let chat = splits[0];

				let mut api = self.client.write().await;

				let success = match api.delete_chat(&chat).await {
					Err(err) => {
						hint!("Failed to delete conversation : {}", err);
						false
					},
					Ok(_) => {
						hint!("deleted conversation :)");

						// reload chats so that it doesn't show up anymore
						//self.chats_view.reload_chats().await;
						true
					},
				};

				drop(api);

				if success {
					self.chats_view.reload_chats().await;

					if let Some(ls) = self.selected_chat {
						let sel_chat = &self.chats_view.chats[ls]
							.chat_identifier;

						if sel_chat.as_str() == chat {
							self.msgs_view.load_in_conversation("").await;
						}
					}

					self.selected_chat = None;
				}
			} else if let Some(ls) = self.selected_chat {
				// if they didn't specify a conversation,
				// let them know how to delete this conversation
				let chat = &self.chats_view.chats[ls].chat_identifier;

				hint!("Please enter ':dc {}' if you'd like \
					to delete this conversation", chat);
			},
			// copy the text of the currently selected message
			// to the system clipboard
			":y" => self.msgs_view.copy_current_to_clipboard(),
			// default
			x => {
				hint!("Command {} not recognized", x);
			}
		};

		// to reset the input view to no input
		self.input_view.handle_escape();
	}

	fn switch_selected_box(&mut self) {
		// switches only between chats and messages
		if let DisplayBox::Chats = self.selected_box {
			self.selected_box = DisplayBox::Messages;
		} else if let DisplayBox::Messages = self.selected_box {
			self.selected_box = DisplayBox::Chats;
		}
	}

	async fn scroll(&mut self, up: bool, distance: u16) {
		// scrolls, depending on what the selected box is
		match self.selected_box {
			DisplayBox::Chats => self.chats_view.scroll(up, distance).await,
			DisplayBox::Messages => self.msgs_view.scroll(up, distance).await,
			DisplayBox::Help => {
				// these comparisons are to ensure it doesn't scroll too far
				if up {
					self.help_scroll = max(
						self.help_scroll as i32 - distance as i32,
						0
					) as u16;
				} else {
					self.help_scroll = min(
						HELP_MSG.len() as u16,
						self.help_scroll + distance
					);
				}
			},
			_ => {
				// this shouldn't ever be called
				hint!("sorry, I haven't implemented scrolling for this box yet :/");
			},
		}
	}

	async fn load_in_conversation(&mut self, idx: usize) {
		// ensure that it's in range
		if idx < self.chats_view.chats.len() {
			// first tell the chats view to load it in
			self.chats_view.load_in_conversation(idx);
			let id = self.chats_view.chats[idx].chat_identifier.to_owned();

			// then you can send it to the messages view
			let loaded = self.msgs_view.load_in_conversation(&id);

			if let Ok(mut state) = STATE.write() {
				state.current_chat = Some(id.to_owned());
			}

			self.selected_chat = Some(idx);

			loaded.await;

			hint!("loading in messages...");
		} else {
			hint!("{} is out of range for the chats", idx);
		}
	}

	async fn load_in_text(&mut self, text: Message) {
		match text.message_type {
			MessageType::Normal => {
				// new_text returns the previous index of the conversation
				// in which the new text was sent. We can use it to determine
				// how to shift self.selected_chat
				let past = self.chats_view.new_text(&text).await;

				let name = match &text.sender {
					Some(name) => name.to_owned(),
					None => {
						let chat_id = text.chat_identifier
							.as_ref()
							.unwrap()
							.to_owned();

						let mut api = self.client.write().await;

						if let Ok(nm) = api.get_name(&chat_id).await {
							nm
						} else {
							chat_id
						}
					}
				};

				let text_content = text.text.to_owned();

				// only show notification if it's not from me &&
				// they want notifications
				let show_notif = if let Ok(set) = SETTINGS.read() {
					set.notifications && !text.is_from_me
				} else {
					!text.is_from_me
				};

				// load_in will be true if I just composed and sent a
				// conversation. It's a kinda hacky workaround to prevent text
				// duplication when creating a new conversation from SMCurser
				let load_in = if let Ok(state) = STATE.read() {
					state.awaiting_new_convo
				} else { false };

				// If we did just compose and send a converation,
				// load in the top conversation.
				if load_in {
					self.load_in_conversation(0).await;

					if let Ok(mut state) = STATE.write() {
						state.awaiting_new_convo = false;
					}
				}

				// idx == the previous index of the conversation in which the
				// new text was sent
				// (since the conversation was automatically moved up to the top,
				// index 0) if that conversation did exist before now
				if let Some(idx) = past {
					if let Some(ls) = self.selected_chat {
						// so if that conversation did exist, and we previously
						// had a conversation selected...

						// only load in the new text to the msgs_view if it's
						// not a conversation we just created
						if idx == ls && !load_in {

							self.selected_chat = Some(0);
							self.msgs_view.new_text(text).await;

						} else if idx > ls {
							// increase the index of our currently selected chat
							// if the old index is above it, since it moving down
							// to index 0 will offset everything under it

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
					// need to grab name now 'cause `text` is moved
					// into msgs_view
					if text.message_type == MessageType::Typing {
						let name = text.sender.as_ref().unwrap_or(id);
						Utilities::show_notification(
							&name,
							&format!("{} is typing...", name)
						);
					}

					// if we have selected a chat...
					if let Some(ls) = self.selected_chat {
						// and it matches the identifier in the new message...
						if id == self.chats_view.chats[ls]
							.chat_identifier
							.as_str() {

							if text.message_type == MessageType::Idle {
								self.msgs_view.set_idle().await;
							} else {
								self.msgs_view.set_typing(text).await;
							}
						}
					}
				}
			},
		}
	}

	async fn loaded_in_chats(&mut self, chats: Vec<Conversation>) {
		match self.chats_view.await_state {
			AwaitState::More => {
				let mut mut_chats = chats;
				self.chats_view.chats.append(&mut mut_chats);
			},
			AwaitState::Replace => self.chats_view.chats = chats,
			_ => return
		}

		self.chats_view.last_height = 0;

		hint!("loaded in chats :)");
	}

	async fn loaded_in_messages(&mut self, msgs: Vec<Message>) {
		match self.msgs_view.await_state {
			AwaitState::More => {
				let mut mut_msgs = msgs;
				mut_msgs.reverse();

				self.msgs_view.selected_msg = self.msgs_view.messages.len() as u16;
				mut_msgs.append(&mut self.msgs_view.messages);

				self.msgs_view.messages = mut_msgs;
			},
			AwaitState::Replace => {
				self.msgs_view.messages = msgs;
				self.msgs_view.selected_msg = self.msgs_view.messages.len() as u16 - 1;
			},
			_ => return
		}

		self.msgs_view.last_height = 0;

		hint!("loaded in messages :)");
	}

	async fn send_text(
		&self,
		chat_id: Option<String>,
		text: Option<String>,
		files: Option<Vec<String>>
	) {
		// make chat_id an option so that it can be manually specified
		// for when you're making a new conversation, or omitted,
		// when you're sending a text in the current chat
		let chat_option = match chat_id {
			Some(ch) => Some(ch),
			None => self.selected_chat
				.map(|sel|
					self.chats_view.chats[sel].chat_identifier.to_owned()
				)
		};

		// tell the websocket that I'm not typing anymore
		self.send_typing_in_current(false).await;

		// only send it if you have a chat
		if let Some(id) = chat_option {
			let api_clone = self.client.clone();

			tokio::spawn(async move {
				let mut api = api_clone.write().await;

				let res = api.send_message(
					id, text, None, files, None
				).await;

				match res {
					Ok(_) => hint!("text sent :)"),
					Err(err) => hint!("text not sent: {}", err),
				}
			});

			hint!("sending text...");
		}
	}

	async fn send_attachments(&self, files: Vec<&str>) {
		let orig = files.join(" ");

		// this retuns a vector of strings, each string specifying the path
		// of a file to be sent
		let files_to_send = self.input_view.get_typed_attachments(orig);

		self.send_text(None, None, Some(files_to_send)).await;
	}

	fn bind_var(&mut self, ops: Vec<String>) {
		// set a variable in settings

		// you have to have the name of the variable to change,
		// and the value to change it to
		if ops.len() < 2 {
			hint!("Please enter at least a variable name and value");
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

	pub async fn send_tapback(&self, tap: &str) {
		let msgs = ["love", "like", "dislike", "laugh", "emphasize", "question"];
		let guid = &self.msgs_view.messages[
			self.msgs_view.selected_msg as usize].guid;

		// ensure that the tapback type that they specified is in the options
		if let Some(idx) = msgs.iter().position(|c| *c == tap) {
			// ensure that we've actually selected a conversation
			if self.chats_view.last_selected.is_some() {

				let mut api = self.client.write().await;

				match api.send_tapback(&guid, idx as u16, None).await {
					Err(err) => hint!("could not send tapback: {}", err),
					Ok(_) => hint!("sent tapback :)"),
				}
			}
		} else {
			hint!("Did not recognize tapback option {}; possible options are: {}",
				tap,
				msgs.join(", ")
			);
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

#[derive(PartialEq, Debug)]
pub enum AwaitState {
	More,
	Replace,
	Not
}
