#[macro_use]
extern crate serde_derive;

mod actions;
mod data;

use futures::StreamExt;
use irc::client::{data::Config as IrcConfig, prelude::*};
use serde_derive::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc, time::Duration};

#[tokio::main]
async fn main() {
    let config = Arc::new(Config::from_file("config.json").expect("Could not load config"));

    let tasks = (0..config.servers.len()).map(|server_index| {
        let config = Arc::clone(&config);
        async move {
            let server = &config.servers[server_index];
            loop {
                if let Err(e) = run_client(Arc::clone(&config), server).await {
                    eprintln!("Client {} disconnected: {:?}", server.host, e);
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            }
        }
    });

    futures::future::join_all(tasks).await;
}

async fn run_client(
    config: Arc<Config>,
    server_config: &ConfigServer,
) -> Result<Infallible, String> {
    println!("Connecting to {}", server_config.host);
    let irc_client = Client::from_config(IrcConfig {
        server: Some(server_config.host.clone()),
        nickname: Some(server_config.nickname.clone()),
        channels: server_config.channels.clone(),
        port: Some(6697),
        use_tls: Some(true),
        ping_timeout: Some(60),
        ping_time: Some(10),
        ..Default::default()
    })
    .await
    .map_err(|e| e.to_string())?;

    irc_client.identify().map_err(|e| e.to_string())?;

    let client = data::Client::new(
        Arc::clone(&config),
        server_config.host.clone(),
        irc_client.sender(),
    );

    if let Err(e) = run_client_inner(config, server_config, irc_client, client.clone()).await {
        client.set_running(false);
        Err(e)
    } else {
        unreachable!()
    }
}

async fn run_client_inner(
    config: Arc<Config>,
    server_config: &ConfigServer,
    mut irc_client: Client,
    client: data::Client,
) -> Result<Infallible, String> {
    actions::on_start(client.clone()).await?;

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
                let channel = client.find_channel(channel_name);
                let message = data::Message {
                    config: &config,
                    server_config,
                    client: &client,
                    body,
                    reply_to: if channel.is_some() {
                        channel_name
                    } else {
                        nickname
                    },
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
                    client.send_to_channel("NickServ", format!("identify TrangarBot {}", password))
                }
            }
            (_, Command::TOPIC(channel, Some(topic))) => {
                client
                    .find_or_create_channel(channel.clone())
                    .set_topic(topic.clone());
            }
            (_, Command::Response(Response::RPL_TOPIC, args)) => {
                if let (Some(channel), Some(new_topic)) = (args.get(1), args.get(2)) {
                    if let Some(channel) = client.find_channel(channel) {
                        channel.set_topic(new_topic.clone());
                    }
                }
            }
            (Some(Prefix::Nickname(nickname, _, _)), Command::PART(channel, _)) => {
                if let Some(channel) = client.find_channel(channel) {
                    channel.remove_user(nickname);
                }
            }
            (Some(Prefix::Nickname(nickname, _, _)), Command::JOIN(channel, None, None)) => {
                client
                    .find_or_create_channel(channel.clone())
                    .add_user(nickname.clone());
            }
            (Some(Prefix::Nickname(nickname, _, _)), Command::NICK(new_nickname)) => {
                client.for_each_channel(|channel| channel.rename_user(nickname, new_nickname));
            }
            (Some(Prefix::Nickname(nickname, _, _)), Command::QUIT(_)) => {
                client.for_each_channel(|channel| channel.remove_user(nickname));
            }
            (_, Command::ChannelMODE(channel, operations)) => {
                let channel = client.find_or_create_channel(channel.clone());
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
                    let channel = client.find_or_create_channel(channel_name.clone());
                    for name in names.split(' ') {
                        channel.add_user(name.to_owned());
                    }
                }
            }
            (_, Command::Response(Response::RPL_MOTD, _)) | (_, Command::PONG(_, _)) => {}
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
    pub fn from_file(f: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let fs = std::fs::File::open(f)?;
        serde_json::from_reader(fs).map_err(Into::into)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigServer {
    pub nickname: String,
    pub host: String,
    pub channels: Vec<String>,
    pub factorio_channel: Option<String>,
    pub password: Option<String>,
}
