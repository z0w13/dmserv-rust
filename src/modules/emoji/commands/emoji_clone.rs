use base64::{prelude::BASE64_STANDARD, Engine as _};
use futures::StreamExt;
use poise::serenity_prelude::{self as serenity};

use crate::modules::emoji::db::Emoji;
use crate::modules::emoji::shared::parse_emojis_from_string;
use crate::types::{Context, Error};

// requires CREATE_GUILD_EXPRESSIONS permission
#[poise::command(
    slash_command,
    guild_only = true,
    rename = "emoji-clone",
    default_member_permissions = "MANAGE_GUILD"
)]
pub(crate) async fn command(
    ctx: Context<'_>,
    emoji: String,
    new_name: Option<String>,
    prefix: Option<String>,
) -> Result<(), Error> {
    let Some(guild_id) = ctx.guild_id() else {
        unreachable!("command is guild_only");
    };

    let emojis = parse_emojis_from_string(1, &emoji);
    if emojis.is_empty() {
        ctx.reply("no emojis found").await?;
        return Ok(());
    }

    // defer response, we might take a while
    ctx.defer().await?;

    if let Some(new_name) = new_name {
        // add single emote with new_name
        if emojis.len() > 1 {
            ctx.reply("can't add more than one emote at a time when specifying name")
                .await?;
            return Ok(());
        }

        ctx.reply(
            match clone_emoji(ctx, guild_id.get(), emojis.first().unwrap(), &new_name).await {
                Ok(emoji) => format!("**Added:** {}", emoji),
                Err(err) => format!("**Error:** {}", err),
            },
        )
        .await?;

        return Ok(());
    } else {
        // add multiple emotes
        let prefix = prefix.unwrap_or("".into());

        // what a fucken mess to have async map, but it works :)
        let emoji_results: Vec<Result<Emoji, EmojiError>> =
            futures::stream::iter(emojis.into_iter().map(|e| {
                let prefix = prefix.clone();
                async move {
                    clone_emoji(ctx, guild_id.get(), &e, &format!("{}{}", prefix, e.name)).await
                }
            }))
            .buffered(1)
            .collect()
            .await;

        let emojis_added: Vec<String> = emoji_results
            .iter()
            .filter_map(|r| match r {
                Ok(emoji) => Some(emoji.to_string()),
                Err(_) => None,
            })
            .collect();

        let emoji_errors: Vec<String> = emoji_results
            .iter()
            .filter_map(|r| match r {
                Ok(_) => None,
                Err(e) => Some(format!("* {}", e)),
            })
            .collect();

        ctx.reply(format!(
            "{}\n{}",
            match emojis_added.is_empty() {
                true => "".into(),
                false => format!("**Added:** {}", emojis_added.join("")),
            },
            match emoji_errors.is_empty() {
                true => "".into(),
                false => format!("**Errors:**\n{}", emoji_errors.join("\n")),
            },
        ))
        .await?;
    }

    Ok(())
}

async fn download_emoji(id: u64, animated: bool) -> Result<String, reqwest::Error> {
    reqwest::get(format!(
        "https://cdn.discordapp.com/emojis/{}.{}",
        id,
        match animated {
            true => "gif",
            false => "webp",
        },
    ))
    .await?
    .error_for_status()? // error if we don't get a 200 status
    .bytes()
    .await
    .map(|b| {
        // convert to a data uri
        format!(
            "data:{};base64,{}",
            match animated {
                true => "image/gif",
                false => "image/webp",
            },
            BASE64_STANDARD.encode(b),
        )
    })
}
async fn clone_emoji(
    ctx: Context<'_>,
    guild_id: u64,
    emoji: &Emoji,
    new_name: &str,
) -> Result<Emoji, EmojiError> {
    let emoji_data_uri = download_emoji(emoji.id, emoji.animated)
        .await
        .map_err(|err| EmojiError::Download(emoji.clone(), err))?;

    let new_emoji = ctx
        .partial_guild()
        .await
        .ok_or(EmojiError::Other(
            emoji.clone(),
            "couldn't fetch guild".into(),
        ))?
        .create_emoji(&ctx, new_name, &emoji_data_uri)
        .await
        .map_err(|e| EmojiError::Create(emoji.clone(), e))?;

    Ok(Emoji::from_serenity(new_emoji, guild_id))
}

pub(crate) enum EmojiError {
    Other(Emoji, Error),
    Download(Emoji, reqwest::Error),
    Create(Emoji, serenity::Error),
}

impl EmojiError {
    pub(crate) fn as_str(&self) -> String {
        match self {
            Self::Download(emoji, err) => {
                format!("error downloading emoji ({}): {}", emoji.name, err)
            }
            Self::Create(emoji, err) => {
                format!("error creating emoji ({}): {}", emoji.name, err)
            }
            Self::Other(emoji, err) => format!("unkown error ({}): {}", emoji.name, err),
        }
    }
}

impl std::fmt::Display for EmojiError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(&self.as_str())
    }
}
impl std::fmt::Debug for EmojiError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, fmt)
    }
}

impl std::error::Error for EmojiError {}
