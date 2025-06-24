use std::sync::Arc;
use std::time::Instant;

use poise::serenity_prelude::{GuildId, Http, UserId};
use sqlx::{PgConnection, PgPool};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::database::models::allowed_guilds::AllowedGuild;
use crate::database::models::allowed_roles::AllowedRole;
use crate::database::models::telegram_groups::TelegramGroup;
use crate::database::models::user_links::UserLink;
use crate::env::Env;
use crate::error::{AppError, Result};
use crate::messages::{CronAction, TelegramAction};
use crate::utils::with_tx;

/// Configuration for role verification service
#[derive(Debug, Clone)]
pub struct RoleVerificationConfig {
    /// Delay between API calls to avoid rate limiting (in milliseconds)
    pub api_delay_ms: u64,
    /// How often to run the job automatically (in seconds)
    pub schedule_interval_secs: u64,
}

impl Default for RoleVerificationConfig {
    fn default() -> Self {
        Self {
            api_delay_ms: 250,
            schedule_interval_secs: 24 * 60 * 60,
        }
    }
}

#[derive(Debug, Clone)]
struct CronContext {
    env: Arc<Env>,
    pool: PgPool,
    telegram_sender: UnboundedSender<TelegramAction>,
    config: RoleVerificationConfig,
}

pub async fn init(
    env: Arc<Env>,
    pool: PgPool,
    cron_receiver: UnboundedReceiver<CronAction>,
    telegram_sender: UnboundedSender<TelegramAction>,
    config: RoleVerificationConfig,
) {
    let context = CronContext {
        env,
        pool,
        telegram_sender,
        config,
    };

    tokio::spawn(manual_role_verification_runner(
        context.clone(),
        cron_receiver,
    ));

    role_verification_runner(context).await;
}

async fn manual_role_verification_runner(
    ctx: CronContext,
    mut cron_receiver: UnboundedReceiver<CronAction>,
) {
    while (cron_receiver.recv().await).is_some() {
        tracing::info!("executing manually triggered cron job");
        let Ok(mut conn) = ctx.pool.acquire().await else {
            tracing::error!("failed to acquire pool connection, skipping cron job");
            continue;
        };

        run_role_verification_cycle(
            ctx.env.clone(),
            conn.as_mut(),
            ctx.telegram_sender.clone(),
            ctx.config.clone(),
        )
        .await;
    }
}

async fn role_verification_runner(ctx: CronContext) {
    let interval = tokio::time::Duration::from_secs(ctx.config.schedule_interval_secs);
    let mut scheduler = tokio::time::interval(interval);

    tracing::info!(
        interval_secs = ctx.config.schedule_interval_secs,
        "Role verification scheduler initialized"
    );

    loop {
        scheduler.tick().await;
        let Ok(mut conn) = ctx.pool.acquire().await else {
            tracing::error!("faield to acquire pool connection, skipping cron job");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            continue;
        };

        run_role_verification_cycle(
            ctx.env.clone(),
            conn.as_mut(),
            ctx.telegram_sender.clone(),
            ctx.config.clone(),
        )
        .await;
    }
}

async fn run_role_verification_cycle(
    env: Arc<Env>,
    pool: &mut PgConnection,
    telegram_sender: UnboundedSender<TelegramAction>,
    config: RoleVerificationConfig,
) {
    let cycle_start = Instant::now();
    tracing::info!("Starting role verification cycle");

    let stats = with_tx(pool, async |tx| {
        verify_all_guilds_user_roles(env.clone(), tx, telegram_sender, config).await
    })
    .await;

    let cycle_duration = cycle_start.elapsed();

    match stats {
        Ok(stats) => tracing::info!(
            duration_ms = cycle_duration.as_millis(),
            users_checked = stats.users_checked,
            users_removed = stats.users_removed,
            users_failed = stats.users_failed,
            "Role verification cycle completed successfully"
        ),
        Err(e) => tracing::error!(
            error = %e,
            duration_ms = cycle_duration.as_millis(),
            "Role verification cycle failed"
        ),
    };
}

#[derive(Debug, Default)]
struct VerificationStats {
    users_checked: u32,
    users_removed: u32,
    users_failed: u32,
}

impl std::ops::AddAssign for VerificationStats {
    fn add_assign(&mut self, other: Self) {
        self.users_checked += other.users_checked;
        self.users_removed += other.users_removed;
        self.users_failed += other.users_failed;
    }
}

#[tracing::instrument(skip_all)]
async fn verify_all_guilds_user_roles(
    env: Arc<Env>,
    conn: &mut PgConnection,
    telegram_sender: UnboundedSender<TelegramAction>,
    config: RoleVerificationConfig,
) -> Result<VerificationStats> {
    let mut stats = VerificationStats::default();

    let discord_client = Http::new(&env.discord_token);

    let allowed_guilds = AllowedGuild::get_guilds(conn).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch allowed guilds from database");
        AppError::Database(e)
    })?;

    for guild in allowed_guilds {
        let guild_stats = verify_guild_user_roles(
            &discord_client,
            conn,
            telegram_sender.clone(),
            guild,
            config.api_delay_ms,
        )
        .await?;

        stats += guild_stats;
    }

    Ok(stats)
}

async fn verify_guild_user_roles(
    discord_client: &Http,
    conn: &mut PgConnection,
    telegram_sender: UnboundedSender<TelegramAction>,
    guild: AllowedGuild,
    api_delay_ms: u64,
) -> Result<VerificationStats> {
    let start_time = Instant::now();
    let guild_id = GuildId::new(guild.guild_id as u64);
    let mut stats = VerificationStats::default();

    let allowed_roles = AllowedRole::get_guild_role_ids(conn, guild.id).await?;

    if allowed_roles.is_empty() {
        tracing::warn!("No allowed roles found in database, skipping role verification");
        return Ok(stats);
    }

    let users = UserLink::get_all_guild_users(conn).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch users from database");
        AppError::Database(e)
    })?;

    verify_users_in_guild(
        discord_client,
        conn,
        telegram_sender,
        guild_id,
        &allowed_roles,
        users,
        api_delay_ms,
        &mut stats,
    )
    .await?;

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

async fn verify_users_in_guild(
    discord_client: &Http,
    conn: &mut PgConnection,
    telegram_sender: UnboundedSender<TelegramAction>,
    guild_id: GuildId,
    allowed_roles: &[u64],
    users: Vec<UserLink>,
    api_delay_ms: u64,
    stats: &mut VerificationStats,
) -> Result<()> {
    let total_users = users.len();

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

        match user_has_required_roles(discord_client, allowed_roles, guild_id, &user).await {
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

                // We send a message to Telegram first to kick the user before removing from DB
                // This ensures we don't lose track of who to remove if the system crashes
                let send_result = telegram_sender.send(TelegramAction::RemoveUser {
                    id: user.telegram_id,
                    group_id: guild_id.get() as i64,
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

        if index < total_users - 1 && api_delay_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(api_delay_ms)).await;
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all, fields(discord_id = user.discord_id))]
async fn user_has_required_roles(
    http: &Http,
    allowed_roles: &[u64],
    guild_id: GuildId,
    user: &UserLink,
) -> Result<bool> {
    let user_id = UserId::new(user.discord_id as u64);

    tracing::debug!("Fetching Discord member information");

    let member = http.get_member(guild_id, user_id).await.map_err(|e| {
        tracing::debug!(error = %e, "Failed to fetch Discord member");
        e
    })?;

    let user_roles: Vec<u64> = member.roles.iter().map(|role| role.get()).collect();
    // User only needs one of the allowed roles to maintain access
    let has_allowed_role = user_roles
        .iter()
        .any(|role_id| allowed_roles.contains(role_id));

    Ok(has_allowed_role)
}
