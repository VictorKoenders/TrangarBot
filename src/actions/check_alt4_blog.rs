//! Check alt-f4 alternative friday facts
//!
//! This polls https://alt-f4.blog/ at a regular interval. If there is a
//! new post available, it will be broadcasted to the IRC client.

use crate::data::Client;
use regex::Regex;
use std::time::Duration;

pub fn spawn(client: Client, channel_name: String) {
    tokio::spawn(async move {
        let mut last_facts = None;
        while client.running() {
            sleep().await;
            let facts = match get_last_facts_post().await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Cannot poll factorio version: {:?}", e);
                    continue;
                }
            };
            if last_facts.is_some() && last_facts != Some(facts.clone()) {
                let url = format!("https://alt-f4.blog/ALTF4-{}/", facts);
                client
                    .send_to_channel(&channel_name, format!("New Al-f4 facts: {} {}", facts, url));

                /*
                // TODO:
                let topic = match client.find_channel(&channel_name) {
                    Some(channel) => channel.topic(),
                    None => {
                        eprintln!(
                            "Tried to notify of a new alt-f4 blog, but could not find channel {:?}",
                            channel_name
                        );
                        continue;
                    }
                };
                let split: Vec<_> =
                    topic.split('|').map(|s| s.trim()).collect();

                if split.len() < 3 {
                    eprintln!("Invalid channel topic, expected at least 3 parts");
                    eprintln!("Topic is now: {:?}", topic);
                } else {
                    let url = format!("https://alt-f4.blog/ALTF4-{}/", facts);
                    split[3] = format!("{}: {}", url, facts);

                    // client.set_channel_topic(&channel_name, split.join(" | "));
                    client.send_to_channel(
                        &channel_name,
                        format!("New Al-f4 facts: #{} {}", facts, url),
                    );
                }
                */
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
    let facts = get_last_facts_post()
        .await
        .expect("Could not find last alt facts post");
    assert!(
        !facts.is_empty(),
        "Facts version is invalid, got: {:?}",
        facts
    );
}

async fn get_last_facts_post() -> Result<String, String> {
    let response = reqwest::get("https://alt-f4.blog/")
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let regex = Regex::new(r#"\#([0-9\.]+)"#).map_err(|e| e.to_string())?;
    if let Some(capture) = regex.captures_iter(&response).next() {
        Ok(capture[0].to_owned().replace("#", ""))
    } else {
        Err(String::from("Could not find last alt facts post"))
    }
}
