use irc::client::data::User as IrcUser;

#[derive(Clone, Debug)]
pub struct Channel {
    pub name: String,
    pub topic: String,
    pub users: Vec<User>,
}

impl Channel {
    pub fn new(name: String) -> Self {
        Self {
            name,
            topic: String::new(),
            users: Vec::new(),
        }
    }

    pub fn add_user(&mut self, mut name: String) {
        let mut flags = Vec::new();
        let possible_flags = &['@', '!', '~']; // TODO: Expand this
        while let Some(flag) = name.chars().next().filter(|c| possible_flags.contains(c)) {
            name.remove(0);
            flags.push(flag);
        }

        if self.users.iter().find(|u| u.name == name).is_none() {
            let user = User { name, flags };

            self.users.push(user);
        }
    }

    pub fn add_op(&mut self, name: &str) {
        let user = match self.users.iter_mut().find(|u| u.name == name) {
            Some(u) => u,
            None => {
                eprintln!("Warning: Adding op to a user that doesn't exist");
                self.users.push(User {
                    name: name.to_owned(),
                    flags: Vec::new(),
                });
                self.users.last_mut().unwrap()
            }
        };
        user.flags.push('@');
    }
    pub fn remove_op(&mut self, name: &str) {
        let user = match self.users.iter_mut().find(|u| u.name == name) {
            Some(u) => u,
            None => {
                eprintln!("Warning: Removing op from a user that doesn't exist");
                self.users.push(User {
                    name: name.to_owned(),
                    flags: Vec::new(),
                });
                self.users.last_mut().unwrap()
            }
        };
        user.flags.retain(|f| *f != '@');
    }
    pub fn rename_user(&mut self, old_name: &str, new_name: &str) {
        for user in &mut self.users {
            if user.name == old_name {
                user.name = new_name.to_owned();
            }
        }
    }
    pub fn remove_user(&mut self, name: &str) {
        self.users.retain(|u| u.name != name);
    }

    pub fn set_topic(&mut self, new_topic: String) {
        self.topic = new_topic;
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

impl Channel {
    pub fn user_is_op(&self, user: &str) -> bool {
        if let Some(user) = self.users.iter().find(|u| u.name == user) {
            if user.flags.iter().any(|f| *f == '@') {
                return true;
            }
            false
        } else {
            eprintln!("Tried to look up a user but it could not be found");
            eprintln!("Channel {:?} - user {:?}", self.name, user);
            eprintln!("All users: {:?}", self.users);
            false
        }
    }
}
