use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{self as serenity};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use tracing::{debug, log::LevelFilter};

use crate::modules::fronters;
use crate::types::Data;

mod config;
mod modules;
mod task;
mod types;
mod util;

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

        // registere module commands
        commands: vec![
            modules::fronters::commands(),
            modules::roles::commands(),
            modules::pk::commands(),
        ]
        .into_iter()
        .flatten()
        .collect(),
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .options(options)
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                let data = Arc::new(Data::new(db));

                // register module tasks
                fronters::start_tasks(ctx.to_owned(), data.clone());

                Ok(data.clone())
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(config.bot.token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
