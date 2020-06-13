use irc::client::data::User as IrcUser;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct Channel(Arc<RwLock<ChannelInner>>);

struct ChannelInner {
    pub name: String,
    pub topic: String,
    pub users: Vec<User>,
}

impl Channel {
    pub fn new(name: String) -> Self {
        Self(Arc::new(RwLock::new(ChannelInner {
            name,
            topic: String::new(),
            users: Vec::new(),
        })))
    }

    pub fn name(&self) -> String {
        self.0.read().name.clone()
    }
    pub fn topic(&self) -> String {
        self.0.read().topic.clone()
    }

    pub fn add_user(&self, mut name: String) {
        let mut flags = Vec::new();
        let possible_flags = &['@', '!', '~']; // TODO: Expand this
        while let Some(flag) = name.chars().next().filter(|c| possible_flags.contains(c)) {
            name.remove(0);
            flags.push(flag);
        }

        let mut inner = self.0.write();
        let users = &mut inner.users;

        if users.iter().find(|u| u.name == name).is_none() {
            let user = User { name, flags };

            users.push(user);
        }
    }

    pub fn add_op(&self, name: &str) {
        let mut inner = self.0.write();
        let users = &mut inner.users;

        let user = match users.iter_mut().find(|u| u.name == name) {
            Some(u) => u,
            None => {
                eprintln!("Warning: Adding op to a user that doesn't exist");
                users.push(User {
                    name: name.to_owned(),
                    flags: Vec::new(),
                });
                users.last_mut().unwrap()
            }
        };
        user.flags.push('@');
    }
    pub fn remove_op(&self, name: &str) {
        let mut inner = self.0.write();
        let users = &mut inner.users;

        let user = match users.iter_mut().find(|u| u.name == name) {
            Some(u) => u,
            None => {
                eprintln!("Warning: Removing op from a user that doesn't exist");
                users.push(User {
                    name: name.to_owned(),
                    flags: Vec::new(),
                });
                users.last_mut().unwrap()
            }
        };
        user.flags.retain(|f| *f != '@');
    }
    pub fn rename_user(&self, old_name: &str, new_name: &str) {
        let mut inner = self.0.write();
        let users = &mut inner.users;

        for user in users.iter_mut() {
            if user.name == old_name {
                user.name = new_name.to_owned();
            }
        }
    }
    pub fn remove_user(&self, name: &str) {
        let mut inner = self.0.write();
        let users = &mut inner.users;

        users.retain(|u| u.name != name);
    }

    pub fn set_topic(&self, new_topic: String) {
        let mut inner = self.0.write();
        inner.topic = new_topic;
    }
    pub fn user_is_op(&self, user: &str) -> bool {
        let inner = self.0.read();
        if let Some(user) = inner.users.iter().find(|u| u.name == user) {
            if user.flags.iter().any(|f| *f == '@') {
                return true;
            }
            false
        } else {
            eprintln!("Tried to look up a user but it could not be found");
            eprintln!("Channel {:?} - user {:?}", inner.name, user);
            eprintln!("All users: {:?}", inner.users);
            false
        }
    }
}

#[derive(Clone, Debug)]
pub struct User {
    pub name: String,
    pub flags: Vec<char>,
}

impl<'a> From<&'a IrcUser> for User {
    fn from(u: &'a IrcUser) -> User {
        panic!("{:?}", u);
    }
}
