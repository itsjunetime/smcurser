#![allow(clippy::manual_range_contains)]

use crate::{
	utilities::Utilities,
	colorscheme::*,
};
use std::{
	collections::HashMap,
	fs::read_to_string,
	slice::Iter,
	iter::Peekable,
	any::type_name,
	str::FromStr,
	path::PathBuf
};

macro_rules! pnt{
	($tui:expr, $msg:expr$(, $args:expr)*) => {
		Utilities::print_msg(format!($msg$(, $args)*), $tui)
	}
}

pub fn default_config() -> String {
	let mut conf = config_dir();
	conf.push("smcurser");
	conf.set_extension("toml");

	(*conf.to_string_lossy()).to_string()
}

pub fn default_colorschemes() -> String {
	let mut conf = config_dir();
	conf.push("colorschemes");
	conf.set_extension("toml");

	(*conf.to_string_lossy()).to_string()
}

pub fn config_dir() -> PathBuf {
	let mut conf = dirs::config_dir()
		.unwrap_or_else(|| {
			let mut home = dirs::home_dir()
				.expect("Unable to get your home directory");

			if cfg!(windows) {
				home.push("AppData");
				home.push("Local");
			} else if cfg!(target_os = "macos") {
				home.push("Library");
				home.push("Application Support");
			} else {
				home.push(".config");
			}

			home
		});

	conf.push("smcurser");
	conf
}

pub struct Settings {
	pub rest_host: String,
	pub fallback_host: String,
	pub rest_port: u16,
	pub socket_host: String,
	pub socket_port: u16,
	pub remote_url: Option<String>,
	pub remote_id: Option<String>,
	pub secure: bool,
	pub notifications: bool,
	pub authenticated: bool,
	pub password: String,
	pub current_chat_indicator: char,
	pub unread_chat_indicator: char,
	pub chat_underline: String,
	pub chats_title: String,
	pub messages_title: String,
	pub input_title: String,
	pub help_title: String,
	pub to_title: String,
	pub compose_title: String,
	pub colorscheme: String,
	pub poll_input: u16,
	pub timeout: u16,
	pub show_help: bool,
	pub config_file: String,
	pub colorscheme_file: String,
	pub custom_colorschemes: Option<Vec<Colorscheme>>,
}

impl Settings {
	pub fn default() -> Settings {
		Settings {
			rest_host: "".to_owned(),
			fallback_host: "".to_owned(),
			rest_port: 8741,
			socket_host: "".to_owned(),
			socket_port: 8740,
			remote_url: None,
			remote_id: None,
			secure: true,
			notifications: true,
			authenticated: false,
			password: "toor".to_owned(),
			current_chat_indicator: '>',
			unread_chat_indicator: '•',
			chat_underline: "▔".to_owned(),
			chats_title: "| chats |".to_owned(),
			messages_title: "| messages |".to_owned(),
			input_title: "| input here :) |".to_owned(),
			help_title: "| help |".to_owned(),
			to_title: "| to: |".to_owned(),
			compose_title: "| message: |".to_owned(),
			colorscheme: "forest".to_owned(),
			poll_input: 10,
			timeout: 10,
			show_help: false,
			custom_colorschemes: None,
			config_file: default_config(),
			colorscheme_file: default_colorschemes(),
		}
	}

	pub fn parse_args(
		&mut self, mut args: Vec<String>, tui_mode: bool, parse_config: bool
	) {
		if parse_config {
			let conf_pos = args.iter().position(|a|
				a.as_str() == "--config" || a.as_str() == "-c"
			);

			if let Some(p) = conf_pos {
				if p + 1 < args.len() {
					let _ = args.drain(p..p+1).next();
					let new_conf = args.drain(p..p+1).next();
					if let Some(conf) = new_conf {
						self.config_file = conf;
					}
				}
			}

			self.parse_config_file();

			let color_pos = args.iter().position(|a|
				a.as_str() == "--theme-file" || a.as_str() == "-f"
			);

			if let Some(pos) = color_pos {
				if pos + 1 < args.len() {
					let _ = args.drain(pos..pos+1).next();
					let new_colors = args.drain(pos..pos+1).next();
					if let Some(cls) = new_colors {
						self.colorscheme_file = cls;
					}
				}
			}

			self.parse_custom_colorschemes();
		}

		let mut it = args.iter().peekable();

		macro_rules! set_matches{
			// we have lots of overrides so that we can include however many
			// arguments we want in the macro call
			($arg:ident,$self:ident) => {
				if let Some(val) = self.get_val_from_it(&mut it, $arg, tui_mode) {
					self.$self = val;
				}
			};

			($arg:ident,$self:ident,op) => {
				if let Some(val) = self.get_val_from_it(&mut it, $arg, tui_mode) {
					self.$self = Some(val);
				}
			};

			($arg:ident,$self:ident,flag) => {
				self.$self = self.get_bool_from_it(&mut it, $arg, tui_mode)
			};

			(
				$arg:ident,
				$(($long:expr, $short:expr, $self:ident $(, $op:ident)?)$(,)?)*
			) => {
				match $arg.replace("--", "").as_str() {
					$($long | $short => set_matches!($arg, $self $(, $op)*),)*
					x => pnt!(tui_mode, "Option {} not recognized. ignoring...", x)
				}
			};
		}

		while let Some(arg) = it.next() {
			set_matches!(arg,
				("rest-host", "-u", rest_host),
				("fallback-host", "-b", fallback_host),
				("rest-port", "-p", rest_port),
				("socket-host", "-o", socket_host),
				("socket-port", "-w", socket_port),
				("secure", "-s", secure, flag),
				("notifications", "-n", notifications, flag),
				("password", "-k", password),
				("chat-indicator", "-x", current_chat_indicator),
				("unread-indicator", "-z", unread_chat_indicator),
				("chat-underline", "-d", chat_underline),
				("chat-title", "-a", chats_title),
				("messages-title", "-m", messages_title),
				("input-title", "-y", input_title),
				("help-title", "-e", help_title),
				("to-title", "-q", to_title),
				("compose-title", "-j", compose_title),
				("theme", "-t", colorscheme),
				("poll-input", "-l", poll_input),
				("timeout", "-g", timeout),
				("remote-url", "-r", remote_url, op),
				("remote-id", "-i", remote_id, op),
				("help", "-h", show_help, flag)
			);
		}

		if self.socket_host.is_empty() {
			self.socket_host = self.rest_host.to_owned();
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
				Err(err) => pnt!(false, "Could not parse config file as TOML: {}", err),
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
									pnt!(false, "\x1b[18;1mError:\x1b[0m Your colorscheme {} does not contain the correct \
										number of color specifiers. Please check the documentation", color_spec);

									continue;
								}

								let mut bad_spec = false;
								let mut map: HashMap<String, Vec<u8>> =
									HashMap::new();

								for key in spec.keys() {
									let mut rgb: Vec<u8> = Vec::new();

									if !names.contains(&key.as_str()) {
										pnt!(false, "\x1b[18;1mError:\x1b[0m You have an incorrect specification in '{}': {}", color_spec, key);

										bad_spec = true;
									} else if !spec[key].is_array() {
										pnt!(false, "\x1b[18;1mError:\x1b[0m The color {} in scheme {} is not formatted correctly", key, color_spec);

										bad_spec = true;
									} else if let Some(arr) =  spec[key].as_array() {

										for val in arr {
											if let Some(uint) = val.as_integer() {
												if uint > 255 || uint < 0 {
													pnt!(false, "\x1b[18;1mError\x1b[0m Please keep rgb values between 0 - 255, inclusive");

													bad_spec = true;
													break;
												}

												rgb.push(uint as u8);
											} else {
												pnt!(false, "\x1b[18;1mError:\x1b[0m RGB values must all be UInts, between 0 - 255, inclusive");

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
				Err(err) => pnt!(false, "Could not parse colorschemes file as TOML: {}", err),
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
					pnt!(tui_mode, "Please enter a value of type {:#?} for the key {}", type_name::<T>(), key);
					None
				}
			},
			None => {
				pnt!(tui_mode, "Please enter a value for the key {}", key);
				None
			}
		}
	}

	fn get_bool_from_it(
		&self, it: &mut Peekable<Iter<String>>, key: &str, tui_mode: bool
	) -> bool {
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
			pnt!(true, "set {} to {}", key, b);
		}

		b
	}
}
