use teloxide::Bot;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Requester, ResponseResult};
use teloxide::types::{ChatId, UserId};

use crate::telegram::replies::make_invite_message;

pub async fn invite_user(bot: &Bot, telegram_id: i64, group_id: i64) {
    tracing::info!(telegram_id = telegram_id, "Processing invite user action");

    if let Err(e) = send_invite_to_user(bot, UserId(telegram_id as u64), group_id).await {
        tracing::error!(
            error = %e,
            telegram_id = telegram_id,
            "Failed to send invite to user"
        );
    } else {
        tracing::info!(
            telegram_id = telegram_id,
            "Invite action completed successfully"
        );
    }
}

#[tracing::instrument(skip_all, fields(user_id = user_id.0))]
async fn send_invite_to_user(bot: &Bot, user_id: UserId, group_id: i64) -> ResponseResult<()> {
    tracing::info!("Creating invite link for user");
    let group_id = ChatId(group_id);

    let invite = bot.create_chat_invite_link(group_id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to create chat invite link");
        e
    })?;

    let link = invite.invite_link;
    tracing::debug!(invite_link = %link, "Invite link created");

    bot.send_message(user_id, make_invite_message(link))
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    tracing::info!("Invite message sent successfully");
    Ok(())
}
