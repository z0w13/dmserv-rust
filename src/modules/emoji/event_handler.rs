use std::collections::HashSet;
use std::sync::Arc;

use poise::serenity_prelude::{self as serenity};
use sqlx::types::chrono;
use tracing::{debug, error, trace};

use super::commands::emoji_stats::handle_emoji_stats_sort;
use super::{db, shared};
use crate::types::Data;
use crate::util;

pub(crate) struct EventHandler {
    pub(crate) data: Arc<Data>,
}

#[serenity::async_trait]
impl serenity::EventHandler for EventHandler {
    async fn message(&self, ctx: serenity::Context, msg: serenity::Message) {
        // only track messages in guilds
        let Some(guild_id) = msg.guild_id else {
            return;
        };

        // don't track PluralKit proxy messages
        if util::is_pk_proxy(&msg.application_id) {
            debug!("skipping PluralKit proxy message");
            return;
        }

        let timestamp = chrono::Utc::now();
        let emotes = shared::parse_emojis_from_string(guild_id.get(), &msg.content);

        trace!(message = msg.content, emotes = ?emotes, "message");

        for emote in emotes.into_iter() {
            if shared::is_guild_emoji(ctx.http.clone(), guild_id, emote.id.into()).await {
                if let Err(err) = db::save_emoji_use(&self.data.db, &emote, timestamp).await {
                    error!(err, guild_id = guild_id.get(), "db::save_emoji_use");
                };
            }
        }
    }
    async fn message_update(
        &self,
        ctx: serenity::Context,
        old_message: Option<serenity::Message>,
        new_message: Option<serenity::Message>,
        evt: serenity::MessageUpdateEvent,
    ) {
        // NOTE: old_message and new_message are always empty with default cache settings
        //       as default CacheSettings::max_messages = 0
        trace!(has_old = ?old_message.is_some(), has_new = ?new_message.is_some(), "message_update");

        let (Some(old_message), Some(new_message)) = (old_message, new_message) else {
            return;
        };

        // don't track PluralKit proxy messages
        if util::is_pk_proxy(&evt.application_id.flatten()) {
            debug!("skipping PluralKit proxy message");
            return;
        }

        let guild = match evt.guild_id {
            Some(id) => match id.to_partial_guild(&ctx).await {
                Ok(guild) => guild,
                Err(_err) => return,
            },
            None => return,
        };

        let Ok(guild_emojis) = guild.emojis(&ctx).await.map(|emojis| {
            emojis
                .into_iter()
                .map(|e| e.id.get())
                .collect::<HashSet<u64>>()
        }) else {
            return;
        };

        let timestamp = chrono::Utc::now();

        let old_emote_count = shared::count_emojis(
            shared::parse_emojis_from_string(guild.id.get(), &old_message.content)
                .into_iter()
                .filter(|e| guild_emojis.contains(&e.id))
                .collect::<Vec<db::Emoji>>(),
        );

        let new_emote_count = shared::count_emojis(
            shared::parse_emojis_from_string(guild.id.get(), &new_message.content)
                .into_iter()
                .filter(|e| guild_emojis.contains(&e.id))
                .collect::<Vec<db::Emoji>>(),
        );

        trace!(old = ?old_emote_count, new = ?new_emote_count, "message_update count");

        // Counting logic:
        //  In old but not new message? -> don't do anything, emote was "used"
        //  In both messages -> don't do anything, emote was "used"
        //  In new but not old message -> new "use" of emote

        for (emote, count) in new_emote_count {
            let change = count - old_emote_count.get(&emote).unwrap_or(&0);
            trace!(change = change, "message_update");

            if change <= 0 {
                // emote count has not incremented, don't need to track
                continue;
            }

            if let Err(err) = db::save_emoji_use(&self.data.db, &emote, timestamp).await {
                error!(
                    err,
                    guild_id = guild.id.get(),
                    emote = ?emote,
                    "db::save_emoji_use"
                );
            };
        }
    }

    async fn reaction_add(&self, ctx: serenity::Context, reaction: serenity::Reaction) {
        debug!(reaction = ?reaction, "reaction_add");
        match reaction.emoji {
            serenity::ReactionType::Custom { animated, id, name } => {
                let now = chrono::Utc::now();
                let (Some(guild_id), Some(name)) = (reaction.guild_id, name) else {
                    return;
                };

                if !shared::is_guild_emoji(ctx.http, guild_id, id).await {
                    return;
                }

                let emote = db::Emoji {
                    id: id.get(),
                    guild_id: guild_id.get(),
                    name: name.clone(),
                    animated,
                };

                if let Err(err) = db::save_emoji_use(&self.data.db, &emote, now).await {
                    error!(err, "db::save_emoji_use");
                };
            }
            serenity::ReactionType::Unicode(_) | _ => {
                // NOTE: We ignore unicode emojis, we're tracking emoji use to see which
                //       are underused, unicode emojis are global anyway
            }
        }
    }

    async fn interaction_create(&self, ctx: serenity::Context, interaction: serenity::Interaction) {
        // convert to component interaction as we only want those
        let Some(interaction) = interaction.message_component() else {
            trace!("not sort_by interaction, ignoring");
            return;
        };

        // only handle the sort_by interaction
        if interaction.data.custom_id != "sort_by" {
            trace!("not sort_by interaction, ignoring");
            return;
        }

        if let Err(err) = handle_emoji_stats_sort(&ctx, &self.data.db, interaction).await {
            error!(err)
        }
    }
}
