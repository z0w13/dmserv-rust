use std::sync::Arc;

use poise::serenity_prelude::{self as serenity};

use crate::spawn_task;
use crate::types::{Data, Error};

pub(crate) mod commands;
pub(crate) mod db;
pub(crate) mod fronters;
pub(crate) mod roles;

pub(crate) fn commands() -> Vec<poise::Command<Arc<Data>, Error>> {
    vec![
        commands::setup_pk(),
        fronters::commands::setup_fronters(),
        fronters::commands::update_fronters(),
        roles::update_member_roles(),
    ]
}

// TODO: Replace tokio_schedule with something using tokio::time::interval and MissedTickBehavior tbh
pub(crate) fn start_tasks(ctx: serenity::Context, data: Arc<Data>) {
    spawn_task!(60, fronters::tasks::update_fronters, ctx, data);
}
