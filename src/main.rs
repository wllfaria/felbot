use std::sync::Arc;
use std::time::Duration;

use env::Env;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod api;
mod cron;
mod discord;
mod env;
mod messages;
mod telegram;
mod templates;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let env = Arc::new(Env::new());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&env.database_url)
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    let (telegram_sender, telegram_receiver) = tokio::sync::mpsc::unbounded_channel();

    let mut telegram_handle = tokio::spawn(telegram::init(env.clone(), telegram_receiver));
    let mut discord_handle = tokio::spawn(discord::init(env.clone()));

    let mut cron_handle = tokio::spawn(cron::init(
        env.clone(),
        pool.clone(),
        telegram_sender.clone(),
    ));

    let mut api_handle = tokio::spawn(api::init(
        env.clone(),
        pool.clone(),
        telegram_sender.clone(),
    ));

    tokio::select! {
        result = &mut discord_handle => {
            println!("Discord bot finished: {result:?}");
            telegram_handle.abort();
            api_handle.abort();
            cron_handle.abort();
        }
        result = &mut telegram_handle => {
            println!("Telegram bot finished: {result:?}");
            api_handle.abort();
            discord_handle.abort();
            cron_handle.abort();
        }
        result = &mut api_handle => {
            println!("API finished: {result:?}");
            discord_handle.abort();
            telegram_handle.abort();
            cron_handle.abort();
        }
        result = &mut cron_handle => {
            println!("Cron job finished: {result:?}");
            discord_handle.abort();
            telegram_handle.abort();
            api_handle.abort();
        }
        _ = tokio::signal::ctrl_c() => {
            discord_handle.abort();
            api_handle.abort();
            telegram_handle.abort();
            cron_handle.abort();
        }
    }
}
