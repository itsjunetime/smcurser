use chrono::prelude::*;
use notify_rust::Notification;
use std::io::prelude::*;

pub struct Utilities;

impl Utilities {

	pub fn date_pad_string(date: i64, width: usize) -> String {
		let unix_timestamp = (date / 1000000000) + 978307200;
		let naive = NaiveDateTime::from_timestamp(unix_timestamp, 0);
		let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
		let format = datetime.format("%m/%d/%Y %H:%M").to_string();

		let pad = (width - format.len()) / 2;
		format!("{}{}{}", " ".repeat(pad), format, " ".repeat(pad))
	}

	pub fn show_notification(title: &str, msg: &str) {
		let mut image_dir = dirs::config_dir().expect("Could not get configuration dir. Please report this to  the maintainer.");
		image_dir.push("smcurser");
		image_dir.push("icon.png");

		let image_str = format!("file://{}",
			image_dir.into_os_string().into_string().unwrap_or("".to_owned()));

		let _ = Notification::new()
			.appname("SMCurser")
			.summary(title)
			.body(msg)
			.icon(&image_str)
			.show();
	}

	#[allow(dead_code)]
	pub fn log(log_str: &str) {
		let mut file = std::fs::OpenOptions::new()
			.create(true)
			.append(true)
			.open("log.log")
			.expect("Cannot open log file for writing");
			
		let _ = writeln!(file, "{}", log_str);
	}
}
