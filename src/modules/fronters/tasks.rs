use std::sync::Arc;

use poise::serenity_prelude::{self as serenity};

use crate::modules::fronters;
use crate::types::{Data, Error};

use super::db::ModPkFrontersRow;

pub(crate) async fn update_fronters(ctx: &serenity::Context, data: Arc<Data>) -> Result<(), Error> {
    let fronter_cats = fronters::db::get_fronter_categories(&data.db).await?;

    for cat in fronter_cats {
        if let Err(err) = update_fronters_for_guild(ctx, cat).await {
            println!("{}", err);
        }
    }

    Ok(())
}

async fn update_fronters_for_guild(
    ctx: &serenity::Context,
    cat: ModPkFrontersRow,
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

    fronters::commands::update_fronter_channels(&ctx, guild.clone(), cat)
        .await
        .map_err(|err| {
            format!(
                "error updating fronters for {} ({}): {}",
                guild.name, guild.id, err
            )
        })?;

    println!("fronters updated '{}' ({})", guild.name, guild.id);

    Ok(())
}
