//! Check factorio version
//!
//! This polls https://forums.factorio.com/viewforum.php?f=3 at a regular interval. If there is a
//! new post available, it will be broadcasted to the IRC client.
//!
//! If `running` is set to `false`, this thread will end

use irc::client::ext::ClientExt;
use regex::Regex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn spawn(data: crate::data::Data, running: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut current_version = None;
        while running.load(Ordering::Relaxed) {
            let (version, url) = match get_last_version() {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Cannot poll factorio version: {:?}", e);
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    continue;
                }
            };
            if current_version != Some(version.clone()) {
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
                        let url = format!(
                            "https://forums.factorio.com/{}",
                            if url.starts_with("./") {
                                &url[2..]
                            } else {
                                &url
                            }
                        );
                        split[1] = format!("Latest version: {} {}", version, url);
                        if let Err(e) = data.client.send_topic("#factorio", split.join(" | ")) {
                            eprintln!("Could not set #factorio topic: {:?}", e);
                        }
                        if let Err(e) = data.client.send_privmsg(
                            "#factorio",
                            format!("Version {} released. {}", version, url),
                        ) {
                            eprintln!("Could not send version message to #factorio: {:?}", e);
                        }
                    }
                }
            }
            current_version = Some(version);

            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}

#[test]
fn load_version() {
    let (url, version) = get_last_version().expect("Could not load version");
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

fn get_last_version() -> Result<(String, String), failure::Error> {
    let response = reqwest::get("https://forums.factorio.com/viewforum.php?f=3")?.text()?;
    let regex = Regex::new(r#"<a href="([^"]*)"[^>]*topictitle">Version ([^<]*)<"#)?;
    if let Some(capture) = regex.captures_iter(&response).next() {
        Ok((capture[1].to_owned(), capture[2].to_owned()))
    } else {
        failure::bail!("Could not find version");
    }
}
