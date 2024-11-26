use std::sync::Arc;

use poise::serenity_prelude::{self as serenity};
use tracing::info;

use crate::types::{Data, Error};

pub(crate) async fn handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    framework: poise::FrameworkContext<'_, Arc<Data>, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            info!(
                user_id = data_about_bot.user.id.get(),
                "connected to discord as '{}{}'",
                data_about_bot.user.name,
                data_about_bot
                    .user
                    .discriminator
                    .map_or("".into(), |d| format!("#{}", d)),
            );
        }
        _ => {}
    }
    Ok(())
}
