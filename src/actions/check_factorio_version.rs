//! Check factorio version
//!
//! This polls https://forums.factorio.com/viewforum.php?f=3 at a regular interval. If there is a
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
        let mut current_version = None;
        while client.read().running {
            sleep().await;
            let (url, version) = match get_last_version().await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Cannot poll factorio version: {:?}", e);
                    continue;
                }
            };
            if current_version.is_some() && current_version != Some(version.to_owned()) {
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
                    let url = format!(
                        "https://forums.factorio.com/{}",
                        if url.starts_with("./") {
                            &url[2..]
                        } else {
                            &url
                        }
                    );
                    split[1] = format!("Latest version: {} {}", version, url);
                    {
                        let sender = client.read();
                        let sender = &sender.sender;
                        if let Err(e) = sender.send_topic(&channel_name, split.join(" | ")) {
                            eprintln!("Could not set {} topic: {:?}", channel_name, e);
                        }
                        if let Err(e) = sender.send_privmsg(
                            &channel_name,
                            format!("Version {} released. {}", version, url),
                        ) {
                            eprintln!(
                                "Could not send version message to {}: {:?}",
                                channel_name, e
                            );
                        }
                    }
                }
            }
            current_version = Some(version);
        }
    });
}

async fn sleep() {
    tokio::time::delay_for(Duration::from_secs(60)).await;
}

#[tokio::test]
async fn load_version() {
    let (url, version) = get_last_version().await.expect("Could not load version");
    println!("Version url: {:?}", url);
    println!("Version: {:?}", version);
    assert!(
        url.starts_with("./viewtopic.php?f=3&amp;t="),
        "Url {:?} is not valid",
        url
    );
    assert!(
        !version.is_empty() && version.chars().all(|c| c == '.' || c.is_numeric()),
        "Version {} is not valid",
        version
    );
}

async fn get_last_version() -> Result<(String, String), String> {
    let response = reqwest::get("https://forums.factorio.com/viewforum.php?f=3")
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let regex = Regex::new(r#"<a href="([^"]*)"[^>]*topictitle">Version ([^<]*)<"#)
        .map_err(|e| e.to_string())?;

    if let Some(capture) = regex.captures_iter(&response).next() {
        Ok((capture[1].to_owned(), capture[2].to_owned()))
    } else {
        Err(String::from("Could not find version"))
    }
}
