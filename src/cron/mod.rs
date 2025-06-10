use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use sqlx::PgPool;
use serenity::http::Http;
use serenity::model::id::{GuildId, UserId};

use crate::env::Env;
use crate::messages::TelegramAction;
use crate::api::models::user_links::UserLink;

pub async fn init(env: Arc<Env>, telegram_sender: UnboundedSender<TelegramAction>, pool: PgPool) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(24 * 60 * 60)); // 24 hours
    
    loop {
        interval.tick().await;
        if let Err(e) = check_user_roles(env.clone(), telegram_sender.clone(), pool.clone()).await {
            tracing::error!("Role check failed: {}", e);
        }
    }
}

async fn check_user_roles(
    env: Arc<Env>, 
    telegram_sender: UnboundedSender<TelegramAction>,
    pool: PgPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("Starting role verification check for all users");
    
    // Get all linked users from database
    let mut tx = pool.acquire().await?;
    let users = UserLink::get_all_users(tx.as_mut()).await?;
    
    // Create Discord HTTP client
    let discord_client = Arc::new(Http::new(&env.discord_token));
    
    let guild_id = GuildId::new(env.discord_guild_id.parse()?);
    
    tracing::info!("Checking roles for {} users", users.len());
    
    // Check each user's roles
    for user in users {
        match check_user_has_allowed_roles(&discord_client, &env, &user, guild_id).await {
            Ok(has_roles) => {
                if !has_roles {
                    tracing::info!("User {} no longer has required roles, removing from group", user.discord_username);
                    
                    // Send remove action to Telegram
                    let action = TelegramAction::RemoveUser {
                        telegram_id: user.telegram_id.clone(),
                        discord_username: user.discord_username.clone(),
                        reason: "No longer has required Discord roles".to_string(),
                    };
                    
                    if let Err(e) = telegram_sender.send(action) {
                        tracing::error!("Failed to send remove action for user {}: {}", user.discord_id, e);
                    }
                    
                    // Remove user link from database
                    if let Err(e) = UserLink::delete_by_discord_id(tx.as_mut(), &user.discord_id).await {
                        tracing::error!("Failed to delete user link for {}: {}", user.discord_id, e);
                    }
                } else {
                    tracing::debug!("User {} still has required roles", user.discord_username);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to check roles for user {}: {}", user.discord_id, e);
            }
        }
    }
    
    tracing::info!("Role verification check completed");
    Ok(())
}

async fn check_user_has_allowed_roles(
    http: &Http,
    env: &Env,
    user: &UserLink,
    guild_id: GuildId,
) -> Result<bool, serenity::Error> {
    let user_id = UserId::new(user.discord_id.parse().map_err(|e| {
        serenity::Error::Other(&format!("Invalid user ID: {}", e))
    })?);
    
    // Get user's roles in the guild
    let member = http.get_member(guild_id, user_id).await?;
    
    // Check if user has any of the allowed roles
    let has_allowed_role = member.roles.iter().any(|role_id| {
        env.discord_allowed_roles.contains(&role_id.to_string())
    });
    
    Ok(has_allowed_role)
}
