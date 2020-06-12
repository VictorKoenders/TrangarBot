use crate::data::Data;
use chrono::{DateTime, Duration, Utc};
use irc::client::ext::ClientExt;
use parking_lot::RwLock;
use std::fmt::Write as _;
use std::io::Write as _;

#[derive(Serialize, Deserialize)]
pub struct Command {
    trigger: String,
    response: Vec<String>,
    #[serde(skip)]
    last_use: Option<DateTime<Utc>>,
}

lazy_static::lazy_static! {
    static ref COMMANDS: RwLock<Vec<Command>> = RwLock::new(Vec::new());
    static ref LAST_HELP_COMMAND: RwLock<DateTime<Utc>> = RwLock::new(chrono::MIN_DATE.and_hms(0, 0, 0));
    static ref COMMAND_TIMEOUT: Duration = Duration::minutes(1);
}

pub fn start() {
    if let Ok(file) = std::fs::File::open("commands.json") {
        let commands: Vec<Command> =
            serde_json::from_reader(file).expect("Could not load commands.json");
        *COMMANDS.write() = commands;
    }
}

pub fn on_message(data: &Data, sender: &str, target: &str, message: &str) {
    if message.trim() == "!help" {
        if *LAST_HELP_COMMAND.read() < Utc::now() - *COMMAND_TIMEOUT {
            let mut message = String::from("Commands: ");
            for (index, command) in COMMANDS.read().iter().enumerate() {
                if index > 0 {
                    message += ", ";
                }
                write!(&mut message, "!{}", command.trigger).expect("Could not create help text");
            }
            write!(&mut message, " (All commands have a 1 minute cooldown)")
                .expect("Could not create help text");
            if let Err(e) = data.client.send_privmsg(target, message) {
                eprintln!("Could not send help command");
                eprintln!("{:?}", e);
            }
            *LAST_HELP_COMMAND.write() = Utc::now();
        } else {
            println!("Cooldown :(");
        }
        return;
    }

    if message.trim().starts_with("!learn") && data.user_is_op(target, sender) {
        let remaining = &message.trim()["!learn".len()..];
        let mut split = remaining.split('=');
        if let Some(left_hand) = split.next() {
            let left_hand = left_hand.trim();

            let right_hand = split
                .fold(String::new(), |s, item| {
                    format!("{}{}{}", s, if s.is_empty() { "" } else { " " }, item)
                })
                .trim()
                .to_owned();
            if !right_hand.is_empty() {
                let mut commands = COMMANDS.write();
                commands.retain(|c| &c.trigger[1..] != left_hand);
                commands.push(Command {
                    trigger: left_hand.to_owned(),
                    response: vec![right_hand],
                    last_use: None,
                });
                let mut f = match std::fs::File::create("commands.json") {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Could not open commands.json for writing");
                        eprintln!("{:?}", e);
                        return;
                    }
                };
                let json = match serde_json::to_string_pretty(&*commands) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("Could not serialize commands");
                        eprintln!("{:?}", e);
                        return;
                    }
                };
                if let Err(e) = f.write_all(json.as_bytes()) {
                    eprintln!("Could not write commands json to file");
                    eprintln!("{:?}", e);
                }
            }
        }
    }
    if message.starts_with('!') {
        let message = &message[1..];
        for command in COMMANDS.write().iter_mut() {
            if message.starts_with(&command.trigger)
                && (command.last_use < Some(Utc::now() - *COMMAND_TIMEOUT))
            {
                command.last_use = Some(Utc::now());
                for response in &command.response {
                    if let Err(e) = data.client.send_privmsg(target, response) {
                        eprintln!("Could not send response to custom command");
                        eprintln!("{:?}", e);
                    }
                }
                return;
            }
        }
    }
}
