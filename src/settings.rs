use crate::*;
use crate::colorscheme::*;
use chrono::prelude::*;

pub struct Settings {
	pub host: String,
	pub fallback_host: String,
	pub server_port: u16,
	pub socket_port: u16,
	pub secure: bool,
	pub notifications: bool,
	pub authenticated: bool,
	pub password: String,
	pub current_chat_indicator: String,
	pub my_chat_end: String,
	pub their_chat_end: String,
	pub chat_underline: String,
	pub chats_title: String,
	pub messages_title: String,
	pub input_title: String,
	pub help_title: String,
	pub to_title: String,
	pub compose_title: String,
	pub colorscheme: Colorscheme,
	pub poll_exit: u16,
	pub timeout: u16,
	pub max_past_commands: u16,
	pub show_help: bool,
}

impl Settings {
	pub fn default() -> Settings {
		Settings {
			host: "".to_owned(),
			fallback_host: "".to_owned(),
			server_port: 8741,
			socket_port: 8740,
			secure: true,
			notifications: true,
			authenticated: false,
			password: "toor".to_owned(),
			current_chat_indicator: ">".to_owned(),
			my_chat_end: "⧹▏".to_owned(),
			their_chat_end: "▕⧸".to_owned(),
			chat_underline: "▔".to_owned(),
			chats_title: "| chats |".to_owned(),
			messages_title: "| messages |".to_owned(),
			input_title: "| input here :) |".to_owned(),
			help_title: "| help |".to_owned(),
			to_title: "| to: |".to_owned(),
			compose_title: "| message: |".to_owned(),
			colorscheme: Colorscheme::from(String::from("forest")),
			poll_exit: 10,
			timeout: 10,
			max_past_commands: 10,
			show_help: false,
		}
	}

	pub fn date_pad_string(date: i64, width: usize) -> String {
		let unix_timestamp = (date / 1000000000) + 978307200;
		let naive = NaiveDateTime::from_timestamp(unix_timestamp, 0);
		let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
		let format = datetime.format("%m/%d/%Y %H:%M").to_string();

		let pad = (width - format.len()) / 2;
		format!("{}{}{}", " ".repeat(pad), format, " ".repeat(pad))
	}

	pub fn push_to_req_url(&self, end: String) -> String {
		let s = if self.secure { "s" } else { "" };
		format!("http{}://{}:{}/{}", s, self.host, self.server_port, end)
	}

	pub fn pass_req_string(&self, pass: Option<String>) -> String {
		let p = pass.unwrap_or(self.password.clone());
		self.push_to_req_url(format!("requests?password={}", p))
	}

	pub fn msg_req_string(
		&self, chat: String, num: Option<i64>, offset: Option<i64>, read: Option<bool>, from: Option<i8>
	) -> String {
		let c = chat.to_owned();

		let ns = match num {
			Some(val) => format!("&num_messages={}", val),
			None => String::from("")
		};

		let os = if offset == None { String::from("") } else { format!("&messages_offset={}", offset.unwrap()) };

		let rs = match read {
			None => "",
			Some(val) => if val { "&read_messages=true" } else { "&read_messages=false" },
		};

		let fs = if from == None { String::from("") } else { format!("&messages_from={}", from.unwrap()) };

		self.push_to_req_url(format!("requests?messages={}{}{}{}{}", c, ns, os, rs, fs))
	}

	pub fn chat_req_string(&self, num: Option<i64>, offset: Option<i64>) -> String {
		let ns = match num {
			Some(val) => format!("={}", val),
			None => String::from("")
		};

		let os = match offset {
			Some(val) => format!("&chats_offset={}", val),
			None => String::from(""),
		};

		self.push_to_req_url(format!("requests?chats{}{}", ns, os))
	}

	pub fn tapback_send_string(
		&self, tapback: i8, tap_guid: String, tap_in_chat: String, remove_tap: Option<bool>
	) -> String {
		let rs = match remove_tap {
			Some(val) => format!("&remove_tap={}", val),
			None => String::from(""),
		};

		if tapback < 0 || tapback > 5 { return String::from(""); }

		self.push_to_req_url(format!("send?tapback={}&tap_guid={}&tap_in_chat={}{}", tapback, tap_guid, tap_in_chat, rs))
	}

	pub fn text_send_string(&self) -> String {
		self.push_to_req_url("send".to_string())
	}

	pub fn search_req_string(
		&self, term: String, case_sensitive: Option<bool>, bridge_gaps: Option<bool>, group_by: Option<String>
	) -> String {
		let cs = match case_sensitive {
			Some(val) => format!("&search_case={}", val),
			None => String::from(""),
		};

		let bg = match bridge_gaps {
			Some(val) => format!("&search_gaps={}", val),
			None => String::from(""),
		};

		let gb = match group_by {
			Some(val) => format!("&search_group={}", val),
			None => String::from(""),
		};

		self.push_to_req_url(format!("requests?search={}{}{}{}", term, cs, bg, gb))
	}

	pub fn attachment_string(&self, path: String) -> String {
		self.push_to_req_url(format!("data?path={}", path))
	}

	pub fn parse_args(&mut self, args: Vec<String>, tui_mode: bool) {
		let mut it = args.iter();

		while let Some(arg) = it.next() {
			match arg.replace("--", "").as_str() {
				"host" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.host = s;
					},
				"fallback_host" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.fallback_host = s;
					}
				"server_port" =>
					if let Some(u) = self.get_u16_from_it(&mut it, arg, tui_mode) {
						self.server_port = u;
					}
				"socket_port" =>
					if let Some(u) = self.get_u16_from_it(&mut it, arg, tui_mode) {
						self.socket_port = u;
					}
				"secure" => self.secure = self.get_bool_from_it(&mut it),
				"notifications" => self.notifications = self.get_bool_from_it(&mut it),
				"password" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.password = s;
					},
				"chat_indicator" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.current_chat_indicator = s;
					},
				"my_chat_end" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.my_chat_end = s;
					},
				"their_chat_end" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.their_chat_end = s;
					},
				"chat_underline" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.chat_underline = s;
					},
				"chats_title" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.chats_title = s;
					},
				"messages_title" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.messages_title = s;
					},
				"input_title" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.input_title = s;
					},
				"help_title" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.help_title = s;
					},
				"to_title" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.to_title = s;
					},
				"compose_title" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.compose_title = s;
					},
				"colorscheme" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.colorscheme = Colorscheme::from(s);
					},
				"poll_exit" =>
					if let Some(u) = self.get_u16_from_it(&mut it, arg, tui_mode) {
						self.poll_exit = u;
					},
				"timeout" =>
					if let Some(u) = self.get_u16_from_it(&mut it, arg, tui_mode) {
						self.timeout = u
					},
				"max_commands" =>
					if let Some(u) = self.get_u16_from_it(&mut it, arg, tui_mode) {
						self.max_past_commands = u
					},
				"help" => self.show_help = self.get_bool_from_it(&mut it) && !tui_mode,
				x => Settings::print_msg(
					format!("Option \x1b[1m{}\x1b[0m not recognized. Ignoring...", x),
					tui_mode
				),
			}
		}

	}

	fn get_u16_from_it(&self, it: &mut std::slice::Iter<String>, key: &str, tui_mode: bool) -> Option<u16> {
		if let Some(to_parse) = it.next() {
			if let Ok(value) = to_parse.parse() {
				Some(value)
			} else {
				let pstr = format!("Please enter an integer value for the key {}", key);
				Settings::print_msg(pstr, tui_mode);
				None
			}
		} else {
			let pstr = format!("Please enter a value for the key {}", key);
			Settings::print_msg(pstr, tui_mode);
			None
		}
	}

	fn get_string_from_it(&self, it: &mut std::slice::Iter<String>, key: &str, tui_mode: bool) -> Option<String> {
		if let Some(value) = it.next() {
			Some(value.to_owned())
		} else {
			let pstr = format!("Please enter a value for the key {}", key);
			Settings::print_msg(pstr, tui_mode);
			None
		}
	}

	fn get_bool_from_it(&self, it: &mut std::slice::Iter<String>) -> bool {
		match it.next() {
			None => true,
			Some(val) => val.parse().unwrap_or(true)
		}
	}

	fn print_msg(msg: String, tui_mode: bool) {
		if tui_mode {
			if let Ok(mut state) = STATE.write() {
				state.hint_msg = msg;
			}
		} else {
			println!("{}", msg)
		}
	}

}
