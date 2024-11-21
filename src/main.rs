use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{self as serenity};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use tokio::time::MissedTickBehavior;
use tracing::{debug, error, log::LevelFilter};

use crate::types::Data;

mod config;
mod modules;
mod types;
mod util;

fn start_long_running_tasks(ctx: serenity::Context, data: Arc<Data>) {
    tokio::spawn(async move {
        let mut update_fronters_interval = tokio::time::interval(Duration::from_secs(60));
        update_fronters_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = update_fronters_interval.tick() => {
                    debug!("long_running_tasks::update_fronters_interval.tick()");
                    if let Err(err) = modules::fronters::tasks::update_fronters(&ctx, data.clone()).await {
                        error!("error running update_fronters(): {}", err);
                    }
                }
            }
        }
    });
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = config::load_config().expect("error loading envfile");
    let connect_opts = config
        .db
        .url
        .parse::<PgConnectOptions>()
        .expect(&format!("couldn't parse db url: {}", config.db.url))
        .log_statements(LevelFilter::Trace)
        .log_slow_statements(LevelFilter::Warn, Duration::from_secs(5));

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(connect_opts)
        .await
        .expect("error connecting to db");

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("error running migrations");

    let intents = serenity::GatewayIntents::all();
    let options = poise::FrameworkOptions {
        pre_command: |ctx| {
            Box::pin(async move {
                debug!("executing command /{}...", ctx.invoked_command_name());
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                debug!("finished executing command /{}", ctx.invoked_command_name());
            })
        },
        commands: vec![
            modules::roles::update_member_roles(),
            modules::fronters::commands::update_fronters(),
            modules::fronters::commands::setup_fronters(),
        ],
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .options(options)
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let data = Arc::new(Data::new(db));

                start_long_running_tasks(ctx.to_owned(), data.clone());

                Ok(data.clone())
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(config.bot.token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
