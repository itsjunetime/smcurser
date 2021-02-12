use tui::style::Color;
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
pub struct Colorscheme {
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
		match val.into().as_str() {
			"forest" => Colorscheme {
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
			_ => Colorscheme::from("forest"),
		}
	}
}
