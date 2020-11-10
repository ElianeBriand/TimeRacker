use tokio::net::TcpStream;
use futures::{SinkExt};
use tokio_util::codec::{LinesCodec, Framed};
use tokio::stream::StreamExt;
use timeracker_common::{ResponseToClient, TimeTrackingState, ClientRequest, TimeTrackingImplDetails};
use timeracker_common::ResponseToClient::{State, Bye};
use clap::{Clap, App, AppSettings};
use directories::ProjectDirs;
use serde::{Serialize, Deserialize};
use std::path::{Path};

extern crate ini;
use ini::Ini;
use ini::ini::Error;

#[derive(Clap)]
#[clap(version = "0.1", author = "liothique <liothique@liothique.xyz>")]
struct CliOptions {
    #[clap(short, long)]
    config: Option<String>,
    #[clap(short, long)]
    server: Option<String>,
    #[clap(subcommand)]
    subcmd: Option<SubCommand>,
}

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
struct Options {
    server: String
}

impl Options {
    pub fn new() -> Options {
        Options {
            server: "localhost:45862".to_string()
        }
    }
}

impl ::std::default::Default for Options {
    fn default() -> Self { Self::new() }
}

#[derive(Clap)]
#[derive(Debug)]
enum SubCommand {
    Enable(Enable),
    Disable(Disable),
    Switch(Switch),
    ShowSettings(ShowSettings)
}


#[derive(Clap)]
#[derive(Debug)]
struct Enable {
}

#[derive(Clap)]
#[derive(Debug)]
struct Disable {
}

#[derive(Clap)]
#[derive(Debug)]
struct Switch {
    id: u64
}

#[derive(Clap)]
#[derive(Debug)]
struct ShowSettings {
}



async fn send_bye(lines: &mut Framed<TcpStream, LinesCodec>) {
    if let Err(e) = lines.send("BYE").await {
        println!("[E] Error on sending BYE command; error = {:?}", e);
    }

    if let Some(result) = lines.next().await {
        match result {
            Ok(line) => {
                let response: ResponseToClient = serde_json::from_str(&line).unwrap();
                match response {
                    Bye{} => { }
                    _ => {println!("Unexpect response to BYE command")}
                }
            }
            Err(e) => {
                println!("[E] Error on decoding from socket; error = {:?}", e);
            }
        }
    } else {
        println!("[E] Connection error while sending BYE command");
    }
}

// This returns  either the cli options if it was set,
// or else, the value in the conf file at given section/key if it is found
// or else, the default value
fn cli_then_conf_then_default_value(conf_ini: &Ini,
                                    ini_section: &str,
                                    ini_key: &str,
                                    value_cli: Option<String>,
                                    default_value: String) -> String {
    value_cli.as_ref().unwrap_or( &conf_ini.get_from(Some(ini_section),
                                                 ini_key)
        .unwrap_or(&default_value).to_string()).to_string()
}

fn reconcile_from_config_file(config_file_path: &Path, cli_options:&mut CliOptions, options: &mut Options) {


    let conf_res = Ini::load_from_file(config_file_path);
    let conf = match conf_res {
        Ok(c) => { c},
        Err(e) => {
            match e {
                Error::Io(_ee) => { /* If there is no file, no problem just return empty*/ Ini::new()},
                Error::Parse(ee) => {
                    /* However, if parse error, signals, then exit*/
                    println!("Error while reading config file {} : {}", config_file_path.to_str().unwrap(),  ee);
                    std::process::exit(1);
                }
            }

        }
    };


    options.server = cli_then_conf_then_default_value (&conf,
                                                       "conn",
                                                       "server",
                                                       cli_options.server.clone(),
                                                       options.server.clone());

}

async fn fetch_remote_state(lines: &mut Framed<TcpStream, LinesCodec>) -> TimeTrackingState {

    // Populate a (fake) remote state before anything is fetched
    let mut remote_state = TimeTrackingState {
        last_assigned_topic_id: 1,
        current_topic_id: 0,
        topics_tree: vec![],
        details: TimeTrackingImplDetails::new()
    };

    lines.send("GET_STATE").await.unwrap();


    if let Some(result) = lines.next().await {
        match result {
            Ok(line) => {
                let response: ResponseToClient = serde_json::from_str(&line).unwrap();
                match response {
                    State{value} => {
                        let state: TimeTrackingState = serde_json::from_str(&value).unwrap();
                        remote_state = state;
                    }
                    _ => {println!("Unexpect other (non-state) format")}
                }

            }
            Err(e) => {
                println!("[E] Error on decoding from socket; error = {:?}", e);
            }
        }
    } else {
        println!("[E] Connection error");
    }

    return remote_state;
}

async fn show_state_command(lines: &mut Framed<TcpStream, LinesCodec>) {
    let remote_state = fetch_remote_state(lines).await;

    let curr_topic_id = remote_state.current_topic_id;

    if curr_topic_id == 0 {
        println!("N: Timetracking disabled (\"enable\" to start tracking)");
    }

    println!();
    let curr_level = 0;
    for topic in remote_state.topics_tree.iter() {
        if topic.id == 0  {
            continue;
        }
        let is_current = if topic.id == curr_topic_id {"***"} else {"   "};
        let mut indentation_str = "".to_string();
        for _n in 0..curr_level {
            indentation_str += "  ";
        }
        println!("  {} {} {:>4}    {:<20}        {:>10} s",is_current, indentation_str, topic.id , topic.name, topic.duration );
    }

    println!();
}


async fn switch_topic_to_id(id: u64, lines: &mut Framed<TcpStream, LinesCodec>) -> bool {

    if let Err(e) = lines.send(ClientRequest::SwitchTopic{id:id}.emit()).await {
        println!("[E] Error on sending SWITCH_TOPIC command; error = {:?}", e);
        return false;
    }

    if let Some(result) = lines.next().await {
        match result {
            Ok(line) => {
                let response: ResponseToClient = serde_json::from_str(&line).unwrap();
                match response{
                    ResponseToClient::Success{details: _} => {  return true; }
                    ResponseToClient::Error{error_code: _, msg: _} => {  return false; }
                    _ => {println!("Unexpect response to SWITCH_TOPIC command"); return false;}
                }
            }
            Err(_e) => { panic!("Unexpected error while waiting for SWITCH_TOPIC response");}
        }
    } else {
        return false;
    }


}

async fn switch_topic_command(switch_subarg: Switch, lines: &mut Framed<TcpStream, LinesCodec>) {
    let success = switch_topic_to_id(switch_subarg.id, lines).await;
    if success {
        println!("R: Topic switched");
    } else {
        println!("R: Failed to switch topic");
    }
    show_state_command(lines).await;
}

async fn enable_time_tracking_command(lines: &mut Framed<TcpStream, LinesCodec>) {

    let state = fetch_remote_state(lines).await;
    if state.current_topic_id != 0 {
        println!("R: Timetracking already enabled");
    } else {
        let success = switch_topic_to_id(1, lines).await;

        if !success {
            println!("[E] Unexpected error while using command SWITCH_TOPIC to enable time tracking.");
            return;
        }

        println!("R: Timetracking enabled");
    }

    show_state_command(lines).await;
}

async fn disable_time_tracking_command(lines: &mut Framed<TcpStream, LinesCodec>) {
    let success = switch_topic_to_id(0, lines).await;

    if !success {
        println!("[E] Unexpected error while using command SWITCH_TOPIC to disable time tracking.");
        return;
    }

    show_state_command(lines).await;
}


#[tokio::main]
async fn main() {

    let _res = App::new("timeracker_cli")
        .setting(AppSettings::DisableHelpSubcommand);

    // Options gathered from command line
    let mut cli_opts: CliOptions = CliOptions::parse();
    // Actual options.
    let mut options: Options = Options::new();

    /* Reconciling between, by order of prority: command line opts, config file options, defaults */

    let mut default_config_path = ProjectDirs::from("com",
                                      "liothique.xyz",
                                      "timeracker_cli")
        .expect("Cannot generate configuration storage directory path").config_dir().to_path_buf();


    // For config file path, consider cmd line options then default
    if cli_opts.config.is_some() {
        default_config_path = Path::new(&cli_opts.config.clone().unwrap()).to_path_buf()
    }

    reconcile_from_config_file(&default_config_path, &mut cli_opts, &mut options);


    /* First establish connection to timeracker_core (required for all actions)  */

    options.server = "localhost:45862".to_string();

    println!("\nS: {} ", &options.server);


    let stream_res = TcpStream::connect(&options.server).await;

    let stream = match stream_res {
        Ok(s) => {s},
        Err(e) => {
            println!("Error while connecting to core @ {} : {}", &options.server,  e);
            std::process::exit(1);
        }
    };

    let mut lines = Framed::new(stream, LinesCodec::new());

    match cli_opts.subcmd {
        Some(subcmd) => {
            match subcmd {
                SubCommand::Enable(_subargs) => { enable_time_tracking_command(&mut lines).await},
                SubCommand::Switch(subargs) => {switch_topic_command(subargs, &mut lines).await},
                SubCommand::Disable(_subargs) => { disable_time_tracking_command(&mut lines).await},
                other => { println!("Unexpected subcommand: {:?}", other); }
            }
        },
        None => {show_state_command(&mut lines).await; }
    }


    // Cleanly ending communication by sending BYE
    send_bye(&mut lines).await;

}
