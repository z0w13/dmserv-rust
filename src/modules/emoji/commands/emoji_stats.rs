use poise::serenity_prelude::{self as serenity};
use tracing::trace;

use crate::modules::emoji::db;
use crate::modules::emoji::shared::StatsSort;
use crate::types::{Context, Error};

fn create_emoji_stats_sort_menu() -> serenity::CreateSelectMenu {
    serenity::CreateSelectMenu::new(
        "sort_by",
        serenity::CreateSelectMenuKind::String {
            options: vec![
                StatsSort::CountDesc.into(),
                StatsSort::CountAsc.into(),
                StatsSort::DateDesc.into(),
                StatsSort::DateAsc.into(),
            ],
        },
    )
    .placeholder("Sort")
}

async fn create_emoji_stats_embed(
    db: &sqlx::PgPool,
    guild: &serenity::PartialGuild,
    sort: &StatsSort,
) -> Result<serenity::CreateEmbed, Error> {
    let emoji_stats = db::get_emoji_stats(db, guild.id.get(), sort).await?;
    let emoji_str = if !emoji_stats.is_empty() {
        emoji_stats
            .into_iter()
            .map(|emoji_stats| {
                format!(
                    "{} • Used {} times • Last used <t:{}:R>",
                    emoji_stats.emoji,
                    emoji_stats.times_used,
                    emoji_stats.last_used_at.and_utc().timestamp(),
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    } else {
        "No Data".to_string()
    };

    Ok(serenity::CreateEmbed::new()
        .title(format!("{} Emotes in {}", sort.name(), guild.name))
        .description(emoji_str))
}

pub(crate) async fn handle_emoji_stats_sort(
    ctx: impl serenity::CacheHttp,
    db: &sqlx::PgPool,
    interaction: serenity::ComponentInteraction,
) -> Result<(), Error> {
    trace!(interaction = ?interaction.data);

    interaction
        .create_response(&ctx, serenity::CreateInteractionResponse::Acknowledge)
        .await?;

    let serenity::ComponentInteractionDataKind::StringSelect { values } = &interaction.data.kind
    else {
        return Err("couldn't get selected values".into());
    };
    trace!(values = ?values);

    let Some(sort_by) = values.first() else {
        return Err("couldn't get sort_by value".into());
    };
    trace!(sort_by = ?sort_by);

    let sort = StatsSort::try_from_string(sort_by)?;
    trace!(sort = ?sort);

    let guild = interaction
        .guild_id
        .ok_or("outside of guild")?
        .to_partial_guild(&ctx)
        .await?;

    trace!("editing response");
    let response = serenity::EditInteractionResponse::new()
        .embed(create_emoji_stats_embed(db, &guild, &sort).await?)
        .select_menu(create_emoji_stats_sort_menu());
    interaction.edit_response(&ctx, response).await?;

    Ok(())
}

#[poise::command(
    slash_command,
    guild_only = true,
    rename = "emoji-stats",
    default_member_permissions = "MANAGE_GUILD"
)]
pub(crate) async fn command(ctx: Context<'_>, sort: Option<StatsSort>) -> Result<(), Error> {
    let sort = sort.unwrap_or(StatsSort::CountDesc);
    let Context::Application(app_ctx) = ctx else {
        return Err("not app context".into());
    };

    let guild = ctx.partial_guild().await.ok_or("No Guild")?;
    let response = serenity::CreateInteractionResponse::Message(
        serenity::CreateInteractionResponseMessage::new()
            .embed(create_emoji_stats_embed(&ctx.data().db, &guild, &sort).await?)
            .select_menu(create_emoji_stats_sort_menu()),
    );
    app_ctx.interaction.create_response(&ctx, response).await?;

    Ok(())
}
