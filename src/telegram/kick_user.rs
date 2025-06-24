use teloxide::Bot;
use teloxide::prelude::{Requester, ResponseResult};
use teloxide::types::{ChatId, UserId};

use crate::env::Env;

pub async fn remove_user(env: &Env, bot: &Bot, telegram_id: i64, group_id: i64) {
    tracing::info!(telegram_id = telegram_id, "Processing remove user action");

    if let Err(e) = kick_user(&env, &bot, UserId(telegram_id as u64), group_id).await {
        tracing::error!(
            error = %e,
            telegram_id = telegram_id,
            "Failed to remove user"
        );
    } else {
        tracing::info!(
            telegram_id = telegram_id,
            "Remove action completed successfully"
        );
    }
}
#[tracing::instrument(skip_all, fields(user_id = user_id.0))]
async fn kick_user(env: &Env, bot: &Bot, user_id: UserId, group_id: i64) -> ResponseResult<()> {
    tracing::info!("Removing user from Telegram group");

    let group_id = ChatId(group_id);
    bot.ban_chat_member(group_id, user_id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to ban user from group");
        e
    })?;

    tracing::debug!("User banned, now unbanning to allow re-entry");

    bot.unban_chat_member(group_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to unban user (user will remain banned)");
            e
        })?;

    tracing::info!("User successfully removed from group");
    Ok(())
}
