use irc::client::Sender;
use parking_lot::RwLock;
use std::sync::Arc;

mod channel;

pub use self::channel::{Channel, User};
use crate::{Config, ConfigServer};

pub struct Client {
    pub running: bool,
    pub sender: Sender,
    pub config: Arc<Config>,
    server_host: String,
    pub channels: Vec<Arc<RwLock<Channel>>>,
}

impl Client {
    pub fn new(config: Arc<Config>, server_host: String, sender: Sender) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            running: true,
            sender,
            config,
            server_host,
            channels: Vec::new(),
        }))
    }

    pub fn server_config(&self) -> &ConfigServer {
        self.config
            .servers
            .iter()
            .find(|s| s.host == self.server_host)
            .unwrap()
    }

    pub fn find_or_create_channel(&mut self, name: String) -> Arc<RwLock<Channel>> {
        if let Some(channel) = self.find_channel(&name) {
            channel
        } else {
            let channel = Arc::new(RwLock::new(Channel::new(name)));
            self.channels.push(Arc::clone(&channel));
            channel
        }
    }

    pub fn find_channel(&self, name: &str) -> Option<Arc<RwLock<Channel>>> {
        for channel in &self.channels {
            let channel_name = channel.read().name.to_owned();
            if channel_name == name {
                return Some(Arc::clone(channel));
            }
        }
        None
    }
}

pub struct Message<'a> {
    pub client: &'a Arc<RwLock<Client>>,
    pub server_config: &'a ConfigServer,
    pub config: &'a Config,
    pub channel: Option<Arc<RwLock<Channel>>>,

    pub reply_to: &'a str,

    pub sender: &'a str,
    pub body: &'a str,
}

impl Message<'_> {
    pub fn reply(&self, text: &str) {
        if let Err(e) = self.client.read().sender.send_privmsg(self.reply_to, text) {
            eprintln!(
                "Could not send a message to {} on server {}",
                self.sender, self.server_config.host
            );
            eprintln!("{:?}", e);
        }
    }
}
