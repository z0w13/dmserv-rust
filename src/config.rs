use std::path::PathBuf;

use serde::Deserialize;

use crate::types::Error;

#[derive(Deserialize, Debug)]
pub(crate) struct BotConfig {
    pub(crate) token: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct DatabaseConfig {
    pub(crate) url: String,
}

pub(crate) struct Config {
    pub(crate) bot: BotConfig,
    pub(crate) db: DatabaseConfig,
}

pub(crate) fn load_config() -> Result<Config, Error> {
    let bot: BotConfig = serde_envfile::prefixed("TULPJE_").from_file(&PathBuf::from(".env"))?;
    let db: DatabaseConfig =
        serde_envfile::prefixed("DATABASE_").from_file(&PathBuf::from(".env"))?;

    Ok(Config { bot, db })
}
