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
		let tls = native_tls::TlsConnector::builder()
			.use_sni(false)
			.danger_accept_invalid_certs(true)
			.danger_accept_invalid_hostnames(true)
			.build()
			.unwrap();

		let client = reqwest::blocking::Client::builder()
			.use_preconfigured_tls(tls)
			.build()
			.unwrap();

		APIClient { client }
	}

	pub fn get_url_string(&self, url: &str) -> Result<String, reqwest::Error> {
		let response = self.client.get(url).send()?;

		Ok(response.text().unwrap())
	}

	pub fn authenticate(&self) -> bool {
		let url = SETTINGS.read().unwrap().pass_req_string(None);
		let res = self.get_url_string(&url);

		let got_auth =  match res {
			Err(err) => {
				println!("err: {}", err);
				false
			},
			Ok(val) => val.to_string().parse().unwrap_or_else(|_| false),
		};

		if got_auth {
			if let Ok(mut set) = SETTINGS.write() {
				set.authenticated = true;
			}
		}

		got_auth
	}

	pub fn check_auth(&self) -> bool {
		if !SETTINGS.read().unwrap().authenticated {
			self.authenticate()
		} else {
			true
		}
	}

	pub fn get_chats(&self, num: Option<i64>, offset: Option<i64>) -> Vec<Conversation> {
		if !self.check_auth() { return Vec::new(); }

		let req_str = SETTINGS.read().unwrap().chat_req_string(num, offset);

		let response = self.get_url_string(&req_str).unwrap();

		let json: serde_json::Value = serde_json::from_str(&response).expect("Bad JSON :(");
		let mut ret_vec = Vec::new();

		let obj = json.as_object().unwrap();
		let chats = &obj["chats"];
		let json_vec = chats.as_array().unwrap();
		for value in json_vec {
			let val = value.as_object().unwrap();
			ret_vec.push(Conversation::from_json(val));
		};

		ret_vec
	}

	pub fn get_texts(
		&self, chat: String, num: Option<i64>, offset: Option<i64>, read: Option<bool>, from: Option<i8>
	) -> Vec<Message> {
		if !self.check_auth() { return Vec::new(); }

		let req_str = SETTINGS.read().unwrap().msg_req_string(chat, num, offset, read, from);

		let response = self.get_url_string(&req_str);
		let json: serde_json::Value = serde_json::from_str(&(response.unwrap())).expect("Bad Texts JSON :(");
		let mut ret_vec = Vec::new();

		let object = json.as_object().unwrap();
		let texts = &object["texts"];
		let json_vec = texts.as_array().unwrap();
		for value in json_vec {
			let val = value.as_object().unwrap();
			ret_vec.push(Message::from_json(val));
		}

		ret_vec
	}

	pub fn send_text(
		&self, body: Option<String>, subject: Option<String>, chat_id: String, files: Option<Vec<String>>, photos: Option<String>
	) -> bool {
		if !self.check_auth() { return false; }

		let req_str = SETTINGS.read().unwrap().text_send_string();
		let mut unfound_files = Vec::new();

		let form: reqwest::blocking::multipart::Form =
			if let Some(fil) = files {
				fil.iter().fold(
					reqwest::blocking::multipart::Form::new(),
					| fold_form, file | {
						if Path::new(file).exists() {
							fold_form.file("attachments", file).unwrap()
						} else {
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
