use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{self as serenity};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use tracing::{debug, info, log::LevelFilter};

use crate::types::Data;

mod config;
mod events;
mod modules;
mod task;
mod types;
mod util;

#[tokio::main]
async fn main() {
    // load env vars
    dotenvy::dotenv().expect("error loading env vars");

    // set-up logging
    tracing_subscriber::fmt::init();

    info!("Starting DMServ ...");
    info!(" [-] Version:  {}", env!("CARGO_PKG_VERSION"));
    info!(
        " [-] Commit:   {}{}",
        env!("VERGEN_GIT_SHA"),
        match env!("VERGEN_GIT_DIRTY") {
            "true" => "-dirty",
            _ => "",
        }
    );
    info!(" [-] Branch:   {}", env!("VERGEN_GIT_BRANCH"));
    info!(" [-] Built At: {}", env!("VERGEN_BUILD_TIMESTAMP"));

    let config = config::load_config().expect("error loading config from env");
    let connect_opts = config
        .db
        .url
        .parse::<PgConnectOptions>()
        .unwrap_or_else(|_| panic!("couldn't parse db url: {}", config.db.url))
        .log_statements(LevelFilter::Trace)
        .log_slow_statements(LevelFilter::Warn, Duration::from_secs(5));

    info!("connecting to database...");
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(connect_opts)
        .await
        .expect("error connecting to db");

    info!("running migrations...");
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

        // registere module commands
        commands: vec![
            modules::fronters::commands(),
            modules::roles::commands(),
            modules::pk::commands(),
            modules::stats::commands(),
        ]
        .into_iter()
        .flatten()
        .collect(),
        ..Default::default()
    };

    let data = Arc::new(Data::new(db));
    let handler = events::EventHandler { data: data.clone() };

    let framework = poise::Framework::builder()
        .options(options)
        // ran on initial connection, also only fires once, unlike FullEvent::Ready
        .setup(|ctx, data_about_bot, framework| {
            Box::pin(async move {
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
                // TODO: figure out if shard total can change during runtime,
                //       if so figure out how to handle it
                data.stats
                    .set_total_shards(data_about_bot.shard.unwrap().total);

                // register commands
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                // register module tasks
                modules::stats::start_tasks(ctx.to_owned(), data.clone());
                modules::fronters::start_tasks(ctx.to_owned(), data.clone());

                Ok(data.clone())
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(config.bot.token, intents)
        .event_handler(handler)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
