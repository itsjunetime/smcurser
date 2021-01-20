struct WSClient {
	out: ws::Sender,
}

impl ws::Handler for Client {
	fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
		self.out.close(ws::CloseCode::Normal)
	}
}
