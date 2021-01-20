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
	pub chat_vertical_offset: u16,
	pub title_offset: u16,
	pub help_inset: u16,
	pub poll_exit: f64,
	pub timeout: u16,
	pub max_past_commands: u16,
	pub debug: bool
}

impl Settings {
	pub fn default() -> Settings {
		Settings {
			host: "192.168.0.180".to_owned(),
			fallback_host: "192.168.0.127".to_owned(),
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
			chat_vertical_offset: 1,
			title_offset: 5,
			help_inset: 5,
			poll_exit: 0.5,
			timeout: 10,
			max_past_commands: 10,
			debug: false,
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

	pub fn parse_args(&mut self, args: Vec<String>) {
				let mut it = args.iter();

		while let Some(arg) = it.next() {
			match arg.replace("--", "").as_str() {
				"host" => self.host = self.get_string_from_it(&mut it, "host"),
				"fallback_host" => self.fallback_host = self.get_string_from_it(&mut it, arg),
				"server_port" => self.server_port = self.get_u16_from_it(&mut it, arg),
				"socket_port" => self.socket_port = self.get_u16_from_it(&mut it, arg),
				"secure" => self.secure = self.get_bool_from_it(&mut it),
				"notifications" => self.notifications = self.get_bool_from_it(&mut it),
				"password" => self.password = self.get_string_from_it(&mut it, "password"),
				"current_chat_indicator" => self.current_chat_indicator = self.get_string_from_it(&mut it, "current_chat_indicator"),
				"my_chat_end" => self.my_chat_end = self.get_string_from_it(&mut it, "my_chat_end"),
				"their_chat_end" => self.their_chat_end = self.get_string_from_it(&mut it, "their_chat_end"),
				"chat_underline" => self.chat_underline = self.get_string_from_it(&mut it, "chat_underline"),
				"chats_title" => self.chats_title = self.get_string_from_it(&mut it, "chats_title"),
				"messages_title" => self.messages_title = self.get_string_from_it(&mut it, "messages_title"),
				"input_title" => self.input_title = self.get_string_from_it(&mut it, "input_title"),
				"help_title" => self.help_title = self.get_string_from_it(&mut it, "help_title"),
				"to_title" => self.to_title = self.get_string_from_it(&mut it, "to_title"),
				"compose_title" => self.compose_title = self.get_string_from_it(&mut it, "compose_title"),
				"colorscheme" => self.colorscheme = Colorscheme::from(self.get_string_from_it(&mut it, "colorscheme")),
				"chat_vertical_offset" => self.chat_vertical_offset = self.get_u16_from_it(&mut it, "chat_vertical_offset"),
				"title_offset" => self.title_offset = self.get_u16_from_it(&mut it, "title_offset"),
				"help_inset" => self.help_inset = self.get_u16_from_it(&mut it, "help_inset"),
				"poll_exit" => {
					self.poll_exit = it.next().expect(format!("Please enter a value for the key {}", arg).as_str())
						.parse().expect(format!("Please enter a float value for the key {}", arg).as_str());
				},
				"timeout" => self.timeout = self.get_u16_from_it(&mut it, "timeout"),
				"max_past_commands" => self.max_past_commands = self.get_u16_from_it(&mut it, "max_past_commands"),
				"debug" => self.debug = self.get_bool_from_it(&mut it),
				x => println!("Option \x1b[1m{}\x1b[0m not recognized. Ignoring...", x),
			}
		}

	}

	fn get_u16_from_it(&self, it: &mut std::slice::Iter<String>, key: &str) -> u16 {
		it.next().expect(format!("Please enter a value for the key {}", key).as_str())
			.parse().expect(format!("Please enter an integer value for the key {}", key).as_str())
	}

	fn get_string_from_it(&self, it: &mut std::slice::Iter<String>, key: &str) -> String {
		it.next()
			.expect(format!("Please enter a value for the key {}", key).as_str())
			.to_owned()
	}

	fn get_bool_from_it(&self, it: &mut std::slice::Iter<String>) -> bool {
		match it.next() {
			None => true,
			Some(val) => val.parse().unwrap_or_else(|_| true)
		}
	}

}
