pub struct MessageLine {
	pub text: String,
	pub message_type: MessageLineType,
	pub relative_index: usize,
	pub from_me: bool,
}

impl MessageLine {
	pub fn new(
		text: String,
		message_type: MessageLineType,
		relative_index: usize,
		from_me: bool,
	) -> MessageLine {
		MessageLine {
			text,
			message_type,
			relative_index,
			from_me,
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
