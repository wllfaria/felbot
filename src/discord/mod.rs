mod commands;
mod error;
mod handlers;
mod permissions;

use std::sync::Arc;

use commands::{channels, roles, telegram, verify_members};
use error::{Error, Result};
use poise::serenity_prelude::{self as serenity};
use tokio::sync::mpsc::UnboundedSender;

use crate::env::Env;
use crate::messages::CronAction;

pub struct Data {
    pool: sqlx::PgPool,
    cron_sender: UnboundedSender<CronAction>,
}
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn init(env: Arc<Env>, pool: sqlx::PgPool, cron_sender: UnboundedSender<CronAction>) {
    tracing::info!("Initializing Discord service");

    let framework = create_framework(pool, cron_sender).await;
    let intents = serenity::GatewayIntents::non_privileged();

    let mut client = serenity::ClientBuilder::new(&env.discord_token, intents)
        .framework(framework)
        .await
        .expect("Failed to create Discord client");

    tracing::info!("Discord client created, starting connection");

    if let Err(e) = client.start().await {
        tracing::error!(error = %e, "Discord client failed");
    }
}

async fn create_framework(
    pool: sqlx::PgPool,
    cron_sender: UnboundedSender<CronAction>,
) -> poise::Framework<Data, Error> {
    let options = poise::FrameworkOptions {
        commands: vec![telegram(), channels(), roles(), verify_members()],
        pre_command: |ctx| {
            Box::pin(async move {
                tracing::debug!(
                    user = %ctx.author().name,
                    command = %ctx.command().qualified_name,
                    "Command executed"
                );
            })
        },
        on_error: |error| Box::pin(handlers::error_handler(error)),
        ..Default::default()
    };

    poise::Framework::builder()
        .options(options)
        .setup(move |ctx, ready, framework| {
            Box::pin(setup(ctx, ready, framework, pool, cron_sender))
        })
        .build()
}

async fn setup(
    ctx: &serenity::Context,
    ready: &serenity::Ready,
    framework: &poise::Framework<Data, Error>,
    pool: sqlx::PgPool,
    cron_sender: UnboundedSender<CronAction>,
) -> Result<Data> {
    tracing::info!(
        bot_username = %ready.user.name,
        bot_id = %ready.user.id,
        guild_count = ready.guilds.len(),
        "Discord bot connected and ready"
    );

    poise::builtins::register_globally(ctx, &framework.options().commands)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to register Discord commands globally");
            e
        })?;

    tracing::info!(
        command_count = framework.options().commands.len(),
        "Discord commands registered globally"
    );

    Ok(Data { pool, cron_sender })
}
