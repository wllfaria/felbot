use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::types::User;
use teloxide::utils::command::BotCommands;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::env::Env;

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

#[derive(Debug)]
pub enum TelegramAction {
    InviteUser {
        telegram_id: String,
        discord_id: String,
    },
    RemoveUser {
        telegram_id: String,
        discord_username: String,
    },
}

async fn answer(env: Arc<Env>, bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    let telegram_group_id = env
        .telegram_group_id
        .parse::<i64>()
        .expect("TELEGRAM_GROUP_ID must be a number");

    match cmd {
        Command::Start => {
            // Only answer messages outside of the group
            if msg.chat.id.0 == telegram_group_id {
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

async fn send_invite_to_user(env: &Env, bot: &Bot, user_chat_id: ChatId) -> ResponseResult<()> {
    let invite = bot
        .create_chat_invite_link(env.telegram_group_id.clone())
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

async fn process_telegram_actions(
    env: Arc<Env>,
    bot: Bot,
    mut receiver: UnboundedReceiver<TelegramAction>,
) {
    while let Some(action) = receiver.recv().await {
        match action {
            TelegramAction::InviteUser { telegram_id, .. } => {
                let chat_id = ChatId(telegram_id.parse::<i64>().unwrap());
                if let Err(e) = send_invite_to_user(&env, &bot, chat_id).await {
                    tracing::error!("Failed to send invite to user {telegram_id}: {e}");
                };
            }
            TelegramAction::RemoveUser { .. } => todo!(),
        }
    }
}
