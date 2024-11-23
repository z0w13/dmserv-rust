use std::sync::Arc;

use poise::serenity_prelude::{self as serenity};
use tracing::{error, info, warn};

use crate::modules::{fronters, pk};
use crate::types::{Data, Error};

use self::pk::db::ModPkGuildRow;

use super::db::ModPkFrontersRow;

pub(crate) async fn update_fronters(ctx: &serenity::Context, data: Arc<Data>) -> Result<(), Error> {
    let fronter_cats = fronters::db::get_fronter_categories(&data.db).await?;
    let guild_settings = pk::db::get_guild_settings(&data.db).await?;

    for cat in fronter_cats {
        let cur_guild_settings = guild_settings
            .iter()
            .find(|gs| u64::try_from(gs.guild_id).unwrap() == cat.guild_id);

        if let Some(gs) = cur_guild_settings {
            if let Err(err) = update_fronters_for_guild(ctx, &gs, &cat).await {
                error!(guild_id = cat.guild_id, category_id = cat.category_id, err);
            }
        } else {
            warn!(
                guild_id = cat.guild_id,
                "couldn't find guild settings for guild"
            );
        }
    }

    Ok(())
}

async fn update_fronters_for_guild(
    ctx: &serenity::Context,
    gs: &ModPkGuildRow,
    cat: &ModPkFrontersRow,
) -> Result<(), Error> {
    let guild = ctx.http.get_guild(cat.guild_id.into()).await?;

    let cat = ctx
        .http
        .get_channel(cat.category_id.into())
        .await
        .map_err(|err| {
            format!(
                "couldn't find category for guild '{}' ({}) {}",
                guild.name, guild.id, err
            )
        })?
        .guild()
        .ok_or(format!(
            "channel {} for guild '{}' ({}) isn't a guild channel",
            cat.category_id, guild.name, guild.id
        ))?;

    fronters::commands::update_fronter_channels(&ctx, guild.clone(), gs, cat)
        .await
        .map_err(|err| {
            format!(
                "error updating fronters for {} ({}): {}",
                guild.name, guild.id, err
            )
        })?;

    info!(
        guild.id = guild.id.get(),
        guild.name = guild.name,
        "fronters updated"
    );

    Ok(())
}
