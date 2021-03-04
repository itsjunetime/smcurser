use crate::*;
use crate::models::*;
use tui::{
    layout::Rect,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, BorderType},
	style::Style,
	terminal::Frame,
};
use std::{
	cmp::{min, max},
	io::Stdout,
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
		let chats = APICLIENT.get_chats(None, None);

		ChatsView {
			scroll: 0,
			chats: chats,
			chats_list: Vec::new(),
			last_width: 0,
			last_height: 0,
			last_selected: None,
		}
	}

	pub fn draw_view(&mut self, frame: &mut Frame<CrosstermBackend<Stdout>>, rect: Rect, is_selected: bool) {
		// draws the view for this specific struct

		if let Ok(set) = SETTINGS.read() {
			let colorscheme = colorscheme::Colorscheme::from(&set.colorscheme);

			// conditionally rerender the strings that make up the view; better performance
			if rect.width != self.last_width || rect.height != self.last_height {
				self.rerender_list(rect);

				self.last_width = rect.width;
				self.last_height = rect.height;
			}

			// create the list of spans, which are what is printed with `tui`.
			let item_list: Vec<Spans> = self.chats_list.iter()
				.fold(Vec::new(), |mut s, c| {
					let (num, rest) = c.split_at(4); // that's where the symbol will be
					let symbol = rest.chars().nth(0).unwrap_or(' ');

					// conditionally color the symbol and create its span
					let spans = vec![
						Span::styled(num, Style::default().fg(colorscheme.text_color)),
						match symbol {
							_ if symbol == set.current_chat_indicator =>
								Span::styled(String::from(symbol), Style::default().fg(colorscheme.chat_indicator)),
							_ if symbol == set.unread_chat_indicator =>
								Span::styled(String::from(symbol), Style::default().fg(colorscheme.unread_indicator)),
							_ => Span::raw(" "),
						},
						Span::styled(rest.replacen(symbol, "", 1), Style::default().fg(colorscheme.text_color)),
					];

					// add spacing and line of text
					s.push(Spans::from(vec![Span::raw("")]));
					s.push(Spans::from(spans));
					s
				});

			// create the border for the view
			let chats_border = Block::default()
				.borders(Borders::ALL)
				.title(set.chats_title.as_str())
				.border_type(BorderType::Rounded)
				.border_style(Style::default().fg(
						if is_selected {
							colorscheme.selected_box
						} else {
							colorscheme.unselected_box
						}));

			// create the actual view that will be printed
			let chats_widget = Paragraph::new(item_list)
				.block(chats_border)
				.scroll((self.scroll * 2, 0));

			// render it!
			frame.render_widget(chats_widget, rect);
		}
	}

	pub fn rerender_list(&mut self, rect: Rect) {
		let max_len: usize = (rect.width as u64 - 8) as usize;

		if let Ok(set) = SETTINGS.read() {

			// iterate over all of them and create the list of strings that will be printed
			self.chats_list = self.chats.iter()
				.enumerate()
				.map(|(i, c)| {
					// get symbol for the chat that will represent whether it has an unread
					// message, is selected, or neither.
					let symbol = if c.is_selected {
						set.current_chat_indicator
					} else {
						if c.has_unread {
							set.unread_chat_indicator
						} else {
							' '
						}
					};

					// only show what part of the name will fit, with ellipsis.
					let name = if c.display_name.len() > max_len {
						format!("{}...", &c.display_name[..max_len - 3])
					} else {
						c.display_name.to_owned()
					};

					// index; number that they will have to use to select the chat
					let idx = format!("{}{}{}",
						if i < 100 { " " } else { "" },
						if i < 10 { " " } else { "" },
						i
					); // I'm just gonna hope that nobody is going 1000 chats deep lol

					// like '  0 > John Smith         '
					format!("{} {} {}", idx, symbol, name)
				})
				.collect();
		}
	}

	pub fn scroll(&mut self, up: bool, distance: u16) {
		// allow people to scroll multiple lines at once
		if !up {
			// only scroll to lower limit
			let max = self.chats_list.len() as u16 - (self.last_height / 2) + 2;
			self.scroll = min(self.scroll + distance, max);

			// load in new texts automatically if you hit the limit
			if self.scroll == max {
				let mut new_chats = APICLIENT.get_chats(None, Some(self.chats.len() as i64));

				self.chats.append(&mut new_chats);
				self.last_height = 0;
			}
		} else {
			// only scroll to upper limit
			self.scroll = max(self.scroll as i32 - distance as i32, 0) as u16;
		}
	}

	pub fn load_in_conversation(&mut self, idx: usize) {
		// de-select old conversation
		if let Some(old) = self.last_selected {
			self.chats[old].is_selected = false;
		}

		// set specifics for new chat
		let mut chat = &mut self.chats[idx];
		chat.has_unread = false;
		chat.is_selected = true;

		self.last_selected = Some(idx);
		self.last_height = 0; // kinda dirty trick to force it to redraw the list next time
	}

	pub fn new_text(&mut self, item: &Message) -> Option<usize> {
		let mut ret: Option<usize> = None;

		// Make sure that the new text has a chat identifier -- it should, if it came through the
		// WebSocket, which it must have.
		if let Some(id) = &item.chat_identifier {
			// check if the conversation already is on the list that is showing.
			let chat = self.chats.iter().position(|c| c.chat_identifier == *id);

			// if it is...
			if let Some(idx) = chat {
				// remove it from the list, set to unread.
				let mut old_chat = self.chats.remove(idx);
				if !item.is_from_me { old_chat.has_unread = true; }

				// last_selected specifies the conversation whose messages
				// are currently being viewed
				if let Some(ls) = self.last_selected {
					// if it's this conversation thatis selected...
					if idx == ls {
						// set the selected index to 0, since this will be at the top.
						self.last_selected = Some(0);
						// also set it back to unread since you currently have it selected
						old_chat.has_unread = false;
					} else if idx > ls {
						// if the new text conversation will be moved to a place before
						// the currently selected conversation in the list, increase the currently
						// selected index.
						self.last_selected = Some(ls + 1);
					}
				}

				// ret will contain the old index of the chat that contains this conversation
				ret = chat;

				// insert it at the top
				self.chats.insert(0, old_chat);
			} else {
				// get the name of the conversation -- it's the only information we need to create
				// a new Conversation object.
				let name = APICLIENT.get_name(id);

				let new_convo = Conversation {
					display_name: name,
					chat_identifier: id.to_owned(),
					latest_text: item.text.to_owned(),
					has_unread: true,
					addresses: id.to_owned(),
					is_selected: false
				};

				// Must increase the currently selected index if one is selected, since this chat
				// won't be on the list.
				if let Some(ls) = self.last_selected {
					self.last_selected = Some(ls + 1);
				}

				// insert it at top
				self.chats.insert(0, new_convo);
			}

			// force it to redraw
			self.last_height = 0;
		}

		ret
	}

	pub fn reload_chats(&mut self)  {
		self.chats = APICLIENT.get_chats(None, None);
	}
}
