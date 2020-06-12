mod autojoin;
mod check_factorio_friday_facts;
mod check_factorio_version;
mod commands;
mod multiplayer_info;
mod wolfram_alpha;

use crate::data::Data;
use crate::Result;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn start(data: Data, running: Arc<AtomicBool>) {
    check_factorio_version::spawn(data.clone(), running.clone());
    check_factorio_friday_facts::spawn(data, running);
    commands::start();
}

pub fn execute(data: &Data, target: &str, message: &str, config: &crate::Config) -> Result<()> {
    commands::on_message(data, "", target, message);
    multiplayer_info::on_message(data, target, message, config);
    wolfram_alpha::on_message(data, target, message, config);
    Ok(())
}
