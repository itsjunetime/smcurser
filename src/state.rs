use crate::models::*;

pub struct GlobalState {
	pub new_text: Option<Message>,
	pub current_chat: Option<String>,
	pub hint_msg: String,
	pub awaiting_new_convo: bool,
	pub outgoing_websocket_msg: Option<String>,
}

impl GlobalState {
	pub fn new() -> GlobalState {
		GlobalState {
			new_text: None,
			current_chat: None,
			hint_msg: "type :h to get help :)".to_string(),
			awaiting_new_convo: false,
			outgoing_websocket_msg: None,
		}
	}

	pub fn set_typing_in_current(&mut self) {
		if let Some(chat) = self.current_chat {
			self.outgoing_websocket_msg = format!("typing:{}", chat);
		}
	}

	pub fn set_idle_in_current(&mut self) {
		if let Some(chat) = self.current_chat {
			self.outgoing_websocket_msg = format!("idle:{}", chat);
		}
	}
}
