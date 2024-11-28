use std::sync::Arc;

use poise::serenity_prelude::{self as serenity};
use sqlx::types::chrono;
use tracing::{debug, info};

use crate::{modules::stats::ShardStats, types::Data};

pub(crate) struct EventHandler {
    pub(crate) data: Arc<Data>,
}

#[serenity::async_trait]
impl serenity::EventHandler for EventHandler {
    async fn ready(&self, _ctx: serenity::Context, data_about_bot: serenity::Ready) {
        info!(
            user_id = data_about_bot.user.id.get(),
            shard = ?data_about_bot.shard,
            "connected to discord as '{}{}'",
            data_about_bot.user.name,
            data_about_bot
                .user
                .discriminator
                .map_or("".into(), |d| format!("#{}", d)),
        );
    }
    async fn shard_stage_update(
        &self,
        _ctx: serenity::Context,
        event: serenity::ShardStageUpdateEvent,
    ) {
        let shard_id = event.shard_id.get();
        debug!(shard_id = shard_id, old = ?event.old, new = ?event.new);

        // create the shard stats if missing
        if !self.data.stats.shards.contains_key(&shard_id) {
            self.data
                .stats
                .shards
                .insert(shard_id, ShardStats::new(shard_id, event.new));
        }

        let mut shard_stats = self.data.stats.shards.get_mut(&shard_id).unwrap();

        shard_stats.stage = event.new;
        if event.old == serenity::ConnectionStage::Connected {
            // we are no longer connected to reset ready timestamp and add a restart
            shard_stats.ready_at = None;
            shard_stats.restarts += 1;
            self.data.stats.dec_connected_shards();
        } else if event.new == serenity::ConnectionStage::Connected {
            shard_stats.ready_at = Some(chrono::Utc::now());
            self.data.stats.inc_connected_shards();
        }
    }
}
