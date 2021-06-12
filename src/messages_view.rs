use crate::{
	*,
	models::*,
	utilities::*,
};
use sdk::{
	models::*,
	api::*,
};
use tui::{
	layout::Rect,
	text::{Span, Spans},
	widgets::{Block, Borders, Paragraph, BorderType},
	style::{Style, Modifier},
	terminal::Frame,
};
use unicode_width::UnicodeWidthStr;
use std::{
	cmp::{min, max},
	io::Stdout,
};
use futures_locks::RwLock;
use clipboard::{ClipboardProvider, ClipboardContext};

pub struct MessagesView {
	pub selected_msg: u16,
	pub messages: Vec<Message>,
	pub line_list: Vec<MessageLine>,
	pub attachments: Vec<String>,
	pub last_width: u16,
	pub last_height: u16,
	pub y_bounds: (u16, u16), // .0 is top, .1 is bottom
	pub typing_idx: Option<usize>,
	pub client: Arc<RwLock<APIClient>>,
}

impl MessagesView {
	pub fn new(client: Arc<RwLock<APIClient>>) -> MessagesView {
		MessagesView {
			selected_msg: 0,
			messages: Vec::new(),
			line_list: Vec::new(),
			attachments: Vec::new(),
			last_width: 0,
			last_height: 0,
			y_bounds: (0, 0),
			typing_idx: None,
			client
		}
	}

	pub fn draw_view(
		&mut self,
		frame: &mut Frame<'_, CrosstermBackend<Stdout>>,
		rect: Rect,
		is_selected: bool
	) {
		// get the title and colorscheme
		let (title, colorscheme) = if let Ok(set) = SETTINGS.read() {
			(
				set.messages_title.to_owned(),
				colorscheme::Colorscheme::from(&set.colorscheme)
			)
		} else {
			("| messages: |".to_owned(), colorscheme::Colorscheme::from("forest"))
		};

		// recreate the vector that is used for drawing
		// if the terminal has been resized
		if rect.width != self.last_width || rect.height != self.last_height {
			self.rerender_list(rect);

			self.last_width = rect.width;
			self.last_height = rect.height;
		}

		// create the vector of spans that will be drawn to the terminal
		let item_list: Vec<Spans> = self.line_list.iter()
			.map(| l | {
				let style = match l.message_type {
					// set the style for the specific line based on its type
					MessageLineType::Blank |
					MessageLineType::TimeDisplay |
					MessageLineType::Text =>
						Style::default().fg(colorscheme.text_color),
					MessageLineType::Sender =>
						Style::default()
						.fg(colorscheme.text_color)
						.add_modifier(Modifier::ITALIC | Modifier::BOLD),
					MessageLineType::Underline =>
						Style::default().fg(
							if l.relative_index as u16 == self.selected_msg {
								colorscheme.selected_underline
							} else if l.from_me {
								colorscheme.my_underline
							} else {
								colorscheme.their_underline
							}
						),
					MessageLineType::Typing =>
						Style::default()
							.fg(colorscheme.text_color)
							.add_modifier(Modifier::ITALIC),
				};

				Spans::from(vec![Span::styled(l.text.as_str(), style)])
			})
			.collect();

		// this will serve as the border for the messages widget
		let messages_border = Block::default()
			.borders(Borders::ALL)
			.title(title)
			.border_type(BorderType::Rounded)
			.border_style(Style::default().fg(
				if is_selected {
					colorscheme.selected_box
				} else {
					colorscheme.unselected_box
				}
			));

		// create the widget and scroll it to the correct location
		let mut messages_widget = Paragraph::new(item_list)
			.block(messages_border);

		// scroll to the correct location
		if !self.messages.is_empty() && self.line_list.len() as u16 >= rect.height {
			messages_widget = messages_widget.scroll((self.y_bounds.0, 0));
		}
		frame.render_widget(messages_widget, rect);
	}

	pub fn rerender_list(&mut self, rect: Rect) {
		// this recreates the line list. It is in a separate function
		// from the draw_view so that it can only be called conditionally,
		// and you don't have to call it every single time the view is redrawn.
		let underline = if let Ok(set) = SETTINGS.read() {
			set.chat_underline.to_owned()
		} else {
			"▔".to_owned()
		};

		let msg_width = rect.width as usize - 2;
		let opts = textwrap::Options::new((msg_width as f64 * 0.6) as usize);

		let mut last_timestamp = 0;
		let mut last_sender = "".to_owned();
		let mut att_temp = Vec::new();

		// This gets a vector of spans for all the messages. It handles stuff like
		// inserting the time when necessary, adding the underlines, splitting the
		// texts into lines of correct length, etc.
		self.line_list = self.messages.iter()
			.enumerate()
			.fold(
				Vec::new(), |mut vec, (i, msg)| {
					// check, add time display if necessary
					if msg.date - last_timestamp >= 3600000000000 {
						let date_pad = Utilities::date_pad_string(
							msg.date, msg_width
						);
						let mut spans = vec![
							MessageLine::blank(i),
							MessageLine::new(
								date_pad,
								MessageLineType::TimeDisplay,
								i,
								msg.is_from_me
							),
							MessageLine::blank(i),
						];
						vec.append(&mut spans);
					}

					// Set the sender's name above their text
					// if it needs to be shown
					if let Some(send) = &msg.sender {
						if *send != last_sender ||
							msg.date - last_timestamp >= 3600000000000 {
							if msg.date - last_timestamp < 3600000000000 {
								vec.push(MessageLine::blank(i));
							}

							vec.push(
								MessageLine::new(
									send.to_owned(),
									MessageLineType::Sender,
									i,
									msg.is_from_me
								)
							);
						}

						last_sender = send.to_owned();
					}

					last_timestamp = msg.date;

					// split the text into its wrapped lines
					let text_lines: Vec<String> = textwrap::fill(
							msg.text.as_str(), opts.clone()
						)
						.split('\n')
						.map(|l| l.to_owned())
						.collect();

					// find the length of the longest line
					// (length calculated by utf-8 chars)
					let mut max = text_lines.iter()
						.fold(0, |m, l| {
							let len = UnicodeWidthStr::width(l.as_str());
							if len > m { len } else { m }
						});

					// do attachments
					for att in msg.attachments.iter() {
						let filename = att.path.split('/')
							.last()
							.unwrap_or_default();

						let att_str = format!("Attachment {}: {}",
							att_temp.len(), filename);

						if att_str.len() > max {
							max = att_str.len();
						}

						let att_line = if msg.is_from_me {
							let space = msg_width - att_str.len();
							format!("{}{}", " ".repeat(space), att_str)
						} else {
							att_str
						};

						vec.push(
							MessageLine::new(
								att_line,
								MessageLineType::Text,
								i,
								msg.is_from_me
							)
						);
						att_temp.push(att.path.to_owned());
					}

					let space = msg_width - max;

					if !msg.text.is_empty() {

						// add padding for my texts, put into spans
						let mut lines: Vec<MessageLine> = text_lines.into_iter()
							.map(|l| {
								let text = if msg.is_from_me {
									format!("{}{}", " ".repeat(space), l)
								} else { l };

								MessageLine::new(
									text,
									MessageLineType::Text,
									i,
									msg.is_from_me
								)
							})
							.collect();

						vec.append(&mut lines);
					}

					// add underline so it's pretty
					let underline = format!("{}{}",
						if msg.is_from_me {
							" ".repeat(space)
						} else {
							"".to_owned()
						},
						underline.as_str().repeat(max)
					);

					vec.push(
						MessageLine::new(
							underline,
							MessageLineType::Underline,
							i,
							msg.is_from_me
						)
					);

					vec
				}
			);

		// have to have a stored vector of attachments
		// so that you can access and open them at will
		self.attachments = att_temp;

		// y_bounds are what are shown
		if self.line_list.len() as u16 >= rect.height {
			self.y_bounds = (
				self.line_list.len() as u16 - rect.height,
				self.line_list.len() as u16 - 1
			);
		}
		self.scroll_none(false);
	}

	pub fn scroll_none(&mut self, up: bool) {
		let fut = self.scroll(up, 0);
		futures::executor::block_on(fut);
	}

	pub async fn scroll(&mut self, up: bool, distance: u16) {

		if self.messages.is_empty() { return; }

		// up == scrolling to older messages
		if !up {
			self.selected_msg = min(
				self.selected_msg + distance,
				self.messages.len() as u16 - 1
			);

			let scroll_opt = self.line_list.iter()
				.position(|m| m.relative_index as u16 > self.selected_msg);

			if let Some(mut scroll) = scroll_opt {
				scroll += 1; // why? don't ask me. Necessary to show underline tho

				if self.y_bounds.1 < scroll as u16 {
					self.y_bounds.0 += scroll as u16 - self.y_bounds.1;
					self.y_bounds.1 = scroll as u16;
				}
			} else { // only if you have the last message selected
				let scroll = self.line_list.len() as u16 + 1;
				self.y_bounds.0 += scroll - self.y_bounds.1;
				self.y_bounds.1 = scroll;
			}
		} else {
			// have to convert to signed to prevent overflow
			self.selected_msg = max(
				self.selected_msg as i32 - distance as i32,
				0
			) as u16;

			let scroll_opt = self.line_list.iter()
				.position(|m| m.relative_index as u16 == self.selected_msg);

			if let Some(scroll) = scroll_opt {
				if self.y_bounds.0 > scroll as u16 {
					self.y_bounds.1 -= self.y_bounds.0 - scroll as u16;
					self.y_bounds.0 = scroll as u16;
				}
			}

			if self.selected_msg == 0 {
				self.load_more_texts().await;
			}
		}
	}

	pub async fn load_in_conversation(&mut self, id: &str) -> anyhow::Result<bool> {
		// load in the messages for a certain conversation

		let mut api = self.client.write().await;

		self.messages = match api.get_messages(id, None, None, None).await {
			Ok(msgs) => msgs,
			Err(err) => return Err(err),
		};

		drop(api);

		// cause ya gotta. SMServer just sends them like that
		self.messages.reverse();

		self.last_width = 0; // to force it to redraw next time
		// set the selected message as the most recent one
		self.selected_msg = self.messages.len() as u16 - 1;

		Ok(true)
	}

	pub async fn load_more_texts(&mut self) {
		// load older texts; is triggered if you scroll up to a certain point
		let old_len = self.messages.len();

		let new_msgs_opt = if let Ok(mut state) = STATE.write() {
			if let Some(chat) = &state.current_chat {
				// get the texts with the current chat,
				// offset by how many we currently have loaded
				let mut api = self.client.write().await;

				match api.get_messages(
					chat, None, Some(self.messages.len() as u32), None
				).await {
					Ok(msgs) => Some(msgs),
					Err(err) => {
						state.hint_msg = format!("Failed to get messages: {}", err);
						None
					}
				}
			} else { None }
		} else { None };

		if let Some(mut new_msgs) = new_msgs_opt {
			if !new_msgs.is_empty() {
				// add it before the existing chats
				new_msgs.reverse();
				new_msgs.append(&mut self.messages);
				self.messages = new_msgs;

				// force it to redraw
				self.selected_msg = old_len as u16;
				self.last_height = 0;

				hint!("loaded in more messages");
			} else {
				// if the length is 0, then they've already loaded in
				// all the texts
				hint!("you have loaded in all the messages for this conversation");
			}
		}
	}

	pub async fn new_text(&mut self, msg: Message) {
		// this basically adds the text onto the list, then runs `rerender_list`
		// but it only rerenders the new text, if that makes sense.

		// so that it doesn't show typing anymore
		let show_typing_again = self.typing_idx.is_some() && msg.is_from_me;
		self.set_idle().await;

		// easy access so that we don't have to keep calling these
		let last = self.messages.last();
		let i = self.messages.len();

		let last_timestamp = match last {
			None => 0,
			Some(val) => val.date,
		};

		// show the time display
		if msg.date - last_timestamp >= 3600000000000 {
			let date_pad = Utilities::date_pad_string(
				msg.date,
				self.last_width as usize - 2
			);
			let mut spans = vec![
				MessageLine::blank(i),
				MessageLine::new(
					date_pad,
					MessageLineType::Text,
					i,
					msg.is_from_me
				),
				MessageLine::blank(i),
			];

			self.line_list.append(&mut spans);
		}

		// Show the sender if it exists
		if let Some(send) = &msg.sender {
			if last.is_none()
				|| send != last.unwrap().sender.as_ref().unwrap_or(&"".to_owned())
				|| msg.date - last_timestamp >= 3600000000000 {

				if msg.date - last_timestamp < 3600000000000 {
					self.line_list.push(MessageLine::blank(i));
				}

				self.line_list.push(
					MessageLine::new(
						send.to_owned(),
						MessageLineType::Sender,
						i,
						msg.is_from_me
					)
				);
			}
		}

		let opts = textwrap::Options::new(
			((self.last_width - 2) as f64 * 0.6) as usize);

		// split the text into its wrapped lines
		let text_lines: Vec<String> = textwrap::fill(msg.text.as_str(), opts)
			.split('\n')
			.map(|l| l.to_owned())
			.collect();

		// find the length of the longest line (length calculated by utf-8 chars)
		let mut max = text_lines.iter()
			.fold(0, |m, l| {
				let len = UnicodeWidthStr::width(l.as_str());
				if len > m { len } else { m }
			});

		// do attachments
		for att in msg.attachments.iter() {
			let att_str = format!("Attachment {}: {}",
				self.attachments.len(), att.mime_type);

			if att_str.len() > max {
				max = att_str.len();
			}

			let att_line = if msg.is_from_me {
				let space = self.last_width as usize - 2 - att_str.len();
				format!("{}{}", " ".repeat(space), att_str)
			} else {
				att_str
			};

			self.line_list.push(
				MessageLine::new(
					att_line,
					MessageLineType::Text,
					i,
					msg.is_from_me
				)
			);
			self.attachments.push(att.path.to_owned());
		}

		let space = self.last_width as usize - 2 - max;

		// add padding to my own texts so that they show correctly
		if !msg.text.is_empty() {
			let mut lines: Vec<MessageLine> = text_lines.into_iter()
				.map(|l| {
					let text = if msg.is_from_me {
						format!("{}{}", " ".repeat(space), l)
					} else { l };

					MessageLine::new(
						text,
						MessageLineType::Text,
						i,
						msg.is_from_me
					)
				})
				.collect();

			self.line_list.append(&mut lines);
		}

		let underline = if let Ok(set) = SETTINGS.read() {
			set.chat_underline.to_owned()
		} else {
			"▔".to_owned()
		};

		// add underline so it's pretty
		let underline = format!("{}{}",
			if msg.is_from_me { " ".repeat(space) } else { "".to_owned() },
			underline.as_str().repeat(max)
		);

		self.line_list.push(
			MessageLine::new(
				underline,
				MessageLineType::Underline,
				i,
				msg.is_from_me
			)
		);

		self.messages.push(msg);

		// if a new text from me just showed, and they're still typing,
		// we hide the typing display while my message is loaded in,
		// then show it again when it finishes loading in
		if show_typing_again {
			if let Ok(state) = STATE.read() {
				if let Some(ref chat) = state.current_chat {
					self.set_typing(Message::typing(chat)).await;
				}
			}
		}

		// select the new text and scroll to it
		self.selected_msg = self.messages.len() as u16 - 1;
		self.scroll_none(false);
	}

	pub async fn set_typing(&mut self, text: Message) {

		// show a new text at the bottom that says `Typing...`
		if self.typing_idx.is_none() {
			let model = MessageLine::new(
				"Typing...".to_owned(),
				MessageLineType::Typing,
				self.messages.len(),
				false
			);
			self.line_list.push(model);

			// set the typing index of for the chat so that we know
			// where the typing indicator is;
			// we use this index to remove it later
			self.typing_idx = Some(self.line_list.len() - 1);

			self.messages.push(text);
			self.selected_msg = self.messages.len() as u16 - 1;
			self.scroll_none(false);
		}
	}

	pub async fn set_idle(&mut self) {
		if let Some(id) = self.typing_idx {
			let line = &self.line_list[id];

			// remove the typing message
			self.messages.remove(line.relative_index);
			self.line_list.remove(id);

			// reset stuff to how it was before they started typing
			self.typing_idx = None;
			self.scroll_none(false);
		}
	}

	pub fn open_attachment(&self, idx: usize) {
		// Download the attachment to their downloads directory

		if idx >= self.attachments.len() {
			hint!("cannot get attachment {} (there are only {})",
				idx, self.attachments.len());
			return;
		}

		let mut down_dir = match dirs::download_dir() {
			Some(dir) => dir,
			None => {
				let mut dir =dirs::home_dir()
					.expect("Can't get download directory or home directory");
				dir.push("Downloads");
				dir
			}
		};

		let att_name = self.attachments[idx].to_owned();

		let location = match att_name.split('/').last() {
			Some(name) => {
				down_dir.push(name);
				match down_dir.as_os_str().to_str() {
					Some(file_loc) => {
						hint!("Downloading file to {}", file_loc);
						Some(file_loc.to_owned())
					}
					None => {
						hint!("Cannot download file. Please ensure the filename \
							({}) is valid unicode", name);
						None
					}
				}
			}
			None => {
				hint!("cannot get the name of the attachment to download :(");
				None
			}
		};

		if let Some(loc) = location {
			let api_clone = self.client.clone();

			tokio::spawn(async move {
				let mut api = api_clone.write().await;

				match api.get_attachment(&att_name).await {
					Ok(data) => {
						match std::fs::write(&loc, data) {
							Ok(_) => hint!("downloaded file to {} :)", loc),
							Err(err) => hint!("Could not write to file at {}: {}",
								loc, err),
						}
					},
					Err(err) => hint!("could not download file: {}", err),
				}
			});
		}
	}

	pub async fn delete_current_text(&mut self) -> bool {
		// deletes the currently selected text

		// first, check the index to make sure that it's in range
		// (I don't know how it wouldn't be but we gotta take precautions)
		if self.messages.len() as u16 <= self.selected_msg {
			hint!("Failed to delete message (not enough messages");
			return false;
		}

		// get the guid of the text
		let identifier = &self.messages[self.selected_msg as usize].guid;

		let mut api = self.client.write().await;

		match api.delete_text(identifier).await {
			Err(err) => {
				hint!("failed to delete text: {}", err);
				false
			},
			Ok(_) => {
				hint!("deleted text :)");
				true
			}
		}
	}

	pub fn copy_current_to_clipboard(&self) {
		let content = &self.messages[self.selected_msg as usize].text;
		let mut ctx: ClipboardContext = match ClipboardProvider::new() {
			Ok(ctx) => ctx,
			Err(err) => {
				hint!("could not copy to clipboard: {}", err);
				return;
			}
		};

		match ctx.set_contents(content.to_owned()) {
			Err(err) => hint!("could not copy to clipboard: {}", err),
			Ok(_) => hint!("copied text to clipboard :)"),
		}
	}
}
