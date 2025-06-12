use std::sync::Arc;
use std::time::Instant;

use poise::serenity_prelude::{GuildId, Http, UserId};
use sqlx::{Connection, PgConnection, PgPool};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::api::models::user_links::UserLink;
use crate::env::Env;
use crate::error::{AppError, Result};
use crate::messages::{CronAction, TelegramAction};

pub async fn init(
    env: Arc<Env>,
    pool: PgPool,
    cron_receiver: UnboundedReceiver<CronAction>,
    telegram_sender: UnboundedSender<TelegramAction>,
) {
    tokio::spawn(manual_trigger_runner(
        env.clone(),
        pool.clone(),
        cron_receiver,
        telegram_sender.clone(),
    ));

    cron_job_runner(env, pool, telegram_sender).await;
}

async fn manual_trigger_runner(
    env: Arc<Env>,
    pool: PgPool,
    mut cron_receiver: UnboundedReceiver<CronAction>,
    telegram_sender: UnboundedSender<TelegramAction>,
) {
    while (cron_receiver.recv().await).is_some() {
        tracing::info!("executing manually triggered cron job");
        let Ok(mut conn) = pool.acquire().await else {
            tracing::error!("faield to acquire pool connection, skipping cron job");
            continue;
        };

        run_cron_job(env.clone(), conn.as_mut(), telegram_sender.clone()).await;
    }
}

async fn cron_job_runner(
    env: Arc<Env>,
    pool: PgPool,
    telegram_sender: UnboundedSender<TelegramAction>,
) {
    const ONE_DAY_IN_SECS: u64 = 24 * 60 * 60;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(ONE_DAY_IN_SECS));
    tracing::info!("Cron service initialized, starting role verification scheduler");

    loop {
        interval.tick().await;
        let Ok(mut conn) = pool.acquire().await else {
            tracing::error!("faield to acquire pool connection, skipping cron job");
            continue;
        };

        run_cron_job(env.clone(), conn.as_mut(), telegram_sender.clone()).await;
    }
}

async fn with_tx<F, T>(conn: &mut PgConnection, f: F) -> Result<T>
where
    F: AsyncFnOnce(&mut PgConnection) -> Result<T>,
{
    let mut tx = Connection::begin(conn).await?;
    let result = f(tx.as_mut()).await;

    match result {
        Ok(_) => tx.commit().await?,
        Err(_) => tx.rollback().await?,
    }

    result
}

async fn run_cron_job(
    env: Arc<Env>,
    pool: &mut PgConnection,
    telegram_sender: UnboundedSender<TelegramAction>,
) {
    let cycle_start = Instant::now();
    tracing::info!("Starting role verification cycle");

    let mut tx = match PgConnection::begin(pool).await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(error = %e, "Failed to start database transaction, skipping cycle");
            return;
        }
    };

    match check_roles(env.clone(), tx.as_mut(), telegram_sender.clone()).await {
        Ok(stats) => match tx.commit().await {
            Ok(_) => {
                let cycle_duration = cycle_start.elapsed();
                tracing::info!(
                    duration_ms = cycle_duration.as_millis(),
                    users_checked = stats.users_checked,
                    users_removed = stats.users_removed,
                    users_failed = stats.users_failed,
                    "Role verification cycle completed successfully"
                );
            }
            Err(e) => tracing::error!(error = %e, "Failed to commit transaction"),
        },
        Err(e) => match tx.rollback().await {
            Ok(_) => {
                let cycle_duration = cycle_start.elapsed();
                tracing::error!(
                    error = %e,
                    duration_ms = cycle_duration.as_millis(),
                    "Role verification cycle failed"
                );
            }
            Err(e) => tracing::error!(error = %e, "Failed to rollback transaction"),
        },
    }
}

#[derive(Debug, Default)]
struct VerificationStats {
    users_checked: u32,
    users_removed: u32,
    users_failed: u32,
}

#[tracing::instrument(skip_all)]
async fn check_roles(
    env: Arc<Env>,
    conn: &mut PgConnection,
    telegram_sender: UnboundedSender<TelegramAction>,
) -> Result<VerificationStats> {
    let start_time = Instant::now();
    let mut stats = VerificationStats::default();

    let discord_client = Http::new(&env.discord_token);
    let guild_id = GuildId::new(env.discord_guild_id);

    let users = match UserLink::get_all_users(conn).await {
        Ok(users) => users,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch users from database");
            return Err(AppError::Database(e));
        }
    };

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
                let duration_ms = check_duration.as_millis();

                if has_roles {
                    tracing::debug!(duration_ms = duration_ms, "User has valid roles");
                    continue;
                }

                tracing::info!(
                    duration_ms = duration_ms,
                    "User no longer has required roles"
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

#[tracing::instrument(skip(http, env), fields(discord_id = user.discord_id))]
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
