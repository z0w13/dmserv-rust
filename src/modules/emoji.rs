use std::sync::Arc;

use crate::types::{Data, Error};

pub(crate) mod commands;
pub(crate) mod db;
pub(crate) mod event_handler;
pub(crate) mod shared;

pub(crate) fn commands() -> Vec<poise::Command<Arc<Data>, Error>> {
    vec![
        commands::emoji_stats::command(),
        commands::emoji_clone::command(),
    ]
}
