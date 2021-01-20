pub struct Colorscheme {
	pub selected_box: i16,
	pub unselected_box: i16,
	pub my_underline: i16,
	pub their_underline: i16,
	pub chat_indicator: i16,
	pub unread_indicator: i16,
	pub text_color: i16,
	pub hints_box: i16,
}

impl From<String> for Colorscheme {
	fn from(val: String) -> Self {
		let colors = match val.as_str() {
			"default" => [6, -1, 39, 248, 219, 39, 231, 9],
			"outrun" => [211, 6, 165, 238, 228, 205, 231, 209],
			"coral" => [202, 208, 251, 117, 207, 73, 7, 79],
			"forest" => [48, 36, 95, 81, 39, 207, 253, 217],
			"soft" => [152, 151, 247, 67, 44, 216, 188, 230],
			x => {
				println!("Colorscheme {} not found. Using default", x);
				[6, -1, 39, 248, 219, 39, 231, 9]
			},
		};

		Colorscheme {
			selected_box: colors[0],
			unselected_box: colors[1],
			my_underline: colors[2],
			their_underline: colors[3],
			chat_indicator: colors[4],
			unread_indicator: colors[5],
			text_color: colors[6],
			hints_box: colors[7],
		}
	}
}
