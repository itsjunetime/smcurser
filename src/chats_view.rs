use crate::*;
use crate::models::*;
use tui::{
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, BorderType},
	style::Style,
	terminal::Frame,
};

pub struct ChatsView {
	pub scroll: u16,
	pub chats: Vec<Conversation>,
	pub chats_list: Vec<String>,
	pub last_width: u16,
	pub last_height: u16,
	pub last_selected: Option<usize>,
}

impl ChatsView {
	pub fn new() -> ChatsView {
		let chats = APICLIENT.read().unwrap().get_chats(None, None);

		ChatsView {
			scroll: 0,
			chats: chats,
			chats_list: Vec::new(),
			last_width: 0,
			last_height: 0,
			last_selected: None,
		}
	}

	pub fn draw_view(&mut self, frame: &mut Frame<CrosstermBackend<io::Stdout>>, rect: Rect, is_selected: bool) {
		if let Ok(set) = SETTINGS.read() {

			if rect.width != self.last_width || rect.height != self.last_height {
				self.rerender_list(rect);

				self.last_width = rect.width;
				self.last_height = rect.height;
			}

			let item_list: Vec<Spans> = self.chats_list.iter()
				.fold(Vec::new(), |mut s, c| {
					let (num, rest) = c.split_at(4); // that's where the symbol will be
					let symbol = rest.chars().nth(0).unwrap();

					let spans = vec![
						Span::styled(num, Style::default().fg(set.colorscheme.text_color)),
						match symbol {
							_ if symbol == set.current_chat_indicator =>
								Span::styled(String::from(symbol), Style::default().fg(set.colorscheme.chat_indicator)),
							_ if symbol == set.unread_chat_indicator =>
								Span::styled(String::from(symbol), Style::default().fg(set.colorscheme.unread_indicator)),
							_ => Span::raw(" "),
						},
						Span::styled(rest.replacen(symbol, "", 1), Style::default().fg(set.colorscheme.text_color)),
					];

					s.push(Spans::from(vec![Span::raw("")]));
					s.push(Spans::from(spans));
					s
				});

			let chats_border = Block::default()
				.borders(Borders::ALL)
				.title(set.chats_title.as_str())
				.border_type(BorderType::Rounded)
				.border_style(Style::default().fg(
						if is_selected {
							set.colorscheme.selected_box
						} else {
							set.colorscheme.unselected_box
						}));

			let chats_widget = Paragraph::new(item_list)
				.block(chats_border)
				.scroll((self.scroll * 2, 0));

			frame.render_widget(chats_widget, rect);
		}
	}

	pub fn rerender_list(&mut self, rect: Rect) {
		let max_len: usize = (rect.width as u64 - 8) as usize;

		if let Ok(set) = SETTINGS.read() {
			self.chats_list = self.chats.iter()
				.enumerate()
				.map(|(i, c)| {
					let symbol = if c.is_selected {
						set.current_chat_indicator
					} else {
						if c.has_unread {
							set.unread_chat_indicator
						} else {
							' '
						}
					};

					let name = if c.display_name.len() > max_len {
						format!("{}...", &c.display_name[..max_len - 3])
					} else {
						c.display_name.as_str().to_string()
					};

					let idx = format!("{}{}{}",
						if i < 100 { " " } else { "" },
						if i < 10 { " " } else { "" },
						i
					); // I'm just gonna hope that nobody is going 1000 chats deep lol

					format!("{} {} {}", idx, symbol, name)
				})
				.collect();
		}
	}

	pub fn scroll(&mut self, up: bool, distance: u16) {
		if !up {
			let max = self.chats_list.len() as u16 - (self.last_height / 2) + 2;
			self.scroll = std::cmp::min(self.scroll + distance, max);

			if self.scroll == max {
				let mut new_chats = APICLIENT.read()
					.unwrap().get_chats(None, Some(self.chats.len() as i64));

				self.chats.append(&mut new_chats);
				self.last_height = 0;
			}
		} else {
			self.scroll = std::cmp::max(self.scroll as i32 - distance as i32, 0) as u16;
		}
	}

	pub fn load_in_conversation(&mut self, idx: usize) {
		if let Some(old) = self.last_selected {
			self.chats[old].is_selected = false;
		}

		let mut chat = &mut (self.chats[idx]);
		chat.has_unread = false;
		chat.is_selected = true;

		self.last_selected = Some(idx);
		self.last_height = 0; // kinda dirty trick to force it to redraw the list next time
	}

	pub fn new_text(&mut self, item: &Message) -> Option<usize> {
		let mut ret: Option<usize> = None;

		if let Some(id) = &item.chat_identifier {
			let chat = self.chats.iter().position(|c| c.chat_identifier == *id);

			if let Some(idx) = chat {
				let mut old_chat = self.chats.remove(idx);
				if !item.is_from_me { old_chat.has_unread = true; }

				if let Some(ls) = self.last_selected {
					if idx == ls {
						self.last_selected = Some(0);
						old_chat.has_unread = false;
					} else if idx > ls {
						self.last_selected = Some(ls + 1);
					}
				}

				self.chats.insert(0, old_chat);

				ret = Some(idx);

				self.last_height = 0;
			}
		}

		// this doesn't account for the possibility of somebody sending you a text who hasn't
		// texted you recently. I need to fix that up.
		ret
	}

	pub fn reload_chats(&mut self)  {
		self.chats = APICLIENT.read().unwrap().get_chats(None, None);

		print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
	}
}
