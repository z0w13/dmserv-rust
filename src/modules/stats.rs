use std::sync::Arc;
use std::sync::{
    atomic::{AtomicU32, AtomicU64, Ordering},
    Mutex,
};

use dashmap::DashMap;
use num_format::{Locale, ToFormattedString};
use poise::serenity_prelude::{self as serenity, ConnectionStage};
use sqlx::types::chrono::{self, Utc};
use tokio::spawn;
use tokio_schedule::{every, Job};
use tracing::error;

use crate::modules::fronters;
use crate::types::{Context, Data, Error};
use crate::util;

#[derive(Debug)]
pub(crate) struct Stats {
    pub(crate) num_cpus: usize,
    pub(crate) cpu_usage: AtomicU32,
    pub(crate) mem_usage: AtomicU64,
    pub(crate) started: chrono::DateTime<chrono::Utc>,
    pub(crate) shards: DashMap<u32, ShardStats>,
    pub(crate) total_shards: AtomicU32,
    pub(crate) connected_shards: AtomicU32,
}

impl Stats {
    pub(crate) fn new() -> Self {
        Self {
            started: chrono::Utc::now(),
            // TODO: kinda ugly, we're dealing with side-effects here, find a better way
            num_cpus: num_cpus(),
            cpu_usage: AtomicU32::new(0),
            mem_usage: AtomicU64::new(0),
            shards: DashMap::new(),
            total_shards: AtomicU32::new(0),
            connected_shards: AtomicU32::new(0),
        }
    }

    pub(crate) fn set_total_shards(&self, shards: u32) {
        self.total_shards.store(shards, Ordering::SeqCst);
    }

    pub(crate) fn get_total_shards(&self) -> u32 {
        self.total_shards.load(Ordering::SeqCst)
    }

    pub(crate) fn set_cpu_usage(&self, cpu_usage: f32) {
        self.cpu_usage
            .store((cpu_usage * 100.) as u32, Ordering::SeqCst)
    }

    pub(crate) fn get_cpu_usage(&self) -> f32 {
        self.cpu_usage.load(Ordering::SeqCst) as f32 / 100. / self.num_cpus as f32
    }

    pub(crate) fn set_mem_usage(&self, mem_usage: u64) {
        self.mem_usage.store(mem_usage, Ordering::SeqCst)
    }
    pub(crate) fn get_mem_usage(&self) -> u64 {
        self.mem_usage.load(Ordering::SeqCst)
    }

    pub(crate) fn inc_connected_shards(&self) {
        self.connected_shards
            .store(self.get_connected_shards() + 1, Ordering::SeqCst);
    }
    pub(crate) fn dec_connected_shards(&self) {
        self.connected_shards
            .store(self.get_connected_shards() - 1, Ordering::SeqCst);
    }

    pub(crate) fn get_connected_shards(&self) -> u32 {
        self.connected_shards.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub(crate) struct ShardStats {
    pub(crate) shard_id: u32,
    pub(crate) restarts: u32,
    pub(crate) stage: ConnectionStage,
    pub(crate) ready_at: Option<chrono::DateTime<Utc>>,
}

impl ShardStats {
    pub(crate) fn new(shard_id: u32, stage: ConnectionStage) -> Self {
        // if created while already connected set ready_at to now
        let ready_at = if stage == ConnectionStage::Connected {
            Some(chrono::Utc::now())
        } else {
            None
        };

        Self {
            shard_id,
            restarts: 0,
            stage,
            ready_at,
        }
    }

    pub(crate) async fn latency(&self, ctx: Context<'_>) -> Option<u128> {
        let runners = ctx.framework().shard_manager.runners.lock().await;
        let opt_shard = runners.get(&serenity::ShardId(self.shard_id));
        opt_shard.and_then(|s| s.latency).map(|l| l.as_millis())
    }
}

pub(crate) fn num_cpus() -> usize {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_list(sysinfo::CpuRefreshKind::everything());
    sys.cpus().len()
}

fn update_stats(
    _ctx: &serenity::Context,
    data: Arc<Data>,
    sys_mut: Arc<Mutex<sysinfo::System>>,
    pid: sysinfo::Pid,
) -> Result<(), Error> {
    let mut sys = sys_mut.lock().expect("update_stats mutex got poisoned");

    // NOTE: we're updating all processes as we get 0% usage if we only update our
    //       own PID, even collecting child PIDs first and then updating all of those
    //       doesn't fix the CPU usage
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::new().with_cpu(),
    );

    let proc = sys
        .process(pid)
        .ok_or("couldn't get stats for current process")?;

    data.stats.set_mem_usage(proc.memory());
    data.stats.set_cpu_usage(proc.cpu_usage());

    Ok(())
}

#[poise::command(slash_command)]
pub(crate) async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    let time_before = chrono::Utc::now().timestamp_millis();
    let msg = ctx.reply("...").await?;
    let time_after = chrono::Utc::now().timestamp_millis();
    let api_latency = time_after - time_before;
    let shard_id = ctx.guild_id().map(|g| g.shard_id(ctx.cache())).unwrap_or(0);
    let fronter_systems = fronters::db::get_system_count(&ctx.data().db).await?;
    let stats = &ctx.data().stats;
    let shard_stats = stats.shards.get(&shard_id).ok_or_else(|| {
        format!(
            "no shard in shard_stats with id {}, shouldn't happen",
            shard_id
        )
    })?;
    let mem_usage_mb = stats.get_mem_usage() as f64 / 1024. / 1024.;

    let embed = serenity::CreateEmbed::new()
        .title("DMServ Discord Bot")
        .url("https://github.com/z0w13/dmserv-rust")
        .field(
            "Version",
            format!(
                "{} ({}{})",
                env!("CARGO_PKG_VERSION"),
                env!("VERGEN_GIT_SHA"),
                match env!("VERGEN_GIT_DIRTY") {
                    "true" => "-dirty",
                    _ => "",
                },
            ),
            true,
        )
        .field("Servers", format!("{}", ctx.cache().guilds().len()), true)
        .field(
            "Current Shard",
            format!("Shard #{} (of {} total, {} are up)", shard_stats.shard_id, stats.get_total_shards(), stats.get_connected_shards()),
            true,
        )
        .field(
            "Shard Uptime",
            format!("{} ({} disconnections)",
            util::format_significant_duration(
                shard_stats
                    .ready_at
                    .expect("ready at was None for current shard which should be impossible as we couldn't respond to this command otherwise")
                    .signed_duration_since(Utc::now())
                    .num_seconds()
                    .unsigned_abs()
            ), shard_stats.restarts),
            true,
        )
        .field(
            "Latency",
            format!(
                "API: {} ms, Shard: {}",
                api_latency,
                shard_stats
                    .latency(ctx)
                    .await
                    .map_or("N/A".into(), |f| format!(
                        "{} ms",
                        f.to_formatted_string(&Locale::en)
                    ))
            ),
            true,
        )
        .field("CPU Usage", format!("{:.2} %", stats.get_cpu_usage()), true)
        .field("Memory Usage", format!("{:.02} MiB", mem_usage_mb), true)
        .field(
            "Other Stats",
            format!("Updating fronters for {} system(s)", fronter_systems),
            true,
        )
        .footer(serenity::CreateEmbedFooter::new(
            "DMServ • https://github.com/z0w13/dmserv-rust • Last Restarted:",
        ))
        .timestamp(stats.started);

    let reply = poise::CreateReply::default().content("").embed(embed);

    // Inform user of success
    msg.edit(ctx, reply).await?;
    Ok(())
}

#[poise::command(slash_command)]
pub(crate) async fn shards(ctx: Context<'_>) -> Result<(), Error> {
    let stats = &ctx.data().stats;

    let mut fields: Vec<(String, String, bool)> = Vec::new();
    for shard in stats.shards.iter() {
        fields.push((
            format!("Shard #{}", shard.shard_id),
            format!(
                "Latency: {} / Uptime: {} / Disconnects: {}",
                shard.latency(ctx).await.map_or("N/A".into(), |f| format!(
                    "{} ms",
                    f.to_formatted_string(&Locale::en)
                )),
                util::format_significant_duration(
                    shard
                        .ready_at
                        .unwrap_or(Utc::now())
                        .signed_duration_since(Utc::now())
                        .num_seconds()
                        .unsigned_abs(),
                ),
                shard.restarts,
            ),
            false,
        ));
    }
    fields.sort();

    let embed = serenity::CreateEmbed::new()
        .title("DMServ Shard Stats")
        .fields(fields);

    let reply = poise::CreateReply::default().content("").embed(embed);
    ctx.send(reply).await?;

    Ok(())
}

pub(crate) fn commands() -> Vec<poise::Command<Arc<Data>, Error>> {
    vec![stats(), shards()]
}

pub(crate) fn start_tasks(ctx: serenity::Context, data: Arc<Data>) {
    let sys = Arc::new(Mutex::new(sysinfo::System::new_all()));
    let pid = sysinfo::Pid::from_u32(std::process::id());

    spawn(every(1).seconds().perform(move || {
        let ctx = ctx.to_owned();
        let data = data.to_owned();
        let sys = sys.clone();

        async move {
            if let Err(err) = update_stats(&ctx, data, sys, pid) {
                error!("error updating stats: {}", err);
            }
        }
    }));
}
