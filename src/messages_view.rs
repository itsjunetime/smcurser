use crate::*;
use crate::models::*;
use tui::{
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, BorderType},
	style::{Style, Modifier},
	terminal::Frame,
};
use unicode_segmentation::UnicodeSegmentation;

pub struct MessagesView {
	pub selected_msg: u16,
	pub messages: Vec<Message>,
	pub line_list: Vec<MessageLine>,
	pub attachments: Vec<String>,
	pub last_width: u16,
	pub last_height: u16,
	pub y_bounds: (u16, u16), // .0 is top, .1 is bottom
	pub typing_idx: Option<usize>,
}

impl MessagesView {
	pub fn new() -> MessagesView {
		MessagesView {
			selected_msg: 0,
			messages: Vec::new(),
			line_list: Vec::new(),
			attachments: Vec::new(),
			last_width: 0,
			last_height: 0,
			y_bounds: (0, 0),
			typing_idx: None,
		}
	}

	pub fn draw_view(&mut self, frame: &mut Frame<CrosstermBackend<io::Stdout>>, rect: Rect, is_selected: bool) {
		if let Ok(set) = SETTINGS.read() {
			let colorscheme = colorscheme::Colorscheme::from(&set.colorscheme);

			if rect.width != self.last_width || rect.height != self.last_height {
				self.rerender_list(rect);

				self.last_width = rect.width;
				self.last_height = rect.height;
			}

			let item_list: Vec<Spans> = self.line_list.iter()
				.map(| l | {
					let style = match l.message_type {
						MessageLineType::Blank | MessageLineType::TimeDisplay | MessageLineType::Text =>
							Style::default().fg(colorscheme.text_color),
						MessageLineType::Sender =>
							Style::default().fg(colorscheme.text_color).add_modifier(Modifier::ITALIC | Modifier::BOLD),
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
							Style::default().fg(colorscheme.text_color).add_modifier(Modifier::ITALIC),
					};

					Spans::from(vec![Span::styled(l.text.as_str(), style)])
				})
				.collect();

			// this will serve as the border for the messages widget
			let messages_border = Block::default()
				.borders(Borders::ALL)
				.title(set.messages_title.as_str())
				.border_type(BorderType::Rounded)
				.border_style(Style::default().fg(
					if is_selected {
						colorscheme.selected_box
					} else {
						colorscheme.unselected_box
					}
				));

			// create the widget and scroll it to the correct location
			let mut messages_widget = Paragraph::new(item_list).block(messages_border);
			if self.messages.len() > 0 && self.line_list.len() as u16 >= rect.height {
				messages_widget = messages_widget.scroll((self.y_bounds.0, 0));
			}
			frame.render_widget(messages_widget, rect);
		}
	}

	pub fn rerender_list(&mut self, rect: Rect) {
		if let Ok(set) = SETTINGS.read() {
			let msg_width = rect.width as usize - 2;
			let opts = textwrap::Options::new((msg_width as f64 * 0.6) as usize);

			let mut last_timestamp = 0;
			let mut last_sender = "".to_string();
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
							let date_pad = Settings::date_pad_string(msg.date, msg_width);
							let mut spans = vec![
								MessageLine::blank(i),
								MessageLine::new(date_pad.to_string(), MessageLineType::TimeDisplay, i, msg.is_from_me),
								MessageLine::blank(i),
							];
							vec.append(&mut spans);
						}

						// Set the sender's name above their text if it needs to be shown
						if let Some(send) = &msg.sender {
							if *send != last_sender || msg.date - last_timestamp >= 3600000000000 {
								if msg.date - last_timestamp < 3600000000000 {
									vec.push(MessageLine::blank(i));
								}

								vec.push(MessageLine::new(send.to_string(), MessageLineType::Sender, i, msg.is_from_me));
							}

							last_sender = send.as_str().to_string();
						}

						last_timestamp = msg.date;

						// split the text into its wrapped lines
						let text_lines: Vec<String> = textwrap::fill(msg.text.as_str(), opts.clone())
							.split('\n')
							.map(|l| l.to_string())
							.collect();

						// find the length of the longest line (length calculated by utf-8 chars)
						let mut max = text_lines.iter()
							.fold(0, |m, l| {
								let len = l.graphemes(true).count();
								if len > m { len } else { m }
							});
						let space = msg_width - max;

						// add padding for my texts, put into spans
						let mut lines: Vec<MessageLine> = text_lines.into_iter()
							.map(|l| {
								let text = if msg.is_from_me {
									format!("{}{}", " ".repeat(space), l)
								} else { l };

								MessageLine::new(text, MessageLineType::Text, i, msg.is_from_me)
							})
							.collect();

						vec.append(&mut lines);

						// do attachments
						for att in msg.attachments.iter() {
							let att_line = format!("{}Attachment {}: {}",
											if msg.is_from_me { " ".repeat(space) }
											else { "".to_string() },
											att_temp.len(),
											att.mime_type);

							if att_line.len() > max {
								max = att_line.len();
							}

							vec.push(MessageLine::new(att_line, MessageLineType::Text, i, msg.is_from_me));
							att_temp.push(att.path.as_str().to_string());
						}

						// add underline so it's pretty
						let underline = format!("{}{}",
							if msg.is_from_me { " ".repeat(space) } else { "".to_string() },
							set.chat_underline.as_str().repeat(max)
						);

						vec.push(MessageLine::new(underline, MessageLineType::Underline, i, msg.is_from_me));

						vec
					}
				);

			self.attachments = att_temp;

		}

		self.y_bounds = (self.line_list.len() as u16 - rect.height, self.line_list.len() as u16 - 1);
		self.scroll(false, 0);
	}

	pub fn scroll(&mut self, up: bool, distance: u16) {

		if !up {
			// have to convert to signed to prevent overflow
			self.selected_msg = std::cmp::min(self.selected_msg + distance, self.messages.len() as u16 - 1);

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
			self.selected_msg = std::cmp::max(self.selected_msg as i32 - distance as i32, 0) as u16;

			let scroll_opt = self.line_list.iter().position(|m| m.relative_index as u16 == self.selected_msg);
			if let Some(scroll) = scroll_opt {

				if self.y_bounds.0 > scroll as u16 {
					self.y_bounds.1 -= self.y_bounds.0 - scroll as u16;
					self.y_bounds.0 = scroll as u16;
				}
			}

			if self.selected_msg == 0 {
				self.load_more_texts();
			}
		}
	}

	pub fn load_in_conversation(&mut self, id: &str) {
		self.messages = APICLIENT.get_texts(id.to_string(), None, None, None, None);
		self.messages.reverse(); // cause ya gotta

		self.last_width = 0;
		self.selected_msg = self.messages.len() as u16 - 1;
	}

	pub fn load_more_texts(&mut self) {
		let old_len = self.messages.len();

		let new_msgs_opt = if let Ok(state) = STATE.read() {
			if let Some(chat) = &state.current_chat {
				Some(APICLIENT.get_texts(
						chat.as_str().to_string(),
						None,
						Some(self.messages.len() as i64),
						None,
						None
				))
			} else { None }
		} else { None };

		if let Some(mut new_msgs) = new_msgs_opt {
			new_msgs.reverse();
			new_msgs.append(&mut self.messages);
			self.messages = new_msgs;

			self.selected_msg = old_len as u16;
			self.last_height = 0;

			if let Ok(mut state) = STATE.write() {
				state.hint_msg = "loaded in more messages".to_string();
			}
		}
	}

	pub fn new_text(&mut self, msg: Message) {
		let last = self.messages.last();
		let i = self.messages.len();

		let last_timestamp = match last {
			None => 0,
			Some(val) => val.date,
		};

		// show the time display
		if msg.date - last_timestamp >= 3600000000000 {
			let date_pad = Settings::date_pad_string(msg.date, self.last_width as usize - 2);
			let mut spans = vec![
				MessageLine::blank(i),
				MessageLine::new(date_pad, MessageLineType::Text, i, msg.is_from_me),
				MessageLine::blank(i),
			];

			self.line_list.append(&mut spans);
		}

		// Show the sender if it exists
		if let Some(send) = &msg.sender {
			if last.is_none() || send != last.unwrap().sender.as_ref().unwrap() || msg.date - last_timestamp >= 3600000000000 {
				if msg.date - last_timestamp < 3600000000000 {
					self.line_list.push(MessageLine::blank(i));
				}

				self.line_list.push(MessageLine::new(send.to_string(), MessageLineType::Sender, i, msg.is_from_me));
			}
		}

		let opts = textwrap::Options::new(((self.last_width - 2) as f64 * 0.6) as usize);

		// split the text into its wrapped lines
		let text_lines: Vec<String> = textwrap::fill(msg.text.as_str(), opts)
			.split('\n')
			.map(|l| l.to_string())
			.collect();

		// find the length of the longest line (length calculated by utf-8 chars)
		let mut max = text_lines.iter()
			.fold(0, |m, l| {
				let len = l.graphemes(true).count();
				if len > m { len } else { m }
			});
		let space = self.last_width as usize - 2 - max;

		// add padding for my texts, put into spans
		if let Ok(set) = SETTINGS.read() {

			// do attachments
			for att in msg.attachments.iter() {
				let att_line = format!("{}Attachment {}: {}",
									if msg.is_from_me { " ".repeat(space) }
									else { "".to_string() },
									self.attachments.len(),
									att.mime_type);

				if att_line.len() > max {
					max = att_line.len();
				}

				self.line_list.push(MessageLine::new(att_line, MessageLineType::Text, i, msg.is_from_me));
				self.attachments.push(att.path.as_str().to_string());
			}

			let mut lines: Vec<MessageLine> = text_lines.into_iter()
				.map(|l| {
					let text = if msg.is_from_me {
						format!("{}{}", " ".repeat(space), l)
					} else { l };

					MessageLine::new(text, MessageLineType::Text, i, msg.is_from_me)
				})
				.collect();

			self.line_list.append(&mut lines);

			// add underline so it's pretty
			let underline = format!("{}{}",
				if msg.is_from_me { " ".repeat(space) } else { "".to_string() },
				set.chat_underline.as_str().repeat(max)
			);

			self.line_list.push(MessageLine::new(underline, MessageLineType::Underline, i, msg.is_from_me));
		}

		self.messages.push(msg);

		self.selected_msg = self.messages.len() as u16 - 1;
		self.scroll(false, 0);
	}

	pub fn set_typing(&mut self, text: Message) {
		if let None = self.typing_idx {
			let model = MessageLine::new("Typing...".to_owned(), MessageLineType::Typing, self.messages.len(), false);
			self.line_list.push(model);
			self.typing_idx = Some(self.line_list.len() - 1);

			self.messages.push(text);
		}
	}

	pub fn set_idle(&mut self) {
		if let Some(id) = self.typing_idx {
			let line = &self.line_list[id];

			self.messages.remove(line.relative_index);
			self.line_list.remove(id);
		}
	}

	pub fn open_attachment(&self, idx: usize) {
		if let Ok(set) = SETTINGS.read() {

			if let Err(err) = open::that(
				set.attachment_string(
					self.attachments[idx].as_str().to_string()
				)) {

				if let Ok(mut state) = STATE.write() {
					state.hint_msg = format!("Unable to open link for attachment: {}", err);
				}

			}
		}
	}

	pub fn delete_current_text(&mut self, chat_id: &str) -> bool {
		if self.messages.len() as u16 <= self.selected_msg {
			if let Ok(mut state) = STATE.write() {
				state.hint_msg = "failed to delete text (not enough messages)".to_owned();
			}
			return false;
		}

		let identifier = &self.messages[self.selected_msg as usize].guid;
		
		let del_url = if let Ok(set) = SETTINGS.read() {
			set.delete_string(chat_id, Some(identifier))
		} else { "".to_owned() };

		if del_url.len() > 0 {
			match APICLIENT.get_url_string(&del_url) {
				Err(err) => if let Ok(mut state) = STATE.write() {
					state.hint_msg = format!("failed to delete text: {}", err);
				},
				Ok(_) => {
					if let Ok(mut state) = STATE.write() {
						state.hint_msg = "deleted text :)".to_owned();
					}
					self.load_in_conversation(chat_id);

					return true;
				},
			}
		}

		false
	}
}
