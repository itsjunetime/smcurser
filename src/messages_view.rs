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
use std::io::Write;

pub struct MessagesView {
	pub scroll: u16,
	pub messages: Vec<Message>,
	pub messages_list: Vec<String>,
	pub attachments: Vec<String>,
	pub line_color_map: Vec<Style>,
	pub last_width: u16,
	pub last_height: u16,
	pub total_height: u32,
}

impl MessagesView {
	pub fn new() -> MessagesView {
		MessagesView {
			scroll: 0,
			messages: Vec::new(),
			messages_list: Vec::new(),
			attachments: Vec::new(),
			line_color_map: Vec::new(),
			last_width: 0,
			last_height: 0,
			total_height: 0,
		}
	}

	pub fn draw_view(&mut self, frame: &mut Frame<CrosstermBackend<io::Stdout>>, rect: Rect, is_selected: bool) {
		if let Ok(set) = SETTINGS.read() {
			if rect.width != self.last_width || rect.height != self.last_height {
				self.rerender_list(rect);

				self.last_width = rect.width;
				self.last_height = rect.height;
			}

			let item_list: Vec<Spans> = self.messages_list.iter()
				.enumerate()
				.map(|(i, m)| Spans::from(vec![Span::styled(m.as_str(), self.line_color_map[i])]))
				.collect();

			// this will serve as the border for the messages widget
			let messages_border = Block::default()
				.borders(Borders::ALL)
				.title(set.messages_title.as_str())
				.border_type(BorderType::Rounded)
				.border_style(Style::default().fg(if is_selected { set.colorscheme.selected_box } else { set.colorscheme.unselected_box }));

			// create the widget and scroll it to the correct location
			let mut messages_widget = Paragraph::new(item_list).block(messages_border);
			if self.messages.len() > 0 && self.total_height as u16 >= rect.height {
				messages_widget = messages_widget.scroll((self.total_height as u16 - rect.height + 2 - self.scroll, 0));
			}
			frame.render_widget(messages_widget, rect);
		}
	}

	pub fn rerender_list(&mut self, rect: Rect) {
		if let Ok(set) = SETTINGS.read() {
			let msg_width = rect.width as usize - 2;
			let opts = textwrap::Options::new((msg_width as f64 * 0.6) as usize);

			let mut last_timestamp = 0;
			let mut total_msg_height = 0;
			let mut last_sender = "".to_string();

			let mut lcm_temp = Vec::new();
			let mut att_temp = Vec::new();

			// This gets a vector of spans for all the messages. It handles stuff like
			// inserting the time when necessary, adding the underlines, splitting the
			// texts into lines of correct length, etc.
			self.messages_list = self.messages.iter()
				.enumerate()
				.fold(
					Vec::new(), |mut vec, (_, msg)| {
						// check, add time display if necessary
						if msg.date - last_timestamp >= 3600000000000 {
							let mut spans = vec![
								"".to_string(),
								Settings::date_pad_string(msg.date, msg_width),
								"".to_string(),
							];
							vec.append(&mut spans);

							lcm_temp.append(&mut vec![Style::default().fg(set.colorscheme.text_color); 3]);

							total_msg_height += 3;
						}

						// Set the sender's name above their text if it needs to be shown
						if let Some(send) = &msg.sender {
							if *send != last_sender || msg.date - last_timestamp >= 3600000000000 {
								if msg.date - last_timestamp < 3600000000000 {
									vec.push("".to_string());
									lcm_temp.push(Style::default());
									total_msg_height += 1;
								}

								vec.push(send.to_string());
								lcm_temp.push(Style::default().add_modifier(Modifier::ITALIC | Modifier::BOLD));

								total_msg_height += 1;
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
						let max = text_lines.iter()
							.fold(0, |m, l| {
								let len = l.graphemes(true).count();
								if len > m { len } else { m }
							});
						let space = msg_width - max;

						// add padding for my texts, put into spans
						let mut lines: Vec<String> = text_lines.into_iter()
							.map(|l| {
								total_msg_height += 1;
								lcm_temp.push(Style::default().fg(set.colorscheme.text_color));

								if msg.is_from_me { format!("{}{}", " ".repeat(space), l) }
								else { l }
							})
							.collect();

						vec.append(&mut lines);

						// do attachments
						for att in msg.attachments.iter() {
							lcm_temp.push(Style::default().fg(set.colorscheme.text_color));
							vec.push(format!("Attachment {}: {}", att_temp.len(), att.mime_type));
							att_temp.push(att.path.as_str().to_string());
							total_msg_height += 1;
						}

						// add underline so it's pretty
						let underline = format!("{}{}",
							if msg.is_from_me { " ".repeat(space) } else { "".to_string() },
							set.chat_underline.as_str().repeat(max)
						);

						lcm_temp.push(Style::default().fg(
							if msg.is_from_me {
								set.colorscheme.my_underline
							} else {
								set.colorscheme.their_underline
							}
						));

						vec.push(underline);
						total_msg_height += 1;

						vec
					}
				);

			self.attachments = att_temp;
			self.line_color_map = lcm_temp;
			self.total_height = total_msg_height;

		}
	}

	pub fn scroll(&mut self, up: bool, distance: u16) {

		if !up {
			// have to convert to signed to prevent overflow
			self.scroll = std::cmp::max(self.scroll as i32 - distance as i32, 0) as u16;
		} else {
			let max = self.total_height as u16 - self.last_height;
			self.scroll = std::cmp::min(self.scroll + distance, max);

			if self.scroll == max {
				if let Ok(state) = STATE.read() {
					if let Some(chat) = &state.current_chat {
						let mut new_msgs = APICLIENT.read()
							.unwrap().get_texts(chat.as_str().to_string(), None, Some(self.messages.len() as i64), None, None);

						new_msgs.reverse();
						new_msgs.append(&mut self.messages);
						self.messages = new_msgs;

						self.last_height = 0;
					}
				}
			}
		}
	}

	pub fn load_in_conversation(&mut self, id: &String) {
		self.messages = APICLIENT.read().unwrap().get_texts(id.as_str().to_string(), None, None, None, None);
		self.messages.reverse(); // cause ya gotta

		self.last_width = 0;
		self.scroll = 0;
	}

	pub fn new_text(&mut self, msg: Message) {
		let last = self.messages.last();

		let last_timestamp = match last {
			None => 0,
			Some(val) => val.date,
		};

		// show the time display
		if let Ok(set) = SETTINGS.read() {
			if msg.date - last_timestamp >= 3600000000000 {
				let mut spans = vec![
					"".to_string(),
					Settings::date_pad_string(msg.date, self.last_width as usize - 2),
					"".to_string(),
				];
				self.messages_list.append(&mut spans);

				self.line_color_map.append(&mut vec![Style::default().fg(set.colorscheme.text_color); 3]);

				self.total_height += 3;
			}
		}

		// Show the sender if it exists
		if let Some(send) = &msg.sender {
			if last.is_none() || send != last.unwrap().sender.as_ref().unwrap() || msg.date - last_timestamp >= 3600000000000 {
				if msg.date - last_timestamp < 3600000000000 {
					self.messages_list.push("".to_string());
					self.line_color_map.push(Style::default());
					self.total_height += 1;
				}

				self.messages_list.push(send.to_string());
				self.line_color_map.push(Style::default().add_modifier(Modifier::ITALIC | Modifier::BOLD));

				self.total_height += 1;
			}
		}

		let opts = textwrap::Options::new(((self.last_width - 2) as f64 * 0.6) as usize);

		// split the text into its wrapped lines
		let text_lines: Vec<String> = textwrap::fill(msg.text.as_str(), opts)
			.split('\n')
			.map(|l| l.to_string())
			.collect();

		// find the length of the longest line (length calculated by utf-8 chars)
		let max = text_lines.iter()
			.fold(0, |m, l| {
				let len = l.graphemes(true).count();
				if len > m { len } else { m }
			});
		let space = self.last_width as usize - 2 - max;

		// add padding for my texts, put into spans
		if let Ok(set) = SETTINGS.read() {

			// do attachments
			for att in msg.attachments.iter() {
				self.line_color_map.push(Style::default().fg(set.colorscheme.text_color));
				self.messages_list.push(format!("Attachment {}: {}", self.attachments.len(), att.mime_type));
				self.attachments.push(att.path.as_str().to_string());
				self.total_height += 1;
			}

			let mut lines: Vec<String> = text_lines.into_iter()
				.map(|l| {
					self.total_height += 1;
					self.line_color_map.push(Style::default().fg(set.colorscheme.text_color));

					if msg.is_from_me { format!("{}{}", " ".repeat(space), l) }
					else { l }
				})
				.collect();

			self.messages_list.append(&mut lines);

			// add underline so it's pretty
			let underline = format!("{}{}",
				if msg.is_from_me { " ".repeat(space) } else { "".to_string() },
				set.chat_underline.as_str().repeat(max)
			);

			self.line_color_map.push(Style::default().fg(
				if msg.is_from_me {
					set.colorscheme.my_underline
				} else {
					set.colorscheme.their_underline
				}
			));

			self.messages_list.push(underline);
			self.total_height += 1;
		}

		self.messages.push(msg);
	}

	pub fn open_attachment(&self, idx: usize) {
		if let Ok(set) = SETTINGS.read() {
			open::that(
				format!("http{}://{}:{}/data?path={}",
					if set.secure { "s" } else { "" },
					set.host,
					set.server_port,
					self.attachments[idx],
				)
			);
		}
	}
}
