use pkrs::model::PkId;
use tracing::debug;

use super::db;
use crate::types::{Context, Error};

// TODO: command to see current settings

#[poise::command(slash_command, guild_only = true, rename = "setup-pk")]
pub(crate) async fn setup_pk(
    ctx: Context<'_>,
    #[description = "system id"] system_id: String,
    #[description = "(optional) PluralKit token"] token: Option<String>,
) -> Result<(), Error> {
    let guild = ctx.guild().ok_or("couldn't fetch guild")?.to_owned();
    let user_id = ctx.author().id;

    debug!(
        guild_id = guild.id.get(),
        guild_name = guild.name,
        command = "setup-pk",
        system_id = system_id
    );

    // sanitise and validate system id
    let system_id = system_id.trim().replace("-", "").to_lowercase();
    if !system_id.chars().all(|c| char::is_ascii_alphabetic(&c)) {
        ctx.reply(format!("error: invalid system id, {}", system_id))
            .await?;
        return Ok(());
    }

    db::save_guild_settings(
        &ctx.data().db,
        guild.id.get(),
        user_id.get(),
        &system_id,
        token.clone(),
    )
    .await?;

    let pk = pkrs::client::PkClient {
        token: token.unwrap_or("".into()).into(),
        ..Default::default()
    };

    // TODO: fix pkrs to actually handle 404s correctly
    let system = match pk.get_system(&PkId(system_id.clone().into())).await {
        Ok(system) => system,
        Err(err) => {
            ctx.reply(format!(
                "PluralKit API is having issues or system doesn't exist: {:?}",
                err
            ))
            .await?;
            return Ok(());
        }
    };

    // Inform user of success
    let response_text = format!(
        "PluralKit module setup with system: {}",
        match system.name {
            Some(system_name) => format!("{} (`{}`)", system_name, system_id),
            None => format!("`{}`", system_id),
        }
    );

    ctx.reply(response_text).await?;

    Ok(())
}
