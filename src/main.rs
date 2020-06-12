#[macro_use]
extern crate serde_derive;

mod actions;
mod data;

use futures::StreamExt;
use irc::client::{data::Config as IrcConfig, prelude::*};
use parking_lot::RwLock;
use serde_derive::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc, time::Duration};

#[tokio::main]
async fn main() {
    let config = Arc::new(Config::from_file("config.json"));

    for i in 0..config.servers.len() {
        let config = Arc::clone(&config);
        tokio::spawn(async move {
            let config = config;
            let server = &config.servers[i];
            loop {
                if let Err(e) = run_client(Arc::clone(&config), server).await {
                    eprintln!("Client {} crashed: {:?}", server.host, e);
                    tokio::time::delay_for(Duration::from_secs(5)).await;
                }
            }
        });
    }
    loop {
        tokio::time::delay_for(Duration::from_secs(60)).await;
    }
}

async fn run_client(
    config: Arc<Config>,
    server_config: &ConfigServer,
) -> Result<Infallible, String> {
    println!("Connecting to {}", server_config.host);
    let irc_client = Client::from_config(IrcConfig {
        server: Some(server_config.host.to_owned()),
        nickname: Some(String::from("TrangarBot")),
        channels: server_config.channels.to_owned(),
        port: Some(6697),
        use_tls: Some(true),
        ping_timeout: Some(60),
        ping_time: Some(60),
        ..Default::default()
    })
    .await
    .map_err(|e| e.to_string())?;

    irc_client.identify().map_err(|e| e.to_string())?;

    let client = data::Client::new(
        Arc::clone(&config),
        server_config.host.to_owned(),
        irc_client.sender(),
    );

    if let Err(e) = run_client_inner(config, server_config, irc_client, Arc::clone(&client)).await {
        client.write().running = false;
        return Err(e);
    } else {
        unreachable!()
    }
}

async fn run_client_inner(
    config: Arc<Config>,
    server_config: &ConfigServer,
    mut irc_client: Client,
    client: Arc<RwLock<data::Client>>,
) -> Result<Infallible, String> {
    actions::on_start(Arc::clone(&client)).await?;

    let mut stream = irc_client.stream().unwrap();
    loop {
        let msg = match stream.next().await {
            Some(Ok(msg)) => msg,
            Some(Err(e)) => {
                return Err(format!("Error receiving message: {:?}", e));
            }
            None => {
                return Err(String::from("IRC stream closed"));
            }
        };

        match (&msg.prefix, &msg.command) {
            (
                Some(Prefix::Nickname(nickname, username, hostname)),
                Command::PRIVMSG(channel_name, body),
            ) => {
                let channel = client.read().find_channel(channel_name);
                let message = data::Message {
                    config: &config,
                    server_config,
                    client: &client,
                    body: &body,
                    reply_to: channel_name,
                    channel,
                    sender: nickname,
                };
                if let Err(e) = actions::on_message(&message).await {
                    eprintln!("Could not execute action");
                    eprintln!("Server: {:?}", server_config.host);
                    eprintln!(
                        "PRIVMSG {} {} {} {}: {}",
                        nickname, username, hostname, channel_name, body
                    );
                    eprintln!("{:?}", e);
                }
            }
            (_, Command::Response(Response::RPL_ENDOFMOTD, _)) => {
                if let Some(password) = server_config.password.as_ref() {
                    if let Err(e) = client
                        .read()
                        .sender
                        .send_privmsg("NickServ", format!("identify TrangarBot {}", password))
                    {
                        eprintln!("Could not identify: {:?}", e);
                    }
                }
            }
            (_, Command::TOPIC(channel, Some(topic))) => {
                let mut client = client.write();
                let channel = client.find_or_create_channel(channel.to_owned());
                let mut channel = channel.write();
                channel.set_topic(topic.to_owned());
            }
            (_, Command::Response(Response::RPL_TOPIC, args)) => {
                if let (Some(channel), Some(new_topic)) = (args.get(1), args.get(2)) {
                    if let Some(channel) = client.read().find_channel(channel) {
                        channel.write().set_topic(new_topic.to_owned());
                    }
                }
            }
            (Some(Prefix::Nickname(nickname, _, _)), Command::PART(channel, _)) => {
                if let Some(channel) = client.read().find_channel(&channel) {
                    channel.write().remove_user(&nickname);
                }
            }
            (Some(Prefix::Nickname(nickname, _, _)), Command::JOIN(channel, None, None)) => {
                client
                    .write()
                    .find_or_create_channel(channel.to_owned())
                    .write()
                    .add_user(nickname.to_owned());
            }
            (Some(Prefix::Nickname(nickname, _, _)), Command::NICK(new_nickname)) => {
                let client = client.read();
                for channel in &client.channels {
                    channel.write().rename_user(nickname, new_nickname);
                }
            }
            (_, Command::ChannelMODE(channel, operations)) => {
                let channel = client.write().find_or_create_channel(channel.to_owned());
                let mut channel = channel.write();
                for operation in operations {
                    match operation {
                        Mode::Plus(ChannelMode::Oper, Some(nick)) => channel.add_op(nick),
                        Mode::Minus(ChannelMode::Oper, Some(nick)) => channel.remove_op(nick),
                        _ => {}
                    }
                }
            }
            (_, Command::Response(Response::RPL_NAMREPLY, args)) => {
                if let (Some(channel_name), Some(names)) = (args.get(2), args.get(3)) {
                    let channel = client
                        .write()
                        .find_or_create_channel(channel_name.to_owned());
                    let mut channel = channel.write();
                    for name in names.split(' ') {
                        channel.add_user(name.to_owned());
                    }
                }
            }
            (_, Command::Response(Response::RPL_MOTD, _)) => {}
            (_, Command::PONG(_, _)) => {}
            (_, cmd) => {
                println!("{:?}", cmd);
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
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
    pub factorio_channel: Option<String>,
    pub password: Option<String>,
}
