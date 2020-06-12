//! Check factorio friday facts
//!
//! This polls https://factorio.com/ at a regular interval. If there is a
//! new post available, it will be broadcasted to the IRC client.

use crate::data::Client;
use parking_lot::RwLock;
use regex::Regex;
use std::{sync::Arc, time::Duration};

fn find_channel_topic(client: &Arc<RwLock<Client>>, channel_name: &str) -> Option<String> {
    let client = client.read();
    let channel = client.find_channel(&channel_name)?;
    let channel = channel.read();
    let topic = channel.topic.to_owned();
    Some(topic.to_owned())
}

pub fn spawn(client: Arc<RwLock<Client>>, channel_name: String) {
    tokio::spawn(async move {
        let mut last_facts = None;
        while client.read().running {
            sleep().await;
            let facts = match get_last_facts_post().await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Cannot poll factorio version: {:?}", e);
                    continue;
                }
            };
            if last_facts.is_some() && last_facts != Some(facts.to_owned()) {
                let topic = match find_channel_topic(&client, &channel_name) {
                    Some(topic) => topic,
                    None => {
                        eprintln!("Tried to notify of a new factorio version, but could not find channel {:?}", channel_name);
                        continue;
                    }
                };
                let mut split: Vec<String> =
                    topic.split('|').map(|s| String::from(s.trim())).collect();

                if split.len() < 3 {
                    eprintln!("Invalid channel topic, expected at least 3 parts");
                    eprintln!("Topic is now: {:?}", topic);
                } else {
                    let url = format!("http://factorio.com/blog/post/fff-{}", facts);
                    split[2] = format!("Friday facts #{}: {}", facts, url);
                    {
                        let sender = client.read();
                        let sender = &sender.sender;

                        if let Err(e) = sender.send_topic(&channel_name, split.join(" | ")) {
                            eprintln!("Could not set {} topic: {:?}", channel_name, e);
                        }
                        if let Err(e) = sender.send_privmsg(
                            &channel_name,
                            format!("New factorio facts: {} {}", facts, url),
                        ) {
                            eprintln!(
                                "Could not send version message to {}: {:?}",
                                channel_name, e
                            );
                        }
                    }
                }
            }
            last_facts = Some(facts);
        }
    });
}

async fn sleep() {
    tokio::time::delay_for(Duration::from_secs(10)).await;
}

#[tokio::test]
async fn load_facts() {
    let facts = get_last_facts_post().await.expect("Could not load version");
    println!("Facts: {:?}", facts);
    assert!(
        !facts.is_empty() && facts.chars().all(|c| c == '.' || c.is_numeric()),
        "Facts version is invalid, got: {:?}",
        facts
    );
}

async fn get_last_facts_post() -> Result<String, String> {
    let response = reqwest::get("https://factorio.com/")
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let regex = Regex::new(r#"Friday Facts #([0-9\.]+)"#).map_err(|e| e.to_string())?;
    if let Some(capture) = regex.captures_iter(&response).next() {
        Ok(capture[1].to_owned())
    } else {
        Err(String::from("Could not find last facts post"))
    }
}
