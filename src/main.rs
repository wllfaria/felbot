use std::sync::Arc;
use std::time::Duration;

use env::Env;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[macro_use]
mod env;

mod api;
mod cron;
mod database;
mod discord;
mod error;
mod messages;
mod telegram;
mod templates;
mod utils;

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::from("info");
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true);

    let use_json = env!("LOG_FORMAT") == "json";

    if use_json {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer.json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer.pretty())
            .init();
    }
}

#[tokio::main]
async fn main() {
    init_tracing();

    let env = Arc::new(Env::new());
    tracing::info!(port = %env.port, "Application starting");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&env.database_url)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Database connection established");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    tracing::info!("Database migrations completed");
    tracing::info!("Starting application services");

    let (telegram_sender, telegram_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (cron_sender, cron_receiver) = tokio::sync::mpsc::unbounded_channel();

    let mut telegram_handle = tokio::spawn(telegram::init(env.clone(), telegram_receiver));
    let mut discord_handle = tokio::spawn(discord::init(env.clone()));

    let mut cron_handle = tokio::spawn(cron::init(
        env.clone(),
        pool.clone(),
        cron_receiver,
        telegram_sender.clone(),
    ));

    let mut api_handle = tokio::spawn(api::init(
        env.clone(),
        pool.clone(),
        telegram_sender.clone(),
        cron_sender,
    ));

    tracing::info!("All services started successfully");

    tokio::select! {
        result = &mut discord_handle => {
            tracing::error!(?result, "Discord service exited unexpectedly");
            tracing::info!("Shutting down other services");
            telegram_handle.abort();
            api_handle.abort();
            cron_handle.abort();
        }
        result = &mut telegram_handle => {
            tracing::error!(?result, "Telegram service exited unexpectedly");
            tracing::info!("Shutting down other services");
            api_handle.abort();
            discord_handle.abort();
            cron_handle.abort();
        }
        result = &mut api_handle => {
            tracing::error!(?result, "API service exited unexpectedly");
            tracing::info!("Shutting down other services");
            discord_handle.abort();
            telegram_handle.abort();
            cron_handle.abort();
        }
        result = &mut cron_handle => {
            tracing::error!(?result, "Cron service exited unexpectedly");
            tracing::info!("Shutting down other services");
            discord_handle.abort();
            telegram_handle.abort();
            api_handle.abort();
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received shutdown signal, gracefully shutting down");
            discord_handle.abort();
            api_handle.abort();
            telegram_handle.abort();
            cron_handle.abort();
        }
    }

    tracing::info!("Application shutdown complete");
}
