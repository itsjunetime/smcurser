use multipart::client::lazy::Multipart;
use std::path::PathBuf;
use std::sync::Arc;
use std::vec::Vec;
use std::result::Result;
use rustls::*;
use super::*;
use models::*;

pub struct APIClient {
	agent: ureq::Agent,
}

impl APIClient {
	pub fn new() -> APIClient {
		let cert_ver_arc = Arc::new(SMServerCertVerifier{});

		let mut config = ClientConfig::new();
		config.dangerous().set_certificate_verifier(cert_ver_arc);

		let agent = ureq::builder().tls_config(Arc::new(config.clone())).build();

		APIClient { agent }
	}

	pub fn get_url_string(&self, url: &str) -> Result<String, ureq::Error> {
		let response = self.agent.get(url).call();

		match response {
			Err(err) => Err(err),
			Ok(res) => Ok(res.into_string().unwrap()),
		}
	}

	pub fn authenticate(&self) -> bool {
		let url = SETTINGS.read().unwrap().pass_req_string(None);
		let res = self.get_url_string(&url);

		let got_auth =  match res {
			Err(_) => false,
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
		if !SETTINGS.read().unwrap().authenticated { self.authenticate() } else { true }
	}

	pub fn get_chats(&self, num: Option<i64>, offset: Option<i64>) -> Vec<Conversation> {
		if !self.check_auth() { return Vec::new(); }

		let req_str = SETTINGS.read().unwrap().chat_req_string(num, offset);

		let response = self.get_url_string(&req_str);
		let json: serde_json::Value = serde_json::from_str(&(response.unwrap())).expect("Bad JSON :(");
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

		/*
		// credit for basically this whole thing goes to https://gist.github.com/jhwgh1968/3cd073677f74506474bbcbcadeffb0ee
		let mut vals = Multipart::new();

		vals.add_text("chat", chat_id);

		if let Some(fil) = files {
			for f in fil.iter() {
				let path_buf = PathBuf::from(f);
				let file = std::fs::File::open(&path_buf).expect("");
				let extension = path_buf.extension().and_then(|s| s.to_str()).unwrap_or("");
				let mime = mime_guess::from_ext(&extension).first_or_octet_stream();
				vals.add_stream("attachments", file, Some("blob"), Some(mime));
			}
		}

		if let Some(subj) = subject {
			vals.add_text("subject", subj);
		}
		
		if let Some(ph) = photos {
			vals.add_text("photos", ph);
		}

		if let Some(bod) = body {
			vals.add_text("text", bod);
		}

		let stream = vals.prepare().unwrap();

		let response = self.agent.post(&req_str)
			.set(
				"Content-Type",
				&format!("multipart/form-data; boundary={}", stream.boundary()),
			)
			.send(stream);
		*/

		let vals = &[
			("chat", chat_id.as_str()),
			("text", &body.unwrap_or_default()),
			("subject", &subject.unwrap_or_default()),
			("photos", &photos.unwrap_or_default()),
		]; // works but doesn't have support for sending files. I'll fix that...

		let response = self.agent.post(&req_str).send_form(vals);

		!response.is_err()
	}
}

pub struct SMServerCertVerifier {}

impl ServerCertVerifier for SMServerCertVerifier {
	fn verify_server_cert(
		&self,
		_roots: &RootCertStore,
		_presented_certs: &[Certificate],
		_dns_name: webpki::DNSNameRef<'_>,
		_oscp_response: &[u8]
	) -> Result<ServerCertVerified, TLSError> {
		Ok(ServerCertVerified::assertion())
	}

	fn verify_tls12_signature(
		&self,
		_message: &[u8],
		_cert: &Certificate,
		_dss: &internal::msgs::handshake::DigitallySignedStruct,
	) -> Result<HandshakeSignatureValid, TLSError> {
		Ok(HandshakeSignatureValid::assertion())
	}

	fn verify_tls13_signature(
		&self,
		_message: &[u8],
		_cert: &Certificate,
		_dss: &internal::msgs::handshake::DigitallySignedStruct,
	) -> Result<HandshakeSignatureValid, TLSError> {
		Ok(HandshakeSignatureValid::assertion())
	}
}
