use sdk::models::*;

pub struct GlobalState {
	pub new_text: Option<Message>,
	pub current_chat: Option<String>,
	pub hint_msg: String,
	pub awaiting_new_convo: bool,
	pub outgoing_websocket_msg: Option<String>,
	pub websocket_state: WebSocketState,
}

impl GlobalState {
	pub fn new() -> GlobalState {
		GlobalState {
			new_text: None,
			current_chat: None,
			hint_msg: "type :h to get help :)".to_string(),
			awaiting_new_convo: false,
			outgoing_websocket_msg: None,
			websocket_state: WebSocketState::Disconnected,
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketState {
	Connected,
	//Connecting,
	Disconnected,
}
