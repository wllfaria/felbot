use std::sync::Arc;

use poise::serenity_prelude::Result;
use poise::serenity_prelude::http::Http;
use poise::serenity_prelude::model::id::{GuildId, UserId};
use sqlx::{PgConnection, PgPool};
use tokio::sync::mpsc::UnboundedSender;

use crate::api::models::user_links::UserLink;
use crate::env::Env;
use crate::messages::TelegramAction;

pub async fn init(env: Arc<Env>, pool: PgPool, telegram_sender: UnboundedSender<TelegramAction>) {
    const ONE_DAY_IN_SECS: u64 = 24 * 60 * 60;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(ONE_DAY_IN_SECS));

    loop {
        interval.tick().await;

        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(_) => continue,
        };

        if let Err(e) = check_user_roles(env.clone(), tx.as_mut(), telegram_sender.clone()).await {
            if let Err(e) = tx.rollback().await {
                tracing::error!("failed to rollback transaction: {e}");
            };
            tracing::error!("Role check failed: {}", e);
            continue;
        }

        if let Err(e) = tx.commit().await {
            tracing::error!("failed to commit cron job transaction: {e}");
        };
    }
}

#[tracing::instrument(skip_all)]
async fn check_user_roles(
    env: Arc<Env>,
    conn: &mut PgConnection,
    telegram_sender: UnboundedSender<TelegramAction>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Starting role verification check for all users");

    let discord_client = Http::new(&env.discord_token);
    let guild_id = GuildId::new(env.discord_guild_id);
    let users = UserLink::get_all_users(conn).await?;

    for user in users {
        let span = tracing::info_span!("user({})", user.discord_id);
        let _guard = span.enter();

        tracing::info!("checking for user roles");

        let Ok(has_roles) = has_allowed_roles(&discord_client, &env, &user, guild_id).await else {
            tracing::warn!("Failed to check roles for user");
            continue;
        };

        if !has_roles {
            let send_result = telegram_sender.send(TelegramAction::RemoveUser {
                telegram_id: user.telegram_id,
            });

            if let Err(e) = send_result {
                tracing::error!("Failed to send remove action: {e}");
                continue;
            }

            if let Err(e) = UserLink::delete_by_discord_id(conn, user.discord_id).await {
                tracing::error!("failed to delete user link from database: {e}");
            }
        }
    }

    tracing::info!("Role verification check completed");
    Ok(())
}

async fn has_allowed_roles(
    http: &Http,
    env: &Env,
    user: &UserLink,
    guild_id: GuildId,
) -> Result<bool> {
    let has_allowed_role = http
        .get_member(guild_id, UserId::new(user.discord_id as u64))
        .await?
        .roles
        .iter()
        .any(|id| env.discord_allowed_roles.contains(&id.get()));

    Ok(has_allowed_role)
}
