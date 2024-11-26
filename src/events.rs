use std::sync::Arc;

use poise::serenity_prelude::{self as serenity};
use sqlx::types::chrono;
use tracing::{debug, info};

use crate::{
    modules::stats::ShardStats,
    types::{Data, Error},
};

pub(crate) async fn handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Arc<Data>, Error>,
    data: &Arc<Data>,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            let shard_stats = &data.stats.shards;
            let shard = data_about_bot
                .shard
                .expect("missing shard ID in FullEvent::Ready");
            let shard_id = shard.id.get();

            // create the shard stats if missing
            if !shard_stats.contains_key(&shard_id) {
                shard_stats.insert(
                    shard_id,
                    ShardStats::new(shard.id.get(), serenity::ConnectionStage::Connected),
                );
            }
            data.stats
                .set_connected_shards(data.stats.get_connected_shards() + 1);

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
        serenity::FullEvent::ShardStageUpdate { event } => {
            debug!(shard_id = event.shard_id.get(), old = ?event.old, new = ?event.new);

            let shard_id = event.shard_id.get();
            let mut shard_stats = data.stats.shards.get_mut(&shard_id).unwrap_or_else(||
                // we should always have the shard in in the map as it's created
                // on FullEvent::Ready
                panic!(
                    "no shard in shard_stats with id {}, shouldn't happen",
                    shard_id
                ));

            shard_stats.stage = event.new;

            // we are no longer connected to reset ready timestamp and add a restart
            if event.old == serenity::ConnectionStage::Connected {
                shard_stats.ready_at = None;
                shard_stats.restarts += 1;
                data.stats
                    .set_connected_shards(data.stats.get_connected_shards() - 1);
            } else if event.new == serenity::ConnectionStage::Connected {
                shard_stats.ready_at = Some(chrono::Utc::now());
                data.stats
                    .set_connected_shards(data.stats.get_connected_shards() + 1);
            }
        }
        _ => {}
    }
    Ok(())
}
