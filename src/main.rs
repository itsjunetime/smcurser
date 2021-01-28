mod settings;
mod colorscheme;
mod api_client_mod;
mod models;
mod main_app;
mod chats_view;
mod messages_view;
mod state;

use std::sync::{Arc, RwLock};
use lazy_static::*;
use settings::*;
use api_client_mod::*;
use main_app::*;
use state::GlobalState;
use tui::{Terminal, backend::CrosstermBackend};
use std::io;

lazy_static! {
	static ref SETTINGS: Arc<RwLock<Settings>> = Arc::new(RwLock::new(Settings::default()));
	static ref APICLIENT: Arc<RwLock<APIClient>> = Arc::new(RwLock::new(APIClient::new()));
	static ref STATE: Arc<RwLock<GlobalState>> = Arc::new(RwLock::new(GlobalState::new()));
}

fn main() -> Result<(), io::Error> {
	// clears screen
	print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

	let mut args = std::env::args().collect::<Vec<String>>();
	args.remove(0);
	parse_args(args);

	let stdout = io::stdout();
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	let mut main_app = MainApp::new();
	//let res = main_app.draw(&mut terminal);
	main_app.main_loop(&mut terminal)
}

fn parse_args(args: Vec<String>) {
	let mut set = SETTINGS.write().expect("Couldn't open settings to write. Please try again or contact the developer.");
	set.parse_args(args, false);
}

const HELP_MSG: [&str; 28] = ["COMMANDS:",
":h, :H, help -",
"displays this help message",
"j, J -",
"scrolls down in the selected window",
"k, K -",
"scrolls up in the selected window",
"h, l -",
"switches selected window between messages and conversations",
":q, exit, quit -",
"exits the window, cleaning up. recommended over ctrl+c.",
":c, :C, chat -",
"this should be immediately followed by a number, specifically the index of the conversation whose texts you want to view. the indices are displayed to the left of each conversation in the leftmost box. eg ':c 25'",
":s, :S, send -",
"starts the process for sending a text with the currently selected conversation. after you hit enter on ':s', You will then be able to input the content of your text, and hit <enter> once you are ready to send it, or hit <esc> to cancel. You can also enter enter your text with a space between it and the ':s', e.g.: ':s hello!'",
":f, :F, file -",
"sends attachments to the specified chat. Specify the files specifically as full path strings, surrounded by single or double quotes, e.g. \"/home/user/Documents/file.txt\" or '/home/user/Pictures/file.jpeg'. You can select multiple files, and they will all be send in the order that they were specified.",
":a, :A -",
"this, along with the number of the attachment, will open the selected attachment in your browser. For example, if you see 'Attachment 5: image/jpeg', type ':a 5' and the attachment will be opened to be viewed in your browser",
":b, :B, bind -",
"these allow you to change variables in settings at runtime. all available variables are displayed within lines 11 - 32 in main.py. To change one, you would simply need to do ':b <var> <val>'. E.G. ':b ip 192.168.0.127'. there is no need to encapsulate strings in quotes, and booleans can be typed as either true/false or True/False. If you change something that is displayed on the screen, such as window titles, the windows will not be automatically reloaded.",
":d, :D, display -",
"this allows you view the value of any variable in settings at runtime. just type ':d <var>', and it will display its current value. E.G. ':d ping_interval'",
":r, :R, reload -",
"this reloads the chats, getting current chats from the currently set ip address and port.",
":n, :N, new - ",
"this shows a new composition box, from which you can send a text to a new conversation (or to a conversation that you can\'t quickly access. Type in the recipient(s), then hit enter, and you\'ll be able to enter the body of the message. Once you enter the body, you won\'t be able to change the recipients. Hit ctrl+g to send the text.",
"if characters are not appearing, or the program is acting weird, just type a bunch of random characters and hit enter. No harm will be done for a bad command. For more information, visit: https://github.com/iandwelker/smserver_receiver"];
