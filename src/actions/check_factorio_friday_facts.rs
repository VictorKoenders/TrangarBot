//! Check factorio friday facts
//!
//! This polls https://factorio.com/ at a regular interval. If there is a
//! new post available, it will be broadcasted to the IRC client.

use crate::data::Client;
use lazy_static::lazy_static;
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
                let topic = match client.find_channel(&channel_name) {
                    Some(channel) => channel.topic(),
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

                    client.set_channel_topic(&channel_name, split.join(" | "));
                    client.send_to_channel(
                        &channel_name,
                        format!("New factorio farts: {} {}", facts, url),
                    );
                }
            }
            last_facts = Some(facts);
        }
    });
}

async fn sleep() {
    tokio::time::sleep(Duration::from_secs(60 * 10)).await;
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

lazy_static! {
    static ref FRIDAY_FACTS_REGEX: Regex = Regex::new(r#"Friday Facts #([0-9\.]+)"#).unwrap();
}

async fn get_last_facts_post() -> Result<String, String> {
    let response = reqwest::get("https://factorio.com/")
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let mut captures = FRIDAY_FACTS_REGEX.captures_iter(&response);
    if let Some(capture) = captures.next() {
        Ok(capture[1].to_owned())
    } else {
        Err(String::from("Could not find last facts post"))
    }
}
