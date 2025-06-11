use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::User;
use teloxide::utils::command::BotCommands;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::env::Env;
use crate::messages::TelegramAction;

pub async fn init(env: Arc<Env>, receiver: UnboundedReceiver<TelegramAction>) {
    tracing::info!("Initializing Telegram service");

    let bot = Bot::from_env();

    let new_bot = bot.clone();
    let new_env = env.clone();
    tokio::spawn(async move {
        tracing::info!("Starting Telegram action processor");
        process_telegram_actions(new_env, new_bot, receiver).await;
        tracing::warn!("Telegram action processor stopped");
    });

    tracing::info!("Starting Telegram command handler");
    Command::repl(bot, move |bot, msg, cmd| {
        let env = env.clone();
        async move { answer(env, bot, msg, cmd).await }
    })
    .await;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
}

#[tracing::instrument(skip(env, bot, cmd), fields(
    chat_id = msg.chat.id.0,
    user_id = msg.from.as_ref().map(|u| u.id.0),
    username = msg.from.as_ref().and_then(|u| u.username.as_deref())
))]
async fn answer(env: Arc<Env>, bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    tracing::info!("Processing Telegram command");

    match cmd {
        Command::Start => {
            if msg.chat.id.0 == env.telegram_group_id {
                tracing::debug!("Ignoring /start command in group chat");
                return Ok(());
            }

            let Some(user) = msg.from else {
                tracing::error!("Message has no user information");
                return Ok(());
            };

            tracing::info!(
                user_id = user.id.0,
                username = user.username.as_deref().unwrap_or("none"),
                "Sending welcome message to user"
            );

            let welcome_message = make_help_message(&env, user);
            bot.send_message(msg.chat.id, welcome_message)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to send welcome message");
                    e
                })?;

            tracing::info!("Welcome message sent successfully");
        }
    };

    Ok(())
}

fn make_help_message(env: &Env, user: User) -> String {
    let link_base_url = &env.account_link_url;
    let username = user.username.unwrap_or(user.first_name);
    let user_id = user.id.0;
    let link_url = format!("{link_base_url}?telegram_id={user_id}");

    [
        &format!("<b>Opa @{username}, vc já tá quase no grupo '-'</b>"),
        "",
        "Só precisa vincular sua conta do telegram com sua conta do discord. Só clicar no link aqui em baixo e fazer login com o discord",
        "",
        &format!("<a href=\"{link_url}\">🔗 Linkar minha conta!</a>"),
    ].join("\n")
}

#[tracing::instrument(skip(bot, env), fields(user_id = user_chat_id.0))]
async fn send_invite_to_user(env: &Env, bot: &Bot, user_chat_id: UserId) -> ResponseResult<()> {
    tracing::info!("Creating invite link for user");

    let invite = bot
        .create_chat_invite_link(env.telegram_group_id.to_string())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create chat invite link");
            e
        })?;

    let link = invite.invite_link;
    tracing::debug!(invite_link = %link, "Invite link created");

    let invite_message = [
        "<b>Oi! aqui tá seu link de convite</b>",
        "",
        &format!("<a href=\"{link}\">Clique aqui pra entrar no grupo</a>"),
    ]
    .join("\n");

    bot.send_message(user_chat_id, invite_message)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to send invite message");
            e
        })?;

    tracing::info!("Invite message sent successfully");
    Ok(())
}

#[tracing::instrument(skip(bot, env), fields(user_id = user_id.0, group_id = env.telegram_group_id))]
async fn kick_user(env: &Env, bot: &Bot, user_id: UserId) -> ResponseResult<()> {
    let group_id = ChatId(env.telegram_group_id);

    tracing::info!("Removing user from Telegram group");

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

async fn process_telegram_actions(
    env: Arc<Env>,
    bot: Bot,
    mut receiver: UnboundedReceiver<TelegramAction>,
) {
    let mut action_count = 0u64;

    while let Some(action) = receiver.recv().await {
        action_count += 1;

        let span = tracing::info_span!(
            "telegram_action",
            action_type = match &action {
                TelegramAction::InviteUser { .. } => "invite",
                TelegramAction::RemoveUser { .. } => "remove",
            },
            action_count = action_count
        );
        let _guard = span.enter();

        match action {
            TelegramAction::InviteUser { telegram_id } => {
                tracing::info!(telegram_id = telegram_id, "Processing invite user action");

                if let Err(e) = send_invite_to_user(&env, &bot, UserId(telegram_id as u64)).await {
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
            TelegramAction::RemoveUser { telegram_id } => {
                tracing::info!(telegram_id = telegram_id, "Processing remove user action");

                if let Err(e) = kick_user(&env, &bot, UserId(telegram_id as u64)).await {
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
        }
    }

    tracing::warn!(
        total_actions_processed = action_count,
        "Telegram action processor shutting down"
    );
}
