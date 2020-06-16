//! WolframAlpha
//!
//! Queries wolfram alpha with the given text

use crate::data::Message;
use serde_json::Value;

pub async fn on_message<'a>(message: &'a Message<'a>) -> Result<(), String> {
    if message.body.starts_with("!wa") {
        let query = &message.body["!wa".len() + 1..].trim();
        match query_wolfram_alpha(message.config, query).await {
            Ok(result) => message.reply(&result),
            Err(e) => {
                eprintln!("Could not query WA: {:?}", e);
                message.reply(&format!("Could not query WA: {:?}", e));
            }
        }
    }
    Ok(())
}

pub async fn query_wolfram_alpha(config: &crate::Config, query: &str) -> Result<String, String> {
    let mut url =
        url::Url::parse("https://api.wolframalpha.com/v2/query").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("input", query)
        .append_pair("appid", &config.wolframalpha)
        .append_pair("output", "json");

    let json: serde_json::Value = reqwest::get(url.as_str())
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

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
                        match index {
                            1 => acc += ": ",
                            n if n > 1 => acc += " | ",
                            _ => {}
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

#[tokio::test]
pub async fn test() {
    if let Ok(config) = crate::Config::from_file("config.json") {
        let response = query_wolfram_alpha(&config, "machine train")
            .await
            .expect("Could not query WA");
        println!("{}", response);
    }
}
