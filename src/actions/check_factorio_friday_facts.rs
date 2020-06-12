//! Check factorio friday facts
//!
//! This polls https://factorio.com/ at a regular interval. If there is a
//! new post available, it will be broadcasted to the IRC client.
//!
//! If `running` is set to `false`, this thread will end

use irc::client::ext::ClientExt;
use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn spawn(data: crate::data::Data, running: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut last_facts = None;
        while running.load(Ordering::Relaxed) {
            let facts = match get_last_facts_post() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Cannot poll factorio version: {:?}", e);
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    continue;
                }
            };
            if last_facts != Some(facts.clone()) {
                if let Some(channel) = data.get_channel_topic("#factorio") {
                    let mut split: Vec<String> = channel
                        .topic
                        .split('|')
                        .map(|s| String::from(s.trim()))
                        .collect();

                    if split.len() < 3 {
                        eprintln!("Invalid channel topic, expected at least 3 parts");
                        eprintln!("Topic is now: {:?}", channel.topic);
                    } else {
                        let url = format!("http://factorio.com/blog/post/fff-{}", facts);
                        split[2] = format!("Friday facts #{}: {}", facts, url);
                        if let Err(e) = data.client.send_topic("#factorio", split.join(" | ")) {
                            eprintln!("Could not set #factorio topic: {:?}", e);
                        }
                        if let Err(e) = data.client.send_privmsg(
                            "#factorio",
                            format!("New factorio facts: {} {}", facts, url),
                        ) {
                            eprintln!("Could not send version message to #factorio: {:?}", e);
                        }
                    }
                }
            }
            last_facts = Some(facts);

            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}

#[test]
fn load_facts() {
    let facts = get_last_facts_post().expect("Could not load version");
    println!("Facts: {:?}", facts);
    assert!(
        !facts.is_empty() && facts.chars().all(|c| c == '.' || c.is_numeric()),
        "Facts version is invalid, got: {:?}",
        facts
    );
}

fn get_last_facts_post() -> Result<String, failure::Error> {
    let response = reqwest::get("https://factorio.com/")?.text()?;
    let regex = Regex::new(r#"Friday Facts #([0-9\.]+)"#)?;
    if let Some(capture) = regex.captures_iter(&response).next() {
        Ok(capture[1].to_owned())
    } else {
        failure::bail!("Could not find last facts post");
    }
}
