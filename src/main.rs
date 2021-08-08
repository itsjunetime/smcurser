mod settings;
mod colorscheme;
mod app;
mod models;
mod chats_view;
mod messages_view;
mod input_view;
mod state;
mod utilities;

use std::{
	sync::{Arc, RwLock},
	io::stdout,
	env::args,
};
use lazy_static::*;
use settings::*;
use app::*;
use state::GlobalState;
use tui::{Terminal, backend::CrosstermBackend};

lazy_static! {
	// set global variables. I know they're theoretically bad practice,
	// but it's just so easy :/
	static ref SETTINGS: Arc<RwLock<Settings>> =
		Arc::new(RwLock::new(Settings::default()));
	static ref STATE: Arc<RwLock<GlobalState>> =
		Arc::new(RwLock::new(GlobalState::new()));
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let mut args = args().collect::<Vec<String>>();
	args.remove(0);

	let mut set = SETTINGS.write().expect("Couldn't open settings");

	set.parse_args(args, false, true);

	// if they want help, just show that and do nothing else.
	if set.show_help {
		for s in CMD_HELP.iter() {
			println!("{}", s);
		}
		return Ok(());
	}

	// if they have specified no host, then exit
	// (since you need a host to communicate with)
	if set.rest_host.is_empty() && set.remote_url.is_none() {
		eprintln!(
			"\x1b[31;1mERROR:\x1b[0m Please enter a host to connect to"
		);
		return Ok(());
	}

	drop(set);

	let stdout = stdout();
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	// go!
	let main_app = MainApp::new().await;
	match main_app {
		Err(err) => {
			eprintln!("Failed to create main app: {}", err);
			Err(err)
		}
		Ok(mut app) => app.main_loop(&mut terminal).await
	}
}

const HELP_MSG: [&str; 33] = [
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
	":y, :Y - ",
	"this copies the text from the currently selected text onto into your clipboard",
];

const CMD_HELP: [&str; 52] = [
	"usage: \x1b[1m./smcurser [flags] [options]\x1b[0m",
	"",
	"\x1b[1mFlags:\x1b[0m",
	"    \x1b[1m--help\x1b[0m                      Show this help menu",
	"    \x1b[1m--secure\x1b[0m                    Connect to REST Host with TLS",
	"    \x1b[1m--notifications\x1b[0m             Show notifications when receiving new messages",
	"",
	"\x1b[1mOptions:\x1b[0m",
	"    \x1b[1m--config\x1b[0m, \x1b[1m-c\x1b[0m <value>            The config file to use",
	"                   Default: $XDG_CONFIG_DIR/smcurser/smcurser.toml",
	"    \x1b[1m--rest-host\x1b[0m, \x1b[1m-u\x1b[0m <value>         The hostname of device which you are trying to connect to",
	"                   Default:",
	"    \x1b[1m--fallback-host\x1b[0m, \x1b[1m-b\x1b[0m <value>     The fallback host to connect to, if the host fails",
	"                   Default:",
	"    \x1b[1m--rest-port\x1b[0m, \x1b[1m-p\x1b[0m <value>         The port on which SMServer is running on the host device",
	"                   Default: \x1b[32;1m8741\x1b[0m",
	"    \x1b[1m--socket-host\x1b[0m, \x1b[1m-o\x1b[0m <value>       The host on which the socket is running (generally not needed)",
	"                   Default: \x1b[32;1mSame as server host\x1b[0m",
	"    \x1b[1m--socket-port\x1b[0m, \x1b[1m-w\x1b[0m <value>       The port on which the SMServer websocket is running on the host device",
	"                   Default: \x1b[32;1m8740\x1b[0m",
	"    \x1b[1m--remote-url\x1b[0m, \x1b[1m-r\x1b[0m <value>        The address of the remote server which is hosting the websocket connections. If this value is specified, SMCurser will attempt to connect only through remote websockets, as opposed to the local REST API",
	"                   Default: \x1b[32;1mNone\x1b[0m",
	"    \x1b[1m--remote-id\x1b[0m, \x1b[1m-i\x1b[0m <value>         The ID of the remote connection",
	"                   Default: \x1b[32;1mNone\x1b[0m",
	"    \x1b[1m--password\x1b[0m, \x1b[1m-k\x1b[0m <value>          The password to try to connect to the host device with",
	"                   Default: \x1b[32;1mtoor\x1b[0m",
	"    \x1b[1m--chat-indicator\x1b[0m, \x1b[1m-x\x1b[0m <value>    The character to use to indicate the currently selected chat",
	"                   Default: \x1b[32;1m>\x1b[0m",
	"    \x1b[1m--unread-indicator\x1b[0m, \x1b[1m-z\x1b[0m <value>  The character to use to indicate all chats with unread messages",
	"                   Default: \x1b[32;1m•\x1b[0m",
	"    \x1b[1m--chat-underline\x1b[0m, \x1b[1m-d\x1b[0m <value>    The character to repeat to create the underline of the text messages",
	"                   Default: \x1b[32;1m▔\x1b[0m",
	"    \x1b[1m--chat-title\x1b[0m, \x1b[1m-a\x1b[0m <value>        The string to use as the title of the chats box",
	"                   Default: \x1b[32;1m| chats |\x1b[0m",
	"    \x1b[1m--messages-title\x1b[0m, \x1b[1m-m\x1b[0m <value>    The string to use as the title of the messages box",
	"                   Default: \x1b[32;1m| messages |\x1b[0m",
	"    \x1b[1m--input-title\x1b[0m, \x1b[1m-y\x1b[0m <value>       The string to use as the title of the input box",
	"                   Default: \x1b[32;1m| input here :) |\x1b[0m",
	"    \x1b[1m--help-title\x1b[0m, \x1b[1m-e\x1b[0m <value>        The string to use as the title of the help box",
	"                   Default: \x1b[32;1m| help |\x1b[0m",
	"    \x1b[1m--to-title\x1b[0m, \x1b[1m-q\x1b[0m <value>          The string to use as the title of the address box in the new composition view",
	"                   Default: | to: |",
	"    \x1b[1m--compose-title\x1b[0m, \x1b[1m-j\x1b[0m <value>     The string to use as the title of the body box in the new composition view",
	"                   Default: \x1b[32;1m| message: |\x1b[0m",
	"    \x1b[1m--theme\x1b[0m, \x1b[1m-t\x1b[0m <value>             The colorscheme to use",
	"                   Default: \x1b[32;1mforest\x1b[0m",
	"    \x1b[1m--poll-input\x1b[0m, \x1b[1m-l\x1b[0m <value>        The amount of milliseconds to poll for input",
	"                   Default: \x1b[32;1m10\x1b[0m",
	"    \x1b[1m--timeout\x1b[0m, \x1b[1m-g\x1b[0m <value>           The timeout for API queries in seconds",
	"                   Default: \x1b[32;1m10\x1b[0m",
	"    \x1b[1m--theme-file\x1b[0m, \x1b[1m-f\x1b[0m <value>  The file that SMCurser should parse to find custom colorschemes",
	"                   Default: \x1b[32;1m$XDG_CONFIG_DIR/smcurser/colorschemes.toml\x1b[0m",
];
