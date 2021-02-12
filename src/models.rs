#[allow(unused_doc_comments)]
use std::fmt;

pub struct Conversation {
	pub display_name: String,
	pub chat_identifier: String,
	pub latest_text: String,
	pub has_unread: bool,
	pub addresses: String, // Or maybe vec?
	pub is_selected: bool
}

impl Conversation {
	pub fn from_json(val: &serde_json::Map<String, serde_json::Value>) -> Conversation {
		// it's so ugly :(
		Conversation {
			display_name: val["display_name"].as_str().unwrap().to_owned(),
			chat_identifier: val["chat_identifier"].as_str().unwrap().to_owned(),
			latest_text: val["latest_text"].as_str().unwrap().to_owned(),
			has_unread: val["has_unread"].as_bool().unwrap(),
			addresses: val["addresses"].as_str().unwrap().to_owned().to_owned(),
			is_selected: false,
		}
	}
}

impl fmt::Debug for Conversation {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Conversation")
			.field("display_name", &self.display_name)
			.field("chat_identifier", &self.chat_identifier)
			.field("has_unread", &self.has_unread)
			.field("addresses", &self.addresses)
			.field("is_selected", &self.is_selected)
			.finish()
	}
}

pub struct Message {
	pub guid: String,
	pub date_read: i64,
	pub date: i64,
	pub balloon_bundle_id: String,
	pub cache_has_attachments: bool,
	pub attachments: Vec<Attachment>,
	pub imsg: bool,
	pub is_from_me: bool,
	pub subject: String,
	pub text: String,
	pub associated_message_guid: String,
	pub associated_message_type: i16,
	pub sender: Option<String>,
	pub chat_identifier: Option<String>,
	pub message_type: MessageType,
}

impl Message {
	pub fn from_json(val: &serde_json::Map<String, serde_json::Value>) -> Message {
		Message {
			guid: val["guid"].as_str().unwrap().to_owned(),
			date: val["date"].as_i64().unwrap(),
			balloon_bundle_id: val["balloon_bundle_id"].as_str().unwrap().to_owned(),
			cache_has_attachments: val["cache_has_attachments"].as_bool().unwrap(),
			imsg: val["service"].as_str().unwrap() == "iMessage",
			is_from_me: val["is_from_me"].as_bool().unwrap(),
			subject: val["subject"].as_str().unwrap().to_owned(),
			text: val["text"].as_str().unwrap().to_owned(),
			associated_message_guid: val["associated_message_guid"].as_str().unwrap().to_owned(),
			associated_message_type: val["associated_message_type"].as_i64().unwrap() as i16,
			message_type: MessageType::Normal,
			attachments: if val.contains_key("attachments") {
				val["attachments"].as_array()
					.unwrap()
					.iter()
					.map(|a| Attachment::from_json(a.as_object().unwrap()))
					.collect()
			} else {
				Vec::new()
			},
			date_read: if val.contains_key("date_read") {
				val["date_read"].as_i64().unwrap()
			} else {
				0
			},
			sender: if val.contains_key("sender") {
				Some(val["sender"].as_str().unwrap().to_owned())
			} else {
				None
			},
			chat_identifier: if val.contains_key("chat_identifier") {
				Some(val["chat_identifier"].as_str().unwrap().to_owned())
			} else {
				None
			},
		}
	}

	pub fn typing(chat: &str) -> Message {
		Message {
			guid: "".to_owned(),
			date: 0,
			balloon_bundle_id: "".to_owned(),
			cache_has_attachments: false,
			imsg: true,
			is_from_me: false,
			subject: "".to_owned(),
			text: "".to_owned(),
			associated_message_guid: "".to_owned(),
			associated_message_type: 0,
			message_type: MessageType::Typing,
			chat_identifier: Some(chat.to_owned()),
			attachments: Vec::new(),
			date_read: 0,
			sender: None,
		}
	}

	pub fn idle(chat: &str) -> Message {
		Message {
			guid: "".to_owned(),
			date: 0,
			balloon_bundle_id: "".to_owned(),
			cache_has_attachments: false,
			imsg: true,
			is_from_me: false,
			subject: "".to_owned(),
			text: "".to_owned(),
			associated_message_guid: "".to_owned(),
			associated_message_type: 0,
			message_type: MessageType::Idle,
			chat_identifier: Some(chat.to_owned()),
			attachments: Vec::new(),
			date_read: 0,
			sender: None,
		}
	}
}

impl fmt::Debug for Message {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Message")
			.field("guid", &self.guid)
			.field("date_read", &self.date_read)
			.field("date", &self.date)
			.field("balloon_bundle_id", &self.balloon_bundle_id)
			.field("cache_has_attachments", &self.cache_has_attachments)
			.field("imsg", &self.imsg)
			.field("is_from_me", &self.is_from_me)
			.field("subject", &self.subject)
			.field("text", &self.text)
			.field("associated_message_guid", &self.associated_message_guid)
			.field("associated_message_type", &self.associated_message_type)
			.field("sender", &self.sender)
			.finish()
	}
}

pub enum MessageType {
	Normal,
	Typing,
	Idle,
}

pub struct Attachment {
	pub mime_type: String,
	pub path: String,
}

impl Attachment {
	pub fn from_json(val: &serde_json::Map<String, serde_json::Value>) -> Attachment {
		Attachment {
			mime_type: val["mime_type"].as_str().unwrap().to_owned(),
			path: val["filename"].as_str().unwrap().to_owned(),
		}
	}
}

pub struct MessageLine {
	pub text: String,
	pub message_type: MessageLineType,
	pub relative_index: usize,
	pub from_me: bool
}

impl MessageLine {
	pub fn new(text: String, message_type: MessageLineType, relative_index: usize, from_me: bool) -> MessageLine {
		MessageLine {
			text,
			message_type,
			relative_index,
			from_me
		}
	}

	pub fn blank(ri: usize) -> MessageLine {
		MessageLine {
			text: "".to_string(),
			message_type: MessageLineType::Blank,
			relative_index: ri,
			from_me: true, // since it doesn't matter
		}
	}
}

pub enum MessageLineType {
	Blank,
	TimeDisplay,
	Text,
	Sender,
	Underline,
	Typing,
}
