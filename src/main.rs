use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{self as serenity};
use sqlx::postgres::PgPoolOptions;
use tokio::time::MissedTickBehavior;

use crate::types::Data;

mod config;
mod modules;
mod types;
mod util;

fn start_long_running_tasks(ctx: serenity::Context, data: Arc<Data>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));

        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            interval.tick().await;

            println!("long_running_tasks::tick()");

            if let Err(err) = modules::fronters::tasks::update_fronters(&ctx, data.clone()).await {
                println!("error running update_fronters(): {}", err);
            }
        }
    });
}

#[tokio::main]
async fn main() {
    let config = config::load_config().expect("error loading envfile");
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(config.db.url.as_str())
        .await
        .expect("error connecting to db");

    let intents = serenity::GatewayIntents::all();
    let options = poise::FrameworkOptions {
        pre_command: |ctx| {
            Box::pin(async move {
                println!("executing command {}...", ctx.invoked_command_name());
            })
        },
        post_command: |ctx| {
            Box::pin(async move {
                println!("finished executing command {}", ctx.invoked_command_name());
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
