pub struct GlobalState {
	pub new_text: Option<serde_json::Map<String, serde_json::Value>>,
	pub current_chat: Option<String>,
	pub hint_msg: String,
}

impl GlobalState {
	pub fn new() -> GlobalState {
		GlobalState {
			new_text: None,
			current_chat: None,
			hint_msg: "type :h to get help :)".to_string(),
		}
	}
}
