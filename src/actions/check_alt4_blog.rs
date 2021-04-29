//! Check alt-f4 alternative friday facts
//!
//! This polls https://alt-f4.blog/ at a regular interval. If there is a
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
                let url = format!("https://alt-f4.blog/ALTF4-{}/", facts);
                client.send_to_channel(
                    &channel_name,
                    format!("New Alt-f4 facts: #{} {}", facts, url),
                );
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
    let facts = get_last_facts_post()
        .await
        .expect("Could not find last alt facts post");
    assert!(
        !facts.is_empty(),
        "Facts version is invalid, got: {:?}",
        facts
    );
}

lazy_static! {
    static ref FACT_POST_REGEX: Regex = Regex::new(r#"\#([0-9\.]+)"#).unwrap();
}

async fn get_last_facts_post() -> Result<String, String> {
    let response = reqwest::get("https://alt-f4.blog/")
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let mut captures = FACT_POST_REGEX.captures_iter(&response);
    if let Some(capture) = captures.next() {
        Ok(capture[0].replace("#", ""))
    } else {
        Err(String::from("Could not find last alt facts post"))
    }
}
