//! WolframAlpha
//!
//! Queries wolfram alpha with the given text

use super::Data;
use irc::client::ext::ClientExt;
use serde_json::Value;

pub fn on_message(data: &Data, target: &str, message: &str, config: &crate::Config) {
    if message.starts_with("!wa") {
        let query = &message["!wa".len() + 1..].trim();
        let send_result = match query_wolfram_alpha(config, query) {
            Ok(result) => data.client.send_privmsg(target, result),
            Err(e) => {
                eprintln!("Could not query WA: {:?}", e);
                data.client
                    .send_privmsg(target, format!("Could not query WA: {:?}", e))
            }
        };
        if let Err(e) = send_result {
            eprintln!("Could not reply with a WA result: {:?}", e);
        }
    }
}

pub fn query_wolfram_alpha(config: &crate::Config, query: &str) -> Result<String, failure::Error> {
    let mut url = url::Url::parse("https://api.wolframalpha.com/v2/query")?;
    url.query_pairs_mut()
        .append_pair("input", query)
        .append_pair("appid", &config.wolframalpha)
        .append_pair("output", "json");

    let mut response = reqwest::get(url.as_str())?;
    let json: Value = response.json()?;

    Ok(
        if let Some(Value::Array(pods)) = json.pointer("/queryresult/pods") {
            if let Some(primary_pod) = pods
                .iter()
                .find(|p| p.get("primary") == Some(&Value::Bool(true)))
            {
                let mut result = Vec::new();
                if let Some(s) = primary_pod.get("title").and_then(|v| v.as_str()) {
                    result.push(s);
                }
                if let Some(arr) = primary_pod["subpods"].as_array() {
                    for subpod in arr {
                        if let Some(&Value::String(ref title)) = subpod.get("title") {
                            result.push(title);
                        }
                        if let Some(&Value::String(ref plaintext)) = subpod.get("plaintext") {
                            result.push(plaintext);
                        }
                    }
                }

                result
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .enumerate()
                    .fold(String::new(), |mut acc, (index, s)| {
                        if index == 1 {
                            acc += ": ";
                        } else if index > 1 {
                            acc += " | ";
                        }
                        acc += &s.replace('\n', " ");
                        acc
                    })
            } else {
                format!("Did not find primary pods: {:?}", pods)
            }
        } else if let Some(didyoumean) = json.pointer("/queryresults/didyoumeans") {
            format!("Didyoumeans: {:?}", didyoumean)
        } else {
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
            String::from("Did not find WA result, Trangar has been pinged to fix this")
        },
    )
}

#[test]
pub fn test() {
    let config = crate::Config::from_file("config.json");
    let response = query_wolfram_alpha(&config, "machine train").expect("Could not query WA");
    println!("{}", response);
}
