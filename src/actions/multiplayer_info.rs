use crate::data::Data;
use crate::Result;
use irc::client::ext::ClientExt;
use std::fmt::Write;

pub fn on_message(data: &Data, target: &str, message: &str, config: &crate::Config) {
    if message == "!mp" {
        let result = match load_games(config) {
            Ok(mut games) => {
                games.sort_unstable_by_key(|g| usize::max_value() - g.players.len());
                let mut response = String::from("Top 5 games: ");
                for (index, game) in games.iter().take(5).enumerate() {
                    if index > 0 {
                        response += ", ";
                    }
                    write!(
                        &mut response,
                        "{} ({} players, v{}{}{})",
                        game.name,
                        game.players.len(),
                        game.application_version.game_version,
                        if game.has_password { " +p" } else { "" },
                        if game.mod_count > 0 { " modded" } else { "" }
                    )
                    .expect("Could not append server text to string");
                }
                data.client.send_privmsg(target, response)
            }
            Err(e) => {
                eprintln!("Could not load games: {:?}", e);
                data.client.send_privmsg(target, "Error loading games")
            }
        };

        if let Err(e) = result {
            eprintln!("Could not send privmsg");
            eprintln!("{:?}", e);
        }
    }
}

#[test]
pub fn test() {
    let config = crate::Config::from_file("config.json");
    let games = load_games(&config).expect("Could not load games");
    println!("First MP game: {:?}", games.first());
}

fn load_games(config: &crate::Config) -> Result<Vec<GameInfo>> {
    let mut response = reqwest::get(&format!(
        "https://multiplayer.factorio.com/get-games?username={}&token={}",
        config.factorio_username, config.factorio_key
    ))?;
    let result = response.json()?;
    Ok(result)
}

#[derive(Debug, Deserialize)]
struct GameInfo {
    pub game_id: u64,
    pub name: String,
    #[serde(deserialize_with = "string_or_number_to_u64")]
    pub max_players: u64,
    #[serde(default)]
    pub players: Vec<String>,
    pub application_version: GameVersion,
    #[serde(deserialize_with = "string_or_number_to_u64")]
    pub game_time_elapsed: u64,
    #[serde(deserialize_with = "parse_string_bool")]
    pub has_password: bool,
    #[serde(default)]
    pub server_id: Option<String>,
    pub has_mods: bool,
    pub mod_count: u64,
}

#[derive(Debug, Deserialize)]
struct GameVersion {
    pub game_version: String,
    #[serde(deserialize_with = "string_or_number_to_u64")]
    pub build_version: u64,
    pub build_mode: String,
    pub platform: String,
}

fn parse_string_bool<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct IdVisitor;

    impl<'de> serde::de::Visitor<'de> for IdVisitor {
        type Value = bool;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("user ID as a number or string")
        }

        fn visit_bool<E>(self, b: bool) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(b)
        }

        fn visit_str<E>(self, id: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(id == "true")
        }
    }

    deserializer.deserialize_any(IdVisitor)
}

fn string_or_number_to_u64<'de, D>(deserializer: D) -> std::result::Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct IdVisitor;

    impl<'de> serde::de::Visitor<'de> for IdVisitor {
        type Value = u64;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("user ID as a number or string")
        }

        fn visit_u64<E>(self, id: u64) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(id)
        }

        fn visit_str<E>(self, id: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            id.parse().map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_any(IdVisitor)
}
