use crate::*;
use crate::models::*;
use tui::{
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, BorderType},
	style::{Style, Color},
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
							'>' => Span::styled(String::from(symbol), Style::default().fg(set.colorscheme.chat_indicator)),
							'•' => Span::styled(String::from(symbol), Style::default().fg(set.colorscheme.unread_indicator)),
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
				.border_style(Style::default().fg(if is_selected { set.colorscheme.selected_box } else { set.colorscheme.unselected_box }));

			let chats_widget = Paragraph::new(item_list)
				.block(chats_border)
				.scroll((self.scroll * 2, 0));

			frame.render_widget(chats_widget, rect);
		}
	}

	pub fn rerender_list(&mut self, rect: Rect) {
		let max_len: usize = (rect.width as u64 - 8) as usize;

		self.chats_list = self.chats.iter()
			.enumerate()
			.map(|(i, c)| {
				let symbol = if c.is_selected { ">" } else {
					if c.has_unread { "•" } else { " " }
				};

				let name = if c.display_name.len() > max_len {
					format!("{}...", &c.display_name[..max_len - 3])
				} else {
					c.display_name.as_str().to_string()
				};

				let l = i + self.scroll as usize;

				let idx = format!("{}{}{}",
					if l < 100 { " " } else { "" },
					if l < 10 { " " } else { "" },
					l
				); // I'm just gonna hope that nobody is going 1000 chats deep lol

				format!("{} {} {}", idx, symbol, name)
			})
			.collect();
	}

	pub fn scroll(&mut self, up: bool, distance: u16) {
		if up {
			self.scroll = std::cmp::min(self.scroll + distance, self.chats.len() as u16);
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

	pub fn new_text(&mut self, item: &Message) {
		if let Some(id) = &item.chat_identifier {
			let chat = self.chats.iter().position(|c| c.chat_identifier == *id);

			if let Some(idx) = chat {
				let mut old_chat = self.chats.remove(idx);
				old_chat.has_unread = true;

				self.chats.insert(0, old_chat);
			}
		}
	}

	pub fn reload_chats(&mut self)  {
		self.chats = APICLIENT.read().unwrap().get_chats(None, None);
	}
}
