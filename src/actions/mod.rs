mod autojoin;
mod check_factorio_friday_facts;
mod check_factorio_version;
mod commands;
mod multiplayer_info;
mod wolfram_alpha;

use crate::data::{Client, Message};

pub async fn on_start(client: Client) -> Result<(), String> {
    if let Some(factorio_channel) = client.server_config().factorio_channel {
        check_factorio_version::spawn(client.clone(), factorio_channel.clone());
        check_factorio_friday_facts::spawn(client, factorio_channel);
    }
    commands::start();
    Ok(())
}

pub async fn on_message<'a>(message: &'a Message<'a>) -> Result<(), String> {
    futures::try_join!(
        commands::on_message(message),
        multiplayer_info::on_message(message),
        wolfram_alpha::on_message(message),
    )
    .map(|_| ())
}
