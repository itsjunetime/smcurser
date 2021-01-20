use crate::*;
use crate::models::*;
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap, BorderType},
	style::{Style, Color},
	terminal::Frame,
};
use unicode_segmentation::UnicodeSegmentation;

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
					s.push(Spans::from(vec![Span::raw("")]));
					s.push(Spans::from(vec![Span::raw(c.as_str())]));
					s
				});

			let mut chats_border = Block::default()
				.borders(Borders::ALL)
				.title(set.chats_title.as_str())
				.border_type(BorderType::Rounded);
			if is_selected {
				chats_border = chats_border.border_style(Style::default().fg(Color::Magenta));
			}

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
				let symbol = if c.is_selected { " > " } else {
					if c.has_unread { " • " } else { "   " }
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

				format!("{}{}{}", idx, symbol, name)
			})
			.collect();
	}

	pub fn scroll(&mut self, up: bool) {
		if up && self.scroll < self.chats.len() as u16 {
			self.scroll += 1;
		} else if !up && self.scroll > 0 {
			self.scroll -= 1;
		}
	}

	pub fn load_in_conversation(&mut self, idx: usize) {
		if let Some(old) = self.last_selected {
			self.chats[old].is_selected = false;
		}

		let mut chat = &mut (self.chats[idx]);
		chat.has_unread = false;
		chat.is_selected = true;

		self.last_height = 0; // kinda dirty trick to force it to redraw the list next time
	}
}
