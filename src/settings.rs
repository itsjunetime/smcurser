use crate::*;
use chrono::prelude::*;
use serde::Deserialize;
use notify_rust::Notification;

#[derive(Deserialize)]
pub struct Settings {
	pub host: String,
	pub fallback_host: String,
	pub server_port: u16,
	pub socket_port: u16,
	pub secure: bool,
	pub notifications: bool,
	pub authenticated: bool,
	pub password: String,
	pub current_chat_indicator: char,
	pub unread_chat_indicator: char,
	pub my_chat_end: String,
	pub their_chat_end: String,
	pub chat_underline: String,
	pub chats_title: String,
	pub messages_title: String,
	pub input_title: String,
	pub help_title: String,
	pub to_title: String,
	pub compose_title: String,
	pub colorscheme: String,
	pub poll_exit: u16,
	pub timeout: u16,
	pub show_help: bool,
	pub config_file: String,
}

impl Settings {
	pub fn default() -> Settings {
		let config_file = {
			let mut config_dir = dirs::config_dir()
				.expect("Cannot detect your system's configuration directory. Please file an issue with the maintainer");

			config_dir.push("smserver");
			config_dir.push("smserver");
			config_dir.set_extension("toml");

			config_dir.into_os_string().into_string().unwrap()
		};

		Settings {
			host: "".to_owned(),
			fallback_host: "".to_owned(),
			server_port: 8741,
			socket_port: 8740,
			secure: true,
			notifications: true,
			authenticated: false,
			password: "toor".to_owned(),
			current_chat_indicator: '>',
			unread_chat_indicator: '•',
			my_chat_end: "⧹▏".to_owned(),
			their_chat_end: "▕⧸".to_owned(),
			chat_underline: "▔".to_owned(),
			chats_title: "| chats |".to_owned(),
			messages_title: "| messages |".to_owned(),
			input_title: "| input here :) |".to_owned(),
			help_title: "| help |".to_owned(),
			to_title: "| to: |".to_owned(),
			compose_title: "| message: |".to_owned(),
			colorscheme: "forest".to_owned(),
			poll_exit: 10,
			timeout: 10,
			show_help: false,
			config_file: config_file,
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

	pub fn show_notification(title: &str, msg: &str) {
		let notify = if let Ok(set) = SETTINGS.read() {
			set.notifications
		} else {
			true
		};

		if notify {
			let mut image_dir = dirs::config_dir().expect("Could not get configuration dir. Please report this to  the maintainer.");
			image_dir.push("smserver");
			image_dir.push("icon.png");

			let image_str = format!("file://{}",
				image_dir.into_os_string().into_string().unwrap_or("".to_owned()));

			let _ = Notification::new()
				.appname("SMServer")
				.summary(title)
				.body(msg)
				.icon(&image_str)
				.icon("file:///Users/ian/Library/Application Support/smserver/icon.png")
				.show();
		}
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
		&self, tapback: i8, tap_guid: &str, remove_tap: Option<bool>
	) -> String {
		let rs = match remove_tap {
			Some(val) => format!("&remove_tap={}", val),
			None => String::from(""),
		};

		if tapback < 0 || tapback > 5 { return String::from(""); }

		self.push_to_req_url(format!("send?tapback={}&tap_guid={}{}", tapback, tap_guid, rs))
	}

	pub fn text_send_string(&self) -> String {
		self.push_to_req_url("send".to_string())
	}

	#[allow(dead_code)]
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

	pub fn delete_chat_string(&self, chat: &str) -> String {
		self.push_to_req_url(format!("send?delete_chat={}", chat))
	}

	pub fn delete_text_string(&self, text: &str) -> String {
		self.push_to_req_url(format!("send?delete_text={}", text))
	}

	pub fn parse_args(&mut self, mut args: Vec<String>, tui_mode: bool) {
		if !tui_mode {
			let pos = args.iter().position(|a| a.as_str() == "--config");

			if let Some(p) = pos {
				if p + 1 < args.len() {
					let _ = args.drain(p..p+1).nth(0);
					let new_conf = args.drain(p..p+1).nth(0);
					if let Some(conf) = new_conf {
						self.config_file = conf;
					}
				}
			}

			self.parse_config_file();
		}

		let mut it = args.iter();

		while let Some(arg) = it.next() {
			if !tui_mode && arg.len() > 0 && arg.chars().nth(0).unwrap() != '-' {
				println!("Option {} not recognized. Skipping...", arg);
				continue;
			}

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
				"secure" =>
					self.secure = self.get_bool_from_it(&mut it, arg, tui_mode),
				"notifications" =>
					self.notifications = self.get_bool_from_it(&mut it, arg, tui_mode),
				"password" =>
					if let Some(s) = self.get_string_from_it(&mut it, arg, tui_mode) {
						self.password = s;
					},
				"chat_indicator" =>
					if let Some(c) = self.get_char_from_it(&mut it, arg, tui_mode) {
						self.current_chat_indicator = c;
					},
				"unread_indicator" =>
					if let Some(c) = self.get_char_from_it(&mut it, arg, tui_mode) {
						self.unread_chat_indicator = c;
					}
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
						self.colorscheme = s;
					},
				"poll_exit" =>
					if let Some(u) = self.get_u16_from_it(&mut it, arg, tui_mode) {
						self.poll_exit = u;
					},
				"timeout" =>
					if let Some(u) = self.get_u16_from_it(&mut it, arg, tui_mode) {
						self.timeout = u;
					},
				"help" => self.show_help = self.get_bool_from_it(&mut it, arg, tui_mode) && !tui_mode,
				x => Settings::print_msg(
					format!("Option \x1b[1m{}\x1b[0m not recognized. Ignoring...", x),
					tui_mode
				),
			}
		}
	}

	pub fn parse_config_file(&mut self) {
		let contents_try = std::fs::read_to_string(&self.config_file);

		if let Ok(contents) = contents_try {
			let sets_try = toml::from_str(&contents);

			if let Ok(sets) = sets_try {
				*self = sets;
			} else if let Err(err) = sets_try {
				Settings::print_msg(
					format!("There is an error with your config file; you may be missing some or all of the required fields: {}", err),
					false
				);
			}
		}
	}

	fn get_u16_from_it(&self, it: &mut std::slice::Iter<String>, key: &str, tui_mode: bool) -> Option<u16> {
		if let Some(to_parse) = it.next() {
			if let Ok(value) = to_parse.parse() {
				if tui_mode {
					Settings::print_msg(format!("set {} to {}", key, value), false);
				}
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
			if tui_mode {
				Settings::print_msg(format!("set {} to {}", key, value), true);
			}
			Some(value.to_owned())
		} else {
			let pstr = format!("Please enter a value for the key {}", key);
			Settings::print_msg(pstr, tui_mode);
			None
		}
	}

	fn get_char_from_it(&self, it: &mut std::slice::Iter<String>, key: &str, tui_mode: bool) -> Option<char> {
		if let Some(value) = it.next() {
			if let Ok(c) = value.parse() {
				if tui_mode {
					Settings::print_msg(format!("set {} to {}", key, c), true);
				}
				Some(c)
			} else {
				let pstr = format!("Please enter a single character for the key {}", key);
				Settings::print_msg(pstr, tui_mode);
				None
			}
		} else {
			let pstr = format!("Please enter a single character for the key {}", key);
			Settings::print_msg(pstr, tui_mode);
			None
		}
	}

	fn get_bool_from_it(&self, it: &mut std::slice::Iter<String>, key: &str, tui_mode: bool) -> bool {
		let b = match it.next() {
			None => true,
			Some(val) => val.parse().unwrap_or(true)
		};

		if tui_mode {
			Settings::print_msg(format!("set {} to {}", key, b), true);
		}

		b
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
