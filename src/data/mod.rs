use irc::client::Sender;
use parking_lot::RwLock;
use std::sync::Arc;

mod channel;

pub use self::channel::{Channel, User};
use crate::{Config, ConfigServer};

#[derive(Clone)]
pub struct Client(Arc<RwLock<ClientInner>>);

struct ClientInner {
    pub running: bool,
    pub sender: Sender,
    pub config: Arc<Config>,
    server_host: String,
    pub channels: Vec<Channel>,
}

impl Client {
    pub fn new(config: Arc<Config>, server_host: String, sender: Sender) -> Self {
        Self(Arc::new(RwLock::new(ClientInner {
            running: true,
            sender,
            config,
            server_host,
            channels: Vec::new(),
        })))
    }

    pub fn server_config(&self) -> ConfigServer {
        self.0.read().server_config()
    }

    pub fn running(&self) -> bool {
        self.0.read().running
    }

    pub fn set_running(&self, is_running: bool) {
        let mut inner = self.0.write();
        inner.running = is_running;
    }

    pub fn find_or_create_channel(&self, name: String) -> Channel {
        if let Some(channel) = self.find_channel(&name) {
            channel
        } else {
            let mut inner = self.0.write();
            let channel = Channel::new(name);
            inner.channels.push(channel.clone());
            channel
        }
    }

    pub fn find_channel(&self, name: &str) -> Option<Channel> {
        let inner = self.0.read();
        for channel in &inner.channels {
            let channel_name = channel.name();
            if channel_name == name {
                return Some(channel.clone());
            }
        }
        None
    }

    pub fn send_to_channel(&self, channel: &str, message: impl std::fmt::Display) {
        let inner = self.0.read();
        if let Err(e) = inner.sender.send_privmsg(channel, &message) {
            eprintln!("Could not send message to {}", channel);
            eprintln!("Message: {}", message);
            eprintln!("Error: {:?}", e);
            // TODO: shut down client by calling `self.set_running(false);` ?
        }
    }

    pub fn set_channel_topic(&self, channel: &str, topic: impl std::fmt::Display) {
        let inner = self.0.read();
        if let Err(e) = inner.sender.send_topic(channel, &topic) {
            eprintln!("Could not set channel {:?}'s topic", channel);
            eprintln!("Message: {}", topic);
            eprintln!("Error: {:?}", e);
        }
    }

    pub fn for_each_channel(&self, mut cb: impl FnMut(&Channel) -> ()) {
        let inner = self.0.read();
        for channel in &inner.channels {
            cb(channel);
        }
    }
}

impl ClientInner {
    pub fn server_config(&self) -> ConfigServer {
        self.config
            .servers
            .iter()
            .find(|s| s.host == self.server_host)
            .unwrap()
            .clone()
    }
}

pub struct Message<'a> {
    pub client: &'a Client,
    pub server_config: &'a ConfigServer,
    pub config: &'a Config,
    pub channel: Option<Channel>,

    pub reply_to: &'a str,

    pub sender: &'a str,
    pub body: &'a str,
}

impl Message<'_> {
    pub fn reply(&self, text: impl std::fmt::Display) {
        self.client.send_to_channel(self.reply_to, text);
    }
}
