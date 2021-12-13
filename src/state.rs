use sdk::models::*;

#[macro_export]
macro_rules! hint{
	($msg:expr$(, $args:expr)*) => {
		if let Ok(mut state) = STATE.write() {
			state.hint_msg = format!($msg $(, $args)*);
		}
	}
}

#[macro_export]
macro_rules! log{
	($msg:expr$(, $args:expr)*) => {
		crate::utilities::Utilities::log(format!($msg$(, $args)*));
	}
}

pub struct GlobalState {
	pub new_text: Option<Message>,
	pub new_chats: Option<anyhow::Result<Vec<Conversation>>>,
	pub new_msgs: Option<anyhow::Result<Vec<Message>>>,
	pub current_chat: Option<String>,
	pub hint_msg: String,
	pub awaiting_new_convo: bool,
	pub outgoing_websocket_msg: Option<String>,
	pub websocket_state: WebSocketState,
	pub battery_status: BatteryStatus,
}

impl GlobalState {
	pub fn new() -> GlobalState {
		GlobalState {
			new_text: None,
			new_chats: None,
			new_msgs: None,
			current_chat: None,
			hint_msg: "type :h to get help :)".to_string(),
			awaiting_new_convo: false,
			outgoing_websocket_msg: None,
			websocket_state: WebSocketState::Disconnected,
			battery_status: BatteryStatus::Dead,
		}
	}

	pub fn battery_string(&self) -> String {
		match self.battery_status {
			BatteryStatus::Full => "100%, full".to_owned(),
			BatteryStatus::Charging(x) => format!("{}%, charging", x),
			BatteryStatus::Unplugged(x) => format!("{}%, unplugged", x),
			BatteryStatus::Dead => "0%, dead".to_owned(),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketState {
	Connected,
	//Connecting,
	Disconnected,
}

pub enum BatteryStatus {
	Full,
	Charging(u8),
	Unplugged(u8),
	Dead,
}
