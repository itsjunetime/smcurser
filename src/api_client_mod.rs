use std::{
	vec::Vec,
	result::Result,
	path::Path,
};
use super::*;
use models::*;

pub struct APIClient {
	client: reqwest::blocking::Client,
}

impl APIClient {
	pub fn new() -> APIClient {
		// these specific things are to make sure that the client can connect with SMServer, since
		// it uses a self-signed cert and normally connects with an IP Address, not hostname
		let tls = native_tls::TlsConnector::builder()
			.use_sni(false)
			.danger_accept_invalid_certs(true)
			.danger_accept_invalid_hostnames(true)
			.build()
			.expect("Unable to build TlsConnector");

		let timeout = if let Ok(set) = SETTINGS.read() {
			set.timeout
		} else {
			10
		};

		let client = reqwest::blocking::Client::builder()
			.use_preconfigured_tls(tls)
			.connect_timeout(std::time::Duration::from_secs(timeout as u64))
			.build()
			.expect("Unable to build API Client");

		APIClient { client }
	}

	pub fn get_url_string(&self, url: &str) -> Result<String, reqwest::Error> {
		let response = self.client.get(url).send()?;

		Ok(response.text().unwrap_or("".to_owned()))
	}

	pub fn authenticate(&self) -> Result<bool, reqwest::Error> {
		// authenticate with SMServer so that we can make more requests later without being denied
		let url = SETTINGS.read().expect("Cannot read settings")
			.pass_req_string(None);
		let res = self.get_url_string(&url)?;

		match res.parse().unwrap_or_else(|_| false) {
			true => {
				if let Ok(mut set) = SETTINGS.write() {
					// set this so that we don't manually authenticate before every request to
					// ensure we have access
					set.authenticated = true;
				}
				Ok(true)
			}
			false => Ok(false)
		}
	}

	pub fn check_auth(&self) -> bool {
		if !SETTINGS.read().expect("Cannot read settings").authenticated {
			match self.authenticate() {
				Ok(auth) => auth,
				Err(_) => false,
			}
		} else {
			true
		}
	}

	pub fn get_chats(&self, num: Option<i64>, offset: Option<i64>) -> Vec<Conversation> {
		if !self.check_auth() { return Vec::new(); }

		let req_str = SETTINGS.read().expect("Cannot read settings")
			.chat_req_string(num, offset);

		let response = self.get_url_string(&req_str);

		match response {
			Ok(res) => {
				// if we can't parse the JSON from this, something is majorly off.
				let json: serde_json::Value = serde_json::from_str(&res).expect("Bad JSON :(");

				// so ngl I don't quite understand how ownership works with relation to serde_json so this
				// is kind of a mess. It functions for my purposes tho
				let obj = json.as_object().unwrap();
				let chats = &obj["chats"];
				let json_vec = chats.as_array().unwrap();

				let ret_vec = json_vec.into_iter()
					.map(|c| Conversation::from_json(c.as_object().unwrap()))
					.collect::<Vec<Conversation>>();

				ret_vec
			},
			Err(err) => {
				// just show an error and return nothing if they can't get the chats
				if let Ok(mut state) = STATE.write() {
					state.hint_msg = format!("could not get conversations: {}", err);
				}
				Vec::new()
			}
		}
	}

	pub fn get_texts(
		&self, chat: String, num: Option<i64>, offset: Option<i64>, read: Option<bool>, from: Option<i8>
	) -> Vec<Message> {
		// get the texts for a specific conversation from SMServer
		if !self.check_auth() { return Vec::new(); }

		let req_str = SETTINGS.read().unwrap().msg_req_string(chat, num, offset, read, from);

		let response = self.get_url_string(&req_str);
		match response {
			Ok(res) => {
				let json: serde_json::Value = serde_json::from_str(&res).expect("Bad Texts JSON :(");

				let object = json.as_object().unwrap();
				let texts = &object["texts"];
				let json_vec = texts.as_array().unwrap();

				let ret_vec = json_vec.into_iter()
					.map(|m| Message::from_json(m.as_object().unwrap()))
					.collect::<Vec<Message>>();

				ret_vec
			},
			Err(err) => {
				if let Ok(mut state) = STATE.write() {
					state.hint_msg = format!("could not get messages: {}", err);
				}

				Vec::new()
			}
		}
	}

	pub fn get_name(&self, chat_id: &str) -> String {
		// get the name that corresponds with a specific chat_id from SMServer
		if !self.check_auth() { return "".to_owned(); }

		let req_str = SETTINGS.read().expect("Unable to read settings")
			.name_req_string(chat_id);

		self.get_url_string(&req_str).unwrap_or(chat_id.to_owned())
	}

	pub fn send_text(
		&self, body: Option<String>, subject: Option<String>, chat_id: String, files: Option<Vec<String>>, photos: Option<String>
	) -> bool {
		// send a text through SMServer
		if !self.check_auth() { return false; }

		let req_str = SETTINGS.read().unwrap().text_send_string();
		let mut unfound_files = Vec::new();

		// create the multipart form that is POSTed to SMServer to send the text
		let form: reqwest::blocking::multipart::Form =
			if let Some(fil) = files {
				fil.iter().fold(
					reqwest::blocking::multipart::Form::new(),
					| fold_form, file | {
						if Path::new(file).exists() {
							// if it exists, append it to the form
							fold_form.file("attachments", file).unwrap()
						} else {
							// this array will be used later to notify the user of what files
							// couldn't be found to send.
							unfound_files.push(file.as_str().to_owned());
							fold_form
						}
				})
			} else {
				reqwest::blocking::multipart::Form::new()
			}
			.text("chat", chat_id)
			.text("text", body.unwrap_or("".to_owned()))
			.text("subject", subject.unwrap_or("".to_owned()))
			.text("photos", photos.unwrap_or("".to_owned()));
		// ideally, I wouldn't add the `text`, `subject`, and `photos` fields unless they were
		// Some, but I can't find a way to do that with this API. So SMServer just ignores
		// parameters that are empty.

		// let them know about the files that couldn't be found in the filesystem.
		// maybe I should have a configuration option to not send a text unless every file is found
		if unfound_files.len() > 0 {
			if let Ok(mut state) = STATE.write() {
				state.hint_msg = format!("Could not find the following files to send: {}", unfound_files.join(", "));
			}
		}

		let response = self.client.post(&req_str)
			.multipart(form)
			.send();

		!response.is_err()
	}
}
