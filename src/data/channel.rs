use irc::client::{Client, IrcClient};
use parking_lot::RwLock;
use std::sync::Arc;
use irc::proto::mode::{Mode, ChannelMode};

#[derive(Clone, Debug)]
pub struct Channel {
    pub name: String,
    pub topic: String,
    pub users: Vec<User>,
}

impl Channel {
    pub fn get_or_create_user(&mut self, name: &str) -> &mut User {
        let index = self.users.iter().position(|u| u.name == name).unwrap_or_else(|| {
            self.users.push(User {
                name: name.to_owned(),
                host: String::new(),
                flags: Vec::new()
            });
            self.users.len() - 1
        });
        &mut self.users[index]
    }
}

#[derive(Clone, Debug)]
pub struct User {
    pub name: String,
    pub host: String,
    pub flags: Vec<char>,
}

#[derive(Clone)]
pub struct Data {
    pub client: IrcClient,
    pub channels: Arc<RwLock<Vec<Channel>>>,
}

impl Data {
    pub fn new(client: IrcClient) -> Self {
        let expected_channel_count = client.config().channels.as_ref().map(Vec::len).unwrap_or(0);
        Self {
            client,
            channels: Arc::new(RwLock::new(Vec::with_capacity(expected_channel_count))),
        }
    }

    pub fn get_channel_topic(&self, channel_name: &str) -> Option<Channel> {
        self.channels
            .read()
            .iter()
            .find(|c| c.name == channel_name)
            .cloned()
    }

    pub fn set_topic(&self, channel_name: String, new_topic: String) {
        let mut channels = self.channels.write();

        if let Some(index) = channels.iter().position(|c| c.name == channel_name) {
            channels[index].topic = new_topic;
        } else {
            channels.push(Channel {
                name: channel_name,
                topic: new_topic,
                users: Vec::new(),
            });
        }
    }

    pub fn add_user(&self, channel_name: &str, mut user_name: &str) {
        let mut channels = self.channels.write();
        let index = match channels.iter().position(|c| c.name == channel_name) {
            Some(i) => i,
            None => {
                channels.push(Channel {
                    name: channel_name.to_owned(),
                    topic: String::new(),
                    users: Vec::new(),
                });
                channels.len() - 1
            }
        };
        let channel = &mut channels[index];
        let mut flags = Vec::new();
        let available_flags = ['@', '+'];
        while let Some(flag) = user_name.chars().next() {
            if available_flags.contains(&flag) {
                user_name = &user_name[1..];
                flags.push(flag);
            } else {
                break;
            }
        }

        channel.users.retain(|u| u.name != user_name);
        channel.users.push(User {
            name: user_name.to_owned(),
            host: String::new(),
            flags,
        });
    }

    pub fn update_user_modes(&self, channel_name: &str, operations: Vec<Mode<ChannelMode>>) {
        let mut channels = self.channels.write();
        let index = match channels.iter().position(|c| c.name == channel_name) {
            Some(i) => i,
            None => {
                channels.push(Channel {
                    name: channel_name.to_owned(),
                    topic: String::new(),
                    users: Vec::new(),
                });
                channels.len() - 1
            }
        };
        let channel = &mut channels[index];
        for operation in operations {
            match operation {
                Mode::Plus(ChannelMode::Oper, Some(name)) => {
                    let user = channel.get_or_create_user(&name);
                    if !user.flags.contains(&'@') {
                        user.flags.push('@');
                    }
                },
                Mode::Minus(ChannelMode::Oper, Some(name)) => {
                    let user = channel.get_or_create_user(&name);
                    user.flags.retain(|f| f != &'@');
                }
                _ => {}
            }
        }

        println!("{:?}", channel.users);
    }

    pub fn user_is_op(&self, _channel: &str, _user: &str) -> bool {
        false
    }
}
