use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;

use pkrs::model::PkId;
use poise::serenity_prelude::{self as serenity, CacheHttp};

use crate::types::{Context, Error};

async fn get_desired_fronters(system: &PkId, token: String) -> Result<HashSet<String>, Error> {
    let pk = pkrs::client::PkClient {
        token: token.into(),
        ..Default::default()
    };

    let fronters = pk
        .get_system_fronters(system)
        .await?
        .members
        .iter()
        .filter_map(|m| match m {
            serde_either::StringOrStruct::String(_) => None,
            serde_either::StringOrStruct::Struct(member) => {
                Some(member.display_name.clone().unwrap_or(member.name.clone()))
            }
        })
        .collect();

    Ok(fronters)
}

async fn get_fronter_channels(
    ctx: &serenity::Context,
    guild: serenity::GuildId,
    cat_id: serenity::ChannelId,
) -> Result<Vec<serenity::GuildChannel>, Error> {
    let channels = ctx
        .http()
        .get_guild(guild)
        .await?
        .channels(&ctx)
        .await?
        .into_values()
        .filter(|c| c.parent_id.is_some() && c.parent_id.unwrap() == cat_id)
        .collect();
    Ok(channels)
}

#[derive(Debug, Clone)]
struct NoFronterCategoryError {
    id: String,
    name: String,
}

impl error::Error for NoFronterCategoryError {}

impl fmt::Display for NoFronterCategoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "No fronter category for server '{}' ({})",
            self.name, self.id
        )
    }
}

// TODO: Instrument why this bitch slow, are we even using discord's cache?
//       should definitely do that
pub(crate) async fn update_fronter_channels(
    ctx: &serenity::Context,
    guild: serenity::PartialGuild,
) -> Result<(), Error> {
    let fronters_category = match guild.channels(&ctx).await?.into_values().find(|c| {
        c.name.to_lowercase() == "current fronters" && c.kind == serenity::ChannelType::Category
    }) {
        None => {
            return Err(NoFronterCategoryError {
                id: guild.id.to_string(),
                name: guild.name,
            }
            .into());
        }
        Some(cat) => cat,
    };

    let fronter_channels = get_fronter_channels(ctx, guild.id, fronters_category.id).await?;
    let desired_fronters = get_desired_fronters(&PkId("***REMOVED***".into()), "".into()).await?;
    let current_fronters: HashSet<String> =
        fronter_channels.iter().map(|c| c.name.clone()).collect();

    let mut fronter_channel_map: HashMap<String, serenity::GuildChannel> = fronter_channels
        .iter()
        .map(|c| (c.name.clone(), c.to_owned()))
        .collect();

    let fronter_pos_map: HashMap<String, u16> = desired_fronters
        .iter()
        .enumerate()
        .map(|(k, v)| (v.clone(), k.try_into().unwrap()))
        .collect();

    let delete_fronters = current_fronters.difference(&desired_fronters);
    let create_fronters = desired_fronters.difference(&current_fronters);

    for fronter in delete_fronters {
        let channel = fronter_channel_map.get(fronter).unwrap();
        match channel.delete(&ctx).await {
            Ok(_) => (),
            Err(e) => {
                println!("Error deleting channel: {}", e);
                continue;
            }
        }

        fronter_channel_map.remove(fronter);
    }

    for fronter in create_fronters {
        let pos = fronter_pos_map
            .get(fronter)
            .expect("Couldn't get position for fronter")
            .to_owned();

        let channel = match guild
            .create_channel(
                &ctx,
                serenity::CreateChannel::new(fronter)
                    .position(pos)
                    .category(fronters_category.id)
                    .kind(serenity::ChannelType::Voice),
            )
            .await
        {
            Ok(chan) => chan,
            Err(e) => {
                println!("Error deleting channel: {}", e);
                continue;
            }
        };

        fronter_channel_map.insert(fronter.clone(), channel.to_owned());
    }
    for (name, position) in fronter_pos_map.iter() {
        let channel = fronter_channel_map
            .get(name)
            .expect("Couldn't get channel from fronter_channel_map");

        if channel.position == *position {
            continue;
        }

        match channel
            .clone()
            .edit(&ctx, serenity::EditChannel::new().position(*position))
            .await
        {
            Ok(_) => (),
            Err(e) => {
                println!("Error moving channel: {}", e);
                continue;
            }
        };
    }

    Ok(())
}

#[poise::command(slash_command, guild_only = true, rename = "update-fronters")]
pub(crate) async fn update_fronters(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let guild = ctx.partial_guild().await.unwrap();

    match update_fronter_channels(ctx.serenity_context(), guild).await {
        Err(e) => match e.downcast_ref::<NoFronterCategoryError>() {
            Some(_) => {
                ctx.reply(
                    "No fronters category in the server, please create a 'Current Fronters' category",
                )
                .await?;

                return Ok(());
            }
            None => return Err(e),
        },
        Ok(_) => (),
    };

    ctx.reply("Done!").await?;
    Ok(())
}
