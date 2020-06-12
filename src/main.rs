#[macro_use]
extern crate serde_derive;

mod actions;
mod data;

use irc::client::ext::ClientExt;
use irc::client::{Client, IrcClient};
use irc::proto::command::Command;
use irc::proto::response::Response;
use serde_derive::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use futures::{Future, Stream, future};

pub type Result<T> = std::result::Result<T, failure::Error>;

fn main() {
    let config = Config::from_file("config.json");

    let clients = config
        .servers
        .iter()
        .map(|s| {
            let client = IrcClient::from_config(irc::client::data::Config {
                server: Some(s.host.clone()),
                nickname: Some(String::from("TrangarBotTest")),
                channels: Some(s.channels.clone()),
                port: Some(6697),
                use_ssl: Some(true),
                ..Default::default()
            })
            .unwrap();

            client.identify().unwrap();
            data::Data::new(client)
        })
        .collect::<Vec<_>>();
    let running = Arc::new(AtomicBool::new(true));

    for client in clients.iter().cloned() {
        actions::start(client, Arc::clone(&running));
    }
    let mut streams = Vec::new();
    for client in clients {
        let cloned_client = client.clone();
        let config = config.clone();
        let stream = client.client.stream().for_each(move |msg| {
            match msg.command {
                Command::PRIVMSG(target, message) => {
                    if let Err(e) = actions::execute(&cloned_client, &target, &message, &config) {
                        eprintln!("Could not execute action");
                        eprintln!("Server: {:?}", cloned_client.client.config().server);
                        eprintln!("PRIVMSG {} {}: {}", msg.prefix.unwrap(), target, message);
                        eprintln!("{:?}", e);
                    }
                }

                Command::TOPIC(channel, Some(topic)) => {
                    cloned_client.set_topic(channel, topic);
                }
                Command::Response(Response::RPL_TOPIC, args, suffix) => {
                    if let (Some(channel), Some(topic)) = (args.get(1), suffix) {
                        cloned_client.set_topic(channel.clone(), topic);
                    }
                }
                Command::ChannelMODE(channel, operations) => {
                    cloned_client.update_user_modes(&channel, operations);
                }
                Command::Response(Response::RPL_NAMREPLY, args, Some(suffix)) => {
                    if let Some(channel) = args.get(2) {
                        for name in suffix.split(' ') {
                            cloned_client.add_user(channel, name);
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        });
        streams.push(stream);
    }
    let result = future::join_all(streams).wait();
    if let Err(e) = result {
        eprintln!("IRC bot ended with errors: {:?}", e);
    };
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub irc_password: String,
    pub youtube_key: String,
    pub factorio_username: String,
    pub factorio_key: String,
    pub wolframalpha: String,
    pub servers: Vec<ConfigServer>,
}

impl Config {
    pub fn from_file(f: &str) -> Config {
        let fs = std::fs::File::open(f).expect("Could not open config");
        serde_json::from_reader(fs).expect("Could not read config")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigServer {
    pub host: String,
    pub channels: Vec<String>,
}
