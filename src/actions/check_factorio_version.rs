//! Check factorio version
//!
//! This polls https://forums.factorio.com/viewforum.php?f=3 at a regular interval. If there is a
//! new post available, it will be broadcasted to the IRC client.

use crate::data::Client;
use lazy_static::lazy_static;
use regex::Regex;
use std::time::Duration;

pub fn spawn(client: Client, channel_name: String) {
    tokio::spawn(async move {
        let mut current_version = None;
        while client.running() {
            sleep().await;
            let (url, version) = match get_last_version().await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Cannot poll factorio version: {:?}", e);
                    continue;
                }
            };
            if current_version.is_some() && current_version != Some(version.clone()) {
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
                    let url = format!(
                        "https://forums.factorio.com/{}",
                        url.strip_prefix("./").unwrap_or_else(|| url.as_str())
                    );
                    split[1] = format!("Latest version: {} {}", version, url);

                    client.set_channel_topic(&channel_name, split.join(" | "));
                    client.send_to_channel(
                        &channel_name,
                        format!("Version {} released. {}", version, url),
                    );
                }
            }
            current_version = Some(version);
        }
    });
}

async fn sleep() {
    tokio::time::sleep(Duration::from_secs(60 * 10)).await;
}

#[tokio::test]
async fn load_version() {
    let (url, version) = get_last_version().await.expect("Could not load version");
    println!("Version url: {:?}", url);
    println!("Version: {:?}", version);
    assert!(
        url.starts_with("./viewtopic.php?f=3&t="),
        "Url {:?} is not valid",
        url
    );
    assert!(
        !version.is_empty() && version.chars().all(|c| c == '.' || c.is_numeric()),
        "Version {} is not valid",
        version
    );
}

lazy_static! {
    static ref VERSION_REGEX: Regex =
        Regex::new(r#"<a href="([^"]*)"[^>]*topictitle">Version ([^<]*)<"#).unwrap();
}

async fn get_last_version() -> Result<(String, String), String> {
    let response = reqwest::get("https://forums.factorio.com/viewforum.php?f=3")
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let mut captures = VERSION_REGEX.captures_iter(&response);

    if let Some(capture) = captures.next() {
        Ok((capture[1].replace("&amp;", "&"), capture[2].to_owned()))
    } else {
        Err(String::from("Could not find version"))
    }
}
