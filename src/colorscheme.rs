use tui::style::Color;
use serde::Deserialize;
use std::collections::HashMap;
use crate::SETTINGS;

#[derive(Clone, Deserialize)]
pub struct Colorscheme {
	pub name: String,
	pub selected_box: Color,
	pub unselected_box: Color,
	pub my_underline: Color,
	pub their_underline: Color,
	pub selected_underline: Color,
	pub chat_indicator: Color,
	pub unread_indicator: Color,
	pub text_color: Color,
	pub hints_box: Color,
}

impl<T: Into<String>> From<T> for Colorscheme {
	fn from(val: T) -> Self {
		// yeahhh... ugly. Whatcha gonna do

		let name: String = val.into();

		if let Ok(set) = SETTINGS.read() {
			if let Some(ref colors) = set.custom_colorschemes {
				for color in colors {
					if color.name == name {
						return color.to_owned();
					}
				}
			}
		}

		match name.as_str() {
			"forest" => Colorscheme {
				name: name,
				selected_box: Color::Rgb(36, 139, 84),
				unselected_box: Color::Rgb(28, 102, 83),
				my_underline: Color::Rgb(101, 215, 253),
				their_underline: Color::Rgb(134, 95, 96),
				selected_underline: Color::Rgb(245, 111, 66),
				chat_indicator: Color::Rgb(30, 141, 199),
				unread_indicator: Color::Rgb(245, 111, 66),
				text_color: Color::White,
				hints_box: Color::Rgb(195, 137, 138),
			},
			"rose-pine" => Colorscheme {
				name: name,
				selected_box: Color::Rgb(156, 207, 216),
				unselected_box: Color::Rgb(49, 116, 143),
				my_underline: Color::Rgb(196, 167, 231),
				their_underline: Color::Rgb(235, 188, 186),
				selected_underline: Color::Rgb(156, 207, 216),
				chat_indicator: Color::Rgb(246, 193, 119),
				unread_indicator: Color::Rgb(235, 111, 146),
				text_color: Color::Rgb(224, 222, 244),
				hints_box: Color::Rgb(112, 110, 134),
			},
			"hacker" => Colorscheme {
				name: name,
				selected_box: Color::Rgb(32, 160, 14),
				unselected_box: Color::Rgb(120, 120, 120),
				my_underline: Color::Rgb(32, 160, 14),
				their_underline: Color::Rgb(120, 120, 120),
				selected_underline: Color::White,
				chat_indicator: Color::Rgb(32, 160, 14),
				unread_indicator: Color::Rgb(32, 160, 14),
				text_color: Color::Rgb(236, 236, 236),
				hints_box: Color::Rgb(32, 160, 14),
			},
			"dracula" => Colorscheme {
				name: name,
				selected_box: Color::Rgb(139, 233, 253),
				unselected_box: Color::Rgb(98, 114, 164),
				my_underline: Color::Rgb(189, 147, 249),
				their_underline: Color::Rgb(68, 71, 90),
				selected_underline: Color::Rgb(80, 250, 123),
				chat_indicator: Color::Rgb(255, 121, 198),
				unread_indicator: Color::Rgb(255, 184, 108),
				text_color: Color::Rgb(248, 248, 242),
				hints_box: Color::Rgb(80, 250, 123),
			},
			_ => Colorscheme::from("forest"),
		}
	}
}

impl Colorscheme {
	// this does no validation at all. Will panic if anything is off
	pub fn from_specs(name: String, map: HashMap<String, Vec<u8>>) -> Colorscheme {

		let (sb, ub, mu, tu, su, ci, ui, tc, hb) =
			(
				&map["selected_box"],
				&map["unselected_box"],
				&map["my_underline"],
				&map["their_underline"],
				&map["selected_underline"],
				&map["chat_indicator"],
				&map["unread_indicator"],
				&map["text_color"],
				&map["hints_box"]
			);

		Colorscheme {
			name: name,
			selected_box: Color::Rgb(sb[0], sb[1], sb[2]),
			unselected_box: Color::Rgb(ub[0], ub[1], ub[2]),
			my_underline: Color::Rgb(mu[0], mu[1], mu[2]),
			their_underline: Color::Rgb(tu[0], tu[1], tu[2]),
			selected_underline: Color::Rgb(su[0], su[1], su[2]),
			chat_indicator: Color::Rgb(ci[0], ci[1], ci[2]),
			unread_indicator: Color::Rgb(ui[0], ui[1], ui[2]),
			text_color: Color::Rgb(tc[0], tc[1], tc[2]),
			hints_box: Color::Rgb(hb[0], hb[1], hb[2]),
		}
	}
}
