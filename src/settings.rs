use crate::{
	utilities::Utilities,
	colorscheme::*,
};
use serde::Deserialize;
use std::{
	collections::HashMap,
	fs::read_to_string,
	slice::Iter,
	iter::Peekable,
	any::type_name,
	str::FromStr,
};


#[derive(Deserialize)]
pub struct Settings {
	pub host: String,
	pub fallback_host: String,
	pub server_port: u16,
	pub socket_host: Option<String>,
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
	pub colorscheme_file: String,
	pub custom_colorschemes: Option<Vec<Colorscheme>>,
}

impl Settings {
	pub fn default() -> Settings {
		let (config_file, colorscheme_file) = {
			let mut config_dir = dirs::config_dir()
				.expect("Cannot detect your system's configuration directory. Please file an issue with the maintainer");

			config_dir.push("smcurser");
			let mut colorscheme_dir = config_dir.clone();

			config_dir.push("smcurser");
			config_dir.set_extension("toml");

			let config = config_dir.into_os_string().into_string().unwrap_or("".to_owned());

			colorscheme_dir.push("colorschemes");
			colorscheme_dir.set_extension("toml");
			let colorschemes = colorscheme_dir.into_os_string().into_string().unwrap_or("".to_owned());
			(config, colorschemes)
		};

		Settings {
			host: "".to_owned(),
			fallback_host: "".to_owned(),
			server_port: 8741,
			socket_host: None,
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
			colorscheme_file: colorscheme_file,
			custom_colorschemes: None,
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
			None => "".to_owned()
		};

		let os = match offset {
			Some(off) => format!("&messages_offset={}", off),
			None => "".to_owned()
		};

		let rs = match read {
			None => "",
			Some(val) => if val { "&read_messages=true" } else { "&read_messages=false" },
		};

		let fs = match from {
			Some(fr) => format!("&messages_from={}", fr),
			None => "".to_owned()
		};

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

	pub fn name_req_string(&self, chat_id: &str) -> String {
		self.push_to_req_url(format!("requests?name={}", chat_id))
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

	pub fn set_fallback_as_host(&mut self) {
		std::mem::swap(&mut self.fallback_host, &mut self.host);
	}

	pub fn parse_args(&mut self, mut args: Vec<String>, tui_mode: bool, parse_config: bool) {
		if parse_config {
			let conf_pos = args.iter().position(|a| a.as_str() == "--config");

			if let Some(p) = conf_pos {
				if p + 1 < args.len() {
					let _ = args.drain(p..p+1).nth(0);
					let new_conf = args.drain(p..p+1).nth(0);
					if let Some(conf) = new_conf {
						self.config_file = conf;
					}
				}
			}

			self.parse_config_file();

			let color_pos = args.iter()
				.position(|a| a.as_str() == "--colorscheme_file");

			if let Some(pos) = color_pos {
				if pos + 1 < args.len() {
					let _ = args.drain(pos..pos+1).nth(0);
					let new_colors = args.drain(pos..pos+1).nth(0);
					if let Some(cls) = new_colors {
						self.colorscheme_file = cls;
					}
				}
			}

			self.parse_custom_colorschemes();
		}

		let mut it = args.iter().peekable();

		while let Some(arg) = it.next() {
			if parse_config && arg.len() > 0 && arg.chars().nth(0).unwrap_or(' ') != '-' {
				println!("Option {} not recognized. Skipping...", arg);
				continue;
			}

			match arg.replace("--", "").as_str() {
				"host" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.host = s;
					},
				"fallback_host" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.fallback_host = s;
					},
				"server_port" =>
					if let Some(u) = self.get_val_from_it::<u16>(&mut it, arg, tui_mode) {
						self.server_port = u;
					},
				"socket_host" =>
					if let Some(u) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.socket_host = Some(u);
					},
				"socket_port" =>
					if let Some(u) = self.get_val_from_it::<u16>(&mut it, arg, tui_mode) {
						self.socket_port = u;
					},
				"secure" =>
					self.secure = self.get_bool_from_it(&mut it, arg, tui_mode),
				"notifications" =>
					self.notifications = self.get_bool_from_it(&mut it, arg, tui_mode),
				"password" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.password = s;
					},
				"chat_indicator" =>
					if let Some(c) = self.get_val_from_it::<char>(&mut it, arg, tui_mode) {
						self.current_chat_indicator = c;
					},
				"unread_indicator" =>
					if let Some(c) = self.get_val_from_it::<char>(&mut it, arg, tui_mode) {
						self.unread_chat_indicator = c;
					},
				"my_chat_end" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.my_chat_end = s;
					},
				"their_chat_end" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.their_chat_end = s;
					},
				"chat_underline" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.chat_underline = s;
					},
				"chats_title" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.chats_title = s;
					},
				"messages_title" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.messages_title = s;
					},
				"input_title" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.input_title = s;
					},
				"help_title" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.help_title = s;
					},
				"to_title" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.to_title = s;
					},
				"compose_title" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.compose_title = s;
					},
				"colorscheme" =>
					if let Some(s) = self.get_val_from_it::<String>(&mut it, arg, tui_mode) {
						self.colorscheme = s;
					},
				"poll_exit" =>
					if let Some(u) = self.get_val_from_it::<u16>(&mut it, arg, tui_mode) {
						self.poll_exit = u;
					},
				"timeout" =>
					if let Some(u) = self.get_val_from_it::<u16>(&mut it, arg, tui_mode) {
						self.timeout = u;
					},
				"use_fallback" =>
					if self.get_bool_from_it(&mut it, arg, tui_mode) {
						self.set_fallback_as_host();
					},
				"help" => self.show_help = self.get_bool_from_it(&mut it, arg, tui_mode) && !tui_mode,
				x => Utilities::print_msg(
					format!("Option \x1b[1m{}\x1b[0m not recognized. Ignoring...", x),
					tui_mode
				),
			}
		}
	}

	pub fn parse_config_file(&mut self) {
		let contents_try = read_to_string(&self.config_file);

		if let Ok(contents) = contents_try {

			let toml_value = contents.parse::<toml::Value>();
			match toml_value {
				Ok(val) => {
					if let Some(table) = val.as_table() {
						let mut parsed = Vec::new();

						for i in table.keys() {
							if let Some(value) = table[i].as_str() {
								parsed.push(i.to_owned());
								parsed.push(value.to_owned());
							}
						}

						self.parse_args(parsed, false, false);
					}
				},
				Err(err) => Utilities::print_msg(
					format!("Could not parse config file as TOML: {}", err),
					false
				),
			}
		}
	}

	pub fn parse_custom_colorschemes(&mut self) {
		let contents_try = read_to_string(&self.colorscheme_file);

		if let Ok(contents) = contents_try {

			let toml_value = contents.parse::<toml::Value>();
			match toml_value {
				Ok(val) => {
					if let Some(arr) = val.as_table() {
						let names = vec![
							"selected_box",
							"unselected_box",
							"my_underline",
							"their_underline",
							"selected_underline",
							"chat_indicator",
							"unread_indicator",
							"text_color",
							"hints_box"
						];

						for color_spec in arr.keys() {
							if let Some(spec) = arr[color_spec].as_table() {

								if spec.keys().len() != names.len() {
									Utilities::print_msg(
										format!("\x1b[18;1mError:\x1b[0m Your colorscheme {} does not contain the correct number \
											of color specifiers. Please check the documentation", color_spec),
										false
									);

									continue;
								}

								let mut bad_spec = false;
								let mut map: HashMap<String, Vec<u8>> = HashMap::new();

								for key in spec.keys() {
									let mut rgb: Vec<u8> = Vec::new();

									if !names.contains(&(*key).as_str()) {
										Utilities::print_msg(
											format!("\x1b[18;1mError:\x1b[0m You have an incorrect specification in '{}': {}", color_spec, key),
											false
										);

										bad_spec = true;
									} else if !spec[key].is_array() {
										Utilities::print_msg(
											format!("\x1b[18;1mError:\x1b[0m The color {} in scheme {} is not formatted correctly", key, color_spec),
											false
										);

										bad_spec = true;
									} else if let Some(arr) =  spec[key].as_array() {

										for val in arr {
											if let Some(uint) = val.as_integer() {
												if uint > 255 || uint < 0 {
													Utilities::print_msg(
														format!("\x1b[18;1mError:\x1b[0m Please keep rgb values between 0 - 255, inclusive."),
														false
													);

													bad_spec = true;
													break;
												}

												rgb.push(uint as u8);
											} else {
												Utilities::print_msg(
													format!("\x1b[18;1mError:\x1b[0m RGB values must all be UInts, between 0 - 255, inclusive"),
													false
												);

												bad_spec = true;
											}
										}
									}

									map.insert(key.to_owned(), rgb);

									if bad_spec { break; }

								}

								if bad_spec { continue; }

								let colorscheme = Colorscheme::from_specs(color_spec.to_owned(), map);

								if self.custom_colorschemes.is_none() {
									self.custom_colorschemes = Some(vec![colorscheme]);
								} else {
									self.custom_colorschemes.as_mut()
										.unwrap()
										.push(colorscheme);
								}
							}
						}
					}
				},
				Err(err) => Utilities::print_msg(
					format!("Could not parse colorschemes files as TOML: {}", err),
					false
				),
			}
		}
	}

	fn get_val_from_it<T: FromStr + 'static>(
		&self, it: &mut Peekable<Iter<String>>, key: &str, tui_mode: bool
	) -> Option<T> {
		match it.peek() {
			Some(to_parse) => match to_parse.parse() {
				Ok(value) => {
					let _ = it.next();
					Some(value)
				},
				Err(_) => {
					let pstr = format!("Please enter a value of type {:#?} for the key {}", type_name::<T>(), key);
					Utilities::print_msg(pstr, tui_mode);
					None
				}
			},
			None => {
				let pstr = format!("Please enter a value for the key {}", key);
				Utilities::print_msg(pstr, tui_mode);
				None
			}
		}
	}

	fn get_bool_from_it(&self, it: &mut Peekable<Iter<String>>, key: &str, tui_mode: bool) -> bool {
		let b = match it.peek() {
			None => true,
			Some(val) => if let Ok(b_val) = val.parse() {
				let _ = it.next();
				b_val
			} else {
				true
			}
		};

		if tui_mode {
			Utilities::print_msg(format!("set {} to {}", key, b), true);
		}

		b
	}

}
