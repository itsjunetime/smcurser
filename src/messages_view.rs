use crate::*;
use crate::models::*;
use tui::{
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, BorderType},
	style::{Style, Color},
	terminal::Frame,
};
use unicode_segmentation::UnicodeSegmentation;

pub struct MessagesView {
	pub scroll: u16,
	pub messages: Vec<Message>,
	pub messages_list: Vec<String>,
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
				.map(|m| Spans::from(vec![Span::raw(m.as_str())]))
				.collect();

			// this will serve as the border for the messages widget
			let mut messages_border = Block::default()
				.borders(Borders::ALL)
				.title(set.messages_title.as_str())
				.border_type(BorderType::Rounded);

			// if it's selected, color it correctly
			if is_selected {
				messages_border = messages_border.border_style(Style::default().fg(Color::Blue));
			}

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
							total_msg_height += 3;
						}

						last_timestamp = msg.date;

						// split the text into its wrapped lines
						let text_lines: Vec<String> = textwrap::fill(msg.text.as_str(), opts.clone())
							.split('\n')
							.map(|l| l.to_string())
							.collect();

						// find the length of the longest line (length calculated by utf-8 chars)
						let max = text_lines.iter().fold(
									0, |m, l| {
										let len = l.graphemes(true).count();
										if len > m { len } else { m }
									});
						let space = msg_width - max;

						// add padding for my texts, put into spans
						let mut lines: Vec<String> = text_lines.into_iter()
							.map(|l| {
								total_msg_height += 1;
								if msg.is_from_me { format!("{}{}", " ".repeat(space), l) }
								else { l }
							})
							.collect();

						vec.append(&mut lines);

						// add underline so it's pretty
						let underline = format!("{}{}",
							if msg.is_from_me { " ".repeat(space) } else { "".to_string() },
							set.chat_underline.as_str().repeat(max)
						);

						vec.push(underline);
						total_msg_height += 1;

						vec
					}
				);

			self.total_height = total_msg_height;

		}
	}

	pub fn scroll(&mut self, up: bool, distance: u16) {
		// I don't understand how this logic works. But it does.

		if up {
			// have to convert to signed to prevent overflow
			self.scroll = std::cmp::max(self.scroll as i32 - distance as i32, 0) as u16;
		} else {
			self.scroll = std::cmp::min(self.scroll + distance, self.total_height as u16 - self.last_height);
		}
	}

	pub fn load_in_conversation(&mut self, id: String) {
		self.messages = APICLIENT.read().unwrap().get_texts(id, None, None, None, None);
		self.messages.reverse(); // cause ya gotta

		self.last_width = 0;
		self.scroll = 0;
	}
	
	pub fn new_text(&mut self, msg: &Message) {
		let last_timestamp = self.messages.last().unwrap().date;

		if msg.date - last_timestamp >= 3600000000000 {
			let mut spans = vec![
				"".to_string(),
				Settings::date_pad_string(msg.date, self.last_width as usize - 2),
				"".to_string(),
			];
			self.messages_list.append(&mut spans);
			self.total_height += 3;
		}

		let opts = textwrap::Options::new(((self.last_width - 2) as f64 * 0.6) as usize);

		// split the text into its wrapped lines
		let text_lines: Vec<String> = textwrap::fill(msg.text.as_str(), opts)
			.split('\n')
			.map(|l| l.to_string())
			.collect();

		// find the length of the longest line (length calculated by utf-8 chars)
		let max = text_lines.iter().fold(
					0, |m, l| {
						let len = l.graphemes(true).count();
						if len > m { len } else { m }
					});
		let space = self.last_width as usize - 2 - max;

		// add padding for my texts, put into spans
		let mut lines: Vec<String> = text_lines.into_iter()
			.map(|l| {
				self.total_height += 1;
				if msg.is_from_me { format!("{}{}", " ".repeat(space), l) }
				else { l }
			})
			.collect();

		self.messages_list.append(&mut lines);

		// add underline so it's pretty
		if let Ok(set) = SETTINGS.read() {
			let underline = format!("{}{}",
				if msg.is_from_me { " ".repeat(space) } else { "".to_string() },
				set.chat_underline.as_str().repeat(max)
			);

			self.messages_list.push(underline);
			self.total_height += 1;
		}
	}
}
