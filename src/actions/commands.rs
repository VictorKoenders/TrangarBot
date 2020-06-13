use crate::data::Message;
use parking_lot::RwLock;
use std::{
    fmt::Write as _,
    io::Write as _,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
pub struct Command {
    trigger: String,
    response: Vec<String>,
    #[serde(skip)]
    last_invoke: LastInvoke,
}

const COMMAND_TIMEOUT_SECONDS: u64 = 60;

lazy_static::lazy_static! {
    static ref COMMANDS: RwLock<Vec<Command>> = RwLock::new(Vec::new());
    static ref LAST_HELP_COMMAND: RwLock<LastInvoke> = RwLock::new(LastInvoke::default());
}

pub fn start() {
    if COMMANDS.read().is_empty() {
        if let Ok(file) = std::fs::File::open("commands.json") {
            let commands: Vec<Command> =
                serde_json::from_reader(file).expect("Could not load commands.json");
            *COMMANDS.write() = commands;
        }
    }
}

#[derive(Default)]
struct LastInvoke(Option<Instant>);
impl LastInvoke {
    pub fn can_invoke_again(&self) -> bool {
        let command_timeout = Duration::from_secs(COMMAND_TIMEOUT_SECONDS);

        if let Some(instant) = &self.0 {
            instant.elapsed() > command_timeout
        } else {
            true
        }
    }

    pub fn has_invoked(&mut self) {
        self.0 = Some(Instant::now());
    }
}

pub async fn on_message<'a>(message: &'a Message<'a>) -> Result<(), String> {
    if message.body.trim() == "!help" {
        if LAST_HELP_COMMAND.read().can_invoke_again() {
            let commands = COMMANDS.read();

            let commands = commands
                .iter()
                .map(|c| c.trigger.as_str())
                .collect::<Vec<_>>();

            message.reply(format!(
                "Commands: {} (All commands have a 1 minute cooldown",
                commands.join(", ")
            ));

            LAST_HELP_COMMAND.write().has_invoked();
        }
        return Ok(());
    }

    if message.body.trim().starts_with("!learn")
        && message
            .channel
            .as_ref()
            .map(|c| c.user_is_op(message.sender))
            .unwrap_or(false)
    {
        let remaining = &message.body.trim()["!learn".len()..];
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
                commands.retain(|c| c.trigger != left_hand);
                commands.push(Command {
                    trigger: left_hand.to_owned(),
                    response: vec![right_hand],
                    last_invoke: LastInvoke::default(),
                });
                let mut f = match std::fs::File::create("commands.json") {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Could not open commands.json for writing");
                        eprintln!("{:?}", e);
                        return Ok(());
                    }
                };
                let json = match serde_json::to_string_pretty(&*commands) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("Could not serialize commands");
                        eprintln!("{:?}", e);
                        return Ok(());
                    }
                };
                if let Err(e) = f.write_all(json.as_bytes()) {
                    eprintln!("Could not write commands json to file");
                    eprintln!("{:?}", e);
                }
                message.reply("Command saved");
            }
        }
        return Ok(());
    }
    if message.body.starts_with('!') {
        let text = &message.body[1..];
        for command in COMMANDS.write().iter_mut() {
            if text.starts_with(&command.trigger) && command.last_invoke.can_invoke_again() {
                command.last_invoke.has_invoked();
                for response in &command.response {
                    message.reply(response);
                }
                return Ok(());
            }
        }
    }
    Ok(())
}
