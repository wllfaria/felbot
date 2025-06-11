use std::sync::Arc;
use std::time::Instant;

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

    tracing::info!(
        interval_hours = ONE_DAY_IN_SECS / 3600,
        "Cron service initialized, starting role verification scheduler"
    );

    let mut cycle_count = 0u64;

    loop {
        interval.tick().await;
        cycle_count += 1;

        let cycle_start = Instant::now();
        tracing::info!(cycle = cycle_count, "Starting role verification cycle");

        let mut tx = match pool.begin().await {
            Ok(tx) => {
                tracing::debug!("Database transaction started");
                tx
            }
            Err(e) => {
                tracing::error!(error = %e, cycle = cycle_count, "Failed to start database transaction, skipping cycle");
                continue;
            }
        };

        match check_user_roles(env.clone(), tx.as_mut(), telegram_sender.clone()).await {
            Ok(stats) => {
                if let Err(e) = tx.commit().await {
                    tracing::error!(error = %e, cycle = cycle_count, "Failed to commit transaction");
                } else {
                    let cycle_duration = cycle_start.elapsed();
                    tracing::info!(
                        cycle = cycle_count,
                        duration_ms = cycle_duration.as_millis(),
                        users_checked = stats.users_checked,
                        users_removed = stats.users_removed,
                        users_failed = stats.users_failed,
                        "Role verification cycle completed successfully"
                    );
                }
            }
            Err(e) => {
                if let Err(rollback_err) = tx.rollback().await {
                    tracing::error!(error = %rollback_err, cycle = cycle_count, "Failed to rollback transaction");
                }
                let cycle_duration = cycle_start.elapsed();
                tracing::error!(
                    error = %e,
                    cycle = cycle_count,
                    duration_ms = cycle_duration.as_millis(),
                    "Role verification cycle failed"
                );
            }
        }
    }
}

#[derive(Debug, Default)]
struct VerificationStats {
    users_checked: u32,
    users_removed: u32,
    users_failed: u32,
}

#[tracing::instrument(skip_all)]
async fn check_user_roles(
    env: Arc<Env>,
    conn: &mut PgConnection,
    telegram_sender: UnboundedSender<TelegramAction>,
) -> Result<VerificationStats, Box<dyn std::error::Error + Send + Sync>> {
    let start_time = Instant::now();
    let mut stats = VerificationStats::default();

    let discord_client = Http::new(&env.discord_token);
    let guild_id = GuildId::new(env.discord_guild_id);

    let users = UserLink::get_all_users(conn).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch users from database");
        e
    })?;

    let total_users = users.len();
    tracing::info!(
        total_users = total_users,
        "Starting role verification for all users"
    );

    for (index, user) in users.into_iter().enumerate() {
        let user_start = Instant::now();
        let span = tracing::info_span!(
            "user_verification",
            discord_id = user.discord_id,
            telegram_id = user.telegram_id,
            user_index = index + 1,
            total_users = total_users
        );
        let _guard = span.enter();

        stats.users_checked += 1;

        tracing::debug!("Checking user roles");

        match has_allowed_roles(&discord_client, &env, &user, guild_id).await {
            Ok(has_roles) => {
                let check_duration = user_start.elapsed();

                if has_roles {
                    tracing::debug!(
                        duration_ms = check_duration.as_millis(),
                        "User has valid roles"
                    );
                    continue;
                }

                tracing::info!(
                    duration_ms = check_duration.as_millis(),
                    "User no longer has required roles, removing"
                );

                let send_result = telegram_sender.send(TelegramAction::RemoveUser {
                    telegram_id: user.telegram_id,
                });

                if let Err(e) = send_result {
                    tracing::error!(error = %e, "Failed to send telegram remove action");
                    stats.users_failed += 1;
                    continue;
                }

                if let Err(e) = UserLink::delete_by_discord_id(conn, user.discord_id).await {
                    tracing::error!(error = %e, "Failed to delete user link from database");
                    stats.users_failed += 1;
                    continue;
                }

                stats.users_removed += 1;
                tracing::info!("User successfully removed from system");
            }
            Err(e) => {
                let check_duration = user_start.elapsed();
                tracing::warn!(
                    error = %e,
                    duration_ms = check_duration.as_millis(),
                    "Failed to check user roles, skipping user"
                );
                stats.users_failed += 1;
            }
        }
    }

    let total_duration = start_time.elapsed();
    tracing::info!(
        duration_ms = total_duration.as_millis(),
        users_checked = stats.users_checked,
        users_removed = stats.users_removed,
        users_failed = stats.users_failed,
        "Role verification check completed"
    );

    Ok(stats)
}

#[tracing::instrument(skip(http), fields(discord_id = user.discord_id))]
async fn has_allowed_roles(
    http: &Http,
    env: &Env,
    user: &UserLink,
    guild_id: GuildId,
) -> Result<bool> {
    let user_id = UserId::new(user.discord_id as u64);

    tracing::debug!("Fetching Discord member information");

    let member = http.get_member(guild_id, user_id).await.map_err(|e| {
        tracing::debug!(error = %e, "Failed to fetch Discord member (user may have left server)");
        e
    })?;

    let user_roles: Vec<u64> = member.roles.iter().map(|role| role.get()).collect();
    let allowed_roles = &env.discord_allowed_roles;

    let has_allowed_role = user_roles
        .iter()
        .any(|role_id| allowed_roles.contains(role_id));

    tracing::debug!(
        user_roles = ?user_roles,
        allowed_roles = ?allowed_roles,
        has_allowed_role = has_allowed_role,
        "Role verification completed"
    );

    Ok(has_allowed_role)
}
