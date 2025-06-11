use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::User;
use teloxide::utils::command::BotCommands;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::env::Env;
use crate::messages::TelegramAction;

pub async fn init(env: Arc<Env>, receiver: UnboundedReceiver<TelegramAction>) {
    let bot = Bot::from_env();

    let new_bot = bot.clone();
    let new_env = env.clone();
    tokio::spawn(async move {
        process_telegram_actions(new_env, new_bot, receiver).await;
    });

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

async fn answer(env: Arc<Env>, bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Start => {
            // Only answer messages outside of the group
            if msg.chat.id.0 == env.telegram_group_id {
                return Ok(());
            }

            let welcome_message = make_help_message(&env, msg.from.unwrap());
            bot.send_message(msg.chat.id, welcome_message)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?
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

async fn send_invite_to_user(env: &Env, bot: &Bot, user_chat_id: UserId) -> ResponseResult<()> {
    let invite = bot
        .create_chat_invite_link(env.telegram_group_id.to_string())
        .await?;
    let link = invite.invite_link;

    let invite_message = [
        "<b>Oi! aqui tá seu link de convite</b>",
        "",
        &format!("<a href=\"{link}\">Clique aqui pra entrar no grupo</a>"),
    ]
    .join("\n");

    bot.send_message(user_chat_id, invite_message)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}

async fn kick_user(env: &Env, bot: &Bot, user_id: UserId) -> ResponseResult<()> {
    let group_id = ChatId(env.telegram_group_id);

    bot.ban_chat_member(group_id, user_id).await?;
    bot.unban_chat_member(group_id, user_id).await?;

    Ok(())
}

async fn process_telegram_actions(
    env: Arc<Env>,
    bot: Bot,
    mut receiver: UnboundedReceiver<TelegramAction>,
) {
    while let Some(action) = receiver.recv().await {
        match action {
            TelegramAction::InviteUser { telegram_id } => {
                if let Err(e) = send_invite_to_user(&env, &bot, UserId(telegram_id as u64)).await {
                    tracing::error!("Failed to send invite to user {telegram_id}: {e}");
                }
            }
            TelegramAction::RemoveUser { telegram_id } => {
                if let Err(e) = kick_user(&env, &bot, UserId(telegram_id as u64)).await {
                    tracing::error!("Failed to remove user {telegram_id}: {e}");
                }
            }
        }
    }
}
