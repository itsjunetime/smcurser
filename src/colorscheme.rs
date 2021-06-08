use tui::style::Color;
use std::collections::HashMap;
use crate::SETTINGS;

#[derive(Clone)]
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
				if let Some(cl) = colors.iter().find(|c| c.name == name) {
					return cl.to_owned();
				}
			}
		}


		let vals = match name.as_str() {
			"rose-pine" => [
				[156, 207, 216],
				[49, 116, 143],
				[196, 167, 231],
				[235, 188, 186],
				[156, 207, 216],
				[246, 193, 119],
				[235, 111, 146],
				[224, 222, 244],
				[112, 110, 134],
			],
			"hacker" => [
				[32, 160, 14],
				[120, 120, 120],
				[32, 160, 14],
				[120, 120, 120],
				[255, 255, 255],
				[32, 160, 14],
				[32, 160, 14],
				[236, 236, 236],
				[32, 160, 14],
			],
			"dracula" => [
				[139, 233, 253],
				[98, 114, 164],
				[189, 147, 249],
				[68, 71, 90],
				[80, 250, 123],
				[255, 121, 198],
				[255, 184, 108],
				[248, 248, 242],
				[80, 250, 123],
			],
			_ => [ // forest
				[36, 139, 84],
				[28, 102, 83],
				[101, 215, 253],
				[134, 95, 96],
				[245, 111, 66],
				[30, 141, 199],
				[245, 111, 66],
				[255, 255, 255],
				[195, 137, 138]
			],
		};

		Colorscheme {
			name,
			selected_box: Color::Rgb(vals[0][0], vals[0][1], vals[0][2]),
			unselected_box: Color::Rgb(vals[1][0], vals[1][1], vals[1][2]),
			my_underline: Color::Rgb(vals[2][0], vals[2][1], vals[2][2]),
			their_underline: Color::Rgb(vals[3][0], vals[3][1], vals[3][2]),
			selected_underline: Color::Rgb(vals[4][0], vals[4][1], vals[4][2]),
			chat_indicator: Color::Rgb(vals[5][0], vals[5][1], vals[5][2]),
			unread_indicator: Color::Rgb(vals[6][0], vals[6][1], vals[6][2]),
			text_color: Color::Rgb(vals[7][0], vals[7][1], vals[7][2]),
			hints_box: Color::Rgb(vals[8][0], vals[8][1], vals[8][2]),
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
			name,
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
