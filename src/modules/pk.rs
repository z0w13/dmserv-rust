use std::sync::Arc;

use crate::types::{Data, Error};

pub(crate) mod commands;
pub(crate) mod db;

pub(crate) fn commands() -> Vec<poise::Command<Arc<Data>, Error>> {
    vec![commands::setup_pk()]
}
