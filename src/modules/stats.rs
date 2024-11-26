use std::sync::Arc;
use std::sync::{
    atomic::{AtomicU32, AtomicU64, Ordering},
    Mutex,
};

use num_format::{Locale, ToFormattedString};
use poise::serenity_prelude::{self as serenity};
use sqlx::types::chrono;
use tokio::spawn;
use tokio_schedule::{every, Job};
use tracing::{debug, error};

use crate::modules::fronters;
use crate::types::{Context, Data, Error};

#[derive(Debug)]
pub(crate) struct Stats {
    pub(crate) num_cpus: usize,
    pub(crate) cpu_usage: AtomicU32,
    pub(crate) mem_usage: AtomicU64,
    pub(crate) started: chrono::DateTime<chrono::Utc>,
}

impl Stats {
    pub(crate) fn new() -> Self {
        Self {
            started: chrono::Utc::now(),
            // TODO: kinda ugly, we're dealing with side-effects here, find a better way
            num_cpus: num_cpus(),
            cpu_usage: AtomicU32::new(0),
            mem_usage: AtomicU64::new(0),
        }
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
    let (shard_id, shard_latency) = {
        // Shard 0 is the shard used for DMs
        let shard_id = ctx.guild_id().map(|g| g.shard_id(ctx.cache())).unwrap_or(0);
        let shard_runners = ctx.framework().shard_manager.runners.lock().await;
        let opt_shard = shard_runners.get(&serenity::ShardId(shard_id)).clone();

        (
            shard_id,
            opt_shard.and_then(|s| s.latency).map(|l| l.as_millis()),
        )
    };

    let fronter_systems = fronters::db::get_system_count(&ctx.data().db).await?;

    let stats = &ctx.data().stats;
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
        .field("Current shard", format!("Shard #{}", shard_id), true)
        .field(
            "Latency",
            format!(
                "API: {} ms, Shard: {}",
                api_latency,
                shard_latency.map_or("N/A".into(), |f| format!(
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

pub(crate) fn commands() -> Vec<poise::Command<Arc<Data>, Error>> {
    vec![stats()]
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
