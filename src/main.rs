mod settings;
mod colorscheme;
mod api_client_mod;
mod models;
mod main_app;
mod chats_view;
mod messages_view;
mod input_view;
mod state;
mod utilities;

use std::{
	sync::{Arc, RwLock},
	io::{Error, stdout},
	env::args,
};
use lazy_static::*;
use settings::*;
use api_client_mod::*;
use main_app::*;
use state::GlobalState;
use tui::{Terminal, backend::CrosstermBackend};

lazy_static! {
	// set global variables. I know they're theoretically bad practice, but I've yet to find an
	// easier way of managing global state.
	static ref SETTINGS: Arc<RwLock<Settings>> = Arc::new(RwLock::new(Settings::default()));
	static ref APICLIENT: APIClient = APIClient::new();
	static ref STATE: Arc<RwLock<GlobalState>> = Arc::new(RwLock::new(GlobalState::new()));
}

fn main() -> Result<(), Error> {
	let mut args = args().collect::<Vec<String>>();
	args.remove(0);
	parse_args(args);

	// if they want help, just show that and do nothing else.
	if let Ok(set) = SETTINGS.read() {
		if set.show_help {
			for s in CMD_HELP.iter() {
				println!("{}", s);
			}
			return Ok(());
		}
	}

	// if they have specified no host, then exit (since you need a host to communicate with)
	if let Ok(set) = SETTINGS.read() {
		if set.host.len() == 0 {
			eprintln!("\x1b[31;1mERROR:\x1b[0m Please enter a host to connect to");
			return Ok(());
		}
	}

	let stdout = stdout();
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	// go!
	let mut main_app = MainApp::new();
	main_app.main_loop(&mut terminal)
}

fn parse_args(args: Vec<String>) {
	let mut set = SETTINGS.write().expect("Couldn't open settings to write. Please try again or contact the developer.");
	set.parse_args(args, false, true);
}

const HELP_MSG: [&str; 31] = [
	"COMMANDS:",
	":h, :H -",
	"displays this help message",
	"j, J -",
	"scrolls down in the selected window",
	"k, K -",
	"scrolls up in the selected window",
	"h, l -",
	"switches selected window between messages and conversations",
	":q, :Q, Ctrl+c -",
	"exits SMCurser, cleaning up",
	":c, :C -",
	"this should be immediately followed by a number, specifically the index of the conversation whose texts you want to view. the indices are displayed to the left of each conversation in the leftmost box. eg ':c 25'",
	":s, :S -",
	"sends a text; must be followed by at least one character. Follow the ':s' with a space, and then the body of your text. e.g. ':s hey friend!'",
	":t, :T -",
	"sends a tapback for the currently selected chat. Enter :t <value>, where <value> is either 'love', 'like', 'dislike', 'laugh', 'emphasize', or 'question'.",
	":f, :F -",
	"sends attachments to the specified chat. Specify the files specifically as full path strings, surrounded by single or double quotes, e.g. \"/home/user/Documents/file.txt\" or '/home/user/Pictures/file.jpeg'. You can select multiple files, and they will all be send in the order that they were specified. Also supports tab completion",
	":a, :A -",
	"this, along with the number of the attachment, will open the selected attachment in your browser. For example, if you see 'Attachment 5: image/jpeg', type ':a 5' and the attachment will be opened to be viewed in your browser",
	":b, :B -",
	"these allow you to change variables in settings at runtime. All the available variables to change can be found by passing in the '-h' flag when running SMCurser. To change one, you would simply need to do ':b <var> <val>'. E.G. ':b host 192.168.0.127'. there is no need to encapsulate strings in quotes, and booleans can be typed as either true/false or True/False. If you change something that is displayed on the screen, such as window titles, the windows will not be automatically reloaded.",
	":r, :R -",
	"this reloads the chats, getting current chats from the currently set ip address and port.",
	":n, :N - ",
	"this shows a new composition box, from which you can send a text to a new conversation (or to a conversation that you can\'t quickly access). Type in the recipient(s), then hit enter, and you\'ll be able to enter the body of the message. Once you enter the body, you won\'t be able to change the recipients. Hit ctrl+g to send the text.",
	":dc - ",
	"this deletes the current conversation (if you follow it with the chat_id, e.g. `:dc +11231231234`). If you don't, it will prompt you to do so.",
	":dt - ",
	"this deletes the currently selected text. There is no prompting, it immediately deletes it, so make sure that you are careful with this comand",
];

const CMD_HELP: [&str; 51] = [
	"usage: \x1b[1m./smcurser [options]\x1b[0m",
	"",
	"\x1b[1mOptions:\x1b[0m",
	"    \x1b[1m--help\x1b[0m                    : Show this help menu",
	"                                Default: false",
	"    \x1b[1m--host\x1b[0m <value>            : The hostname of device which you are trying to connect to",
	"                                Default:",
	"    \x1b[1m--fallback_host\x1b[0m <value>   : The fallback host to connect to, if the host fails",
	"                                Default:",
	"    \x1b[1m--server_port\x1b[0m <value>     : The port on which SMServer is running on the host device",
	"                                Default: 8741",
	"    \x1b[1m--socket_port\x1b[0m <value>     : The port on which the SMServer websocket is running on the host device",
	"                                Default: 8740",
	"    \x1b[1m--secure\x1b[0m                  : Toggle connecting to a secure server",
	"                                Default: true",
	"    \x1b[1m--notifications\x1b[0m           : Toggle showing notifications or not",
	"                                Default: true",
	"    \x1b[1m--password\x1b[0m <value>        : The password to try to connect to the host device with",
	"                                Default: toor",
	"    \x1b[1m--chat_indicator\x1b[0m <value>  : The character to use to indicate the currently selected chat",
	"                                Default: >",
	"    \x1b[1m--unread_indicator\x1b[0m <value>: The character to use to indicate all chats with unread messages",
	"                                Default: •",
	"    \x1b[1m--my_chat_end\x1b[0m <value>     : The tail to use on the end of your text messages",
	"                                Default: ⧹▏",
	"    \x1b[1m--their_chat_end\x1b[0m <value>  : The tail to use on the end of their text messages",
	"                                Default: ▕⧸",
	"    \x1b[1m--chat_underline\x1b[0m <value>  : The character to repeat to create the underline of the text messages",
	"                                Default: ▔",
	"    \x1b[1m--chats_title\x1b[0m <value>     : The string to use as the title of the chats box",
	"                                Default: | chats |",
	"    \x1b[1m--messages_title\x1b[0m <value>  : The string to use as the title of the messages box",
	"                                Default: | messages |",
	"    \x1b[1m--input_title\x1b[0m <value>     : The string to use as the title of the input box",
	"                                Default: | input here :) |",
	"    \x1b[1m--help_title\x1b[0m <value>      : The string to use as the title of the help box",
	"                                Default: | help |",
	"    \x1b[1m--to_title\x1b[0m <value>        : The string to use as the title of the address box in the new composition view",
	"                                Default: | to: |",
	"    \x1b[1m--compose_title\x1b[0m <value>   : The string to use as the title of the body box in the new composition view",
	"                                Default: | message : |",
	"    \x1b[1m--colorscheme\x1b[0m <value>     : The colorscheme to use",
	"                                Default: forest",
	"    \x1b[1m--poll_exit\x1b[0m <value>       : The amount of milliseconds to poll for input",
	"                                Default: 10",
	"    \x1b[1m--timeout\x1b[0m <value>         : The timeout for API queries in seconds",
	"                                Default: 10",
	"    \x1b[1m--use_fallback\x1b[0m <value>    : This sets SMCurser to use the fallback host as the main host",
	"                                Default: false",
	"    \x1b[1m--colorscheme_file\x1b[0m <value>: The file that SMCurser should parse to find custom colorschemes",
	"                                Default: $XDG_CONFIG_DIR/smcurser/colorschemes.toml",
];
