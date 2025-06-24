mod invite_user;
mod kick_user;
mod replies;

use std::str::FromStr;
use std::sync::Arc;

use invite_user::invite_user;
use kick_user::remove_user;
use replies::{make_link_message, make_start_message};
use sqlx::PgPool;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::database::models::telegram_groups::TelegramGroup;
use crate::env::Env;
use crate::messages::TelegramAction;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
    Felps,
    Carol,
}

#[derive(Debug)]
pub enum Groups {
    Felps,
    Carol,
}

impl FromStr for Groups {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "felps" => Ok(Groups::Felps),
            "carol" => Ok(Groups::Carol),
            _ => Err(format!("Invalid group: {s}")),
        }
    }
}

impl std::fmt::Display for Groups {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Groups::Felps => write!(f, "felps"),
            Groups::Carol => write!(f, "carol"),
        }
    }
}

pub async fn init(env: Arc<Env>, pool: PgPool, receiver: UnboundedReceiver<TelegramAction>) {
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
        let pool = pool.clone();
        async move { answer(env, pool, bot, msg, cmd).await }
    })
    .await;
}

#[tracing::instrument(skip(env, bot, cmd), fields(
    chat_id = msg.chat.id.0,
    user_id = msg.from.as_ref().map(|u| u.id.0),
    username = msg.from.as_ref().and_then(|u| u.username.as_deref())
))]
async fn answer(
    env: Arc<Env>,
    pool: PgPool,
    bot: Bot,
    msg: Message,
    cmd: Command,
) -> ResponseResult<()> {
    tracing::info!("Processing Telegram command");

    let groups = TelegramGroup::get(&mut pool.acquire().await.unwrap())
        .await
        .unwrap();

    if groups
        .iter()
        .any(|group| group.telegram_group_id == msg.chat.id.0)
    {
        tracing::debug!("Ignoring /start command in any group chat");
        return Ok(());
    }

    match cmd {
        Command::Start => handle_start_command(bot, msg).await?,
        Command::Felps => handle_link_flow(&env, bot, msg, Groups::Felps).await?,
        Command::Carol => handle_link_flow(&env, bot, msg, Groups::Carol).await?,
    };

    Ok(())
}

async fn process_telegram_actions(
    env: Arc<Env>,
    bot: Bot,
    mut receiver: UnboundedReceiver<TelegramAction>,
) {
    while let Some(action) = receiver.recv().await {
        let span = tracing::info_span!(
            "telegram_action",
            action_type = match &action {
                TelegramAction::InviteUser { .. } => "invite",
                TelegramAction::RemoveUser { .. } => "remove",
            },
        );
        let _guard = span.enter();

        match action {
            TelegramAction::InviteUser { id, group_id } => invite_user(&bot, id, group_id).await,
            TelegramAction::RemoveUser { id, group_id } => {
                remove_user(&env, &bot, id, group_id).await
            }
        }
    }
}

async fn handle_start_command(bot: Bot, msg: Message) -> ResponseResult<()> {
    let Some(user) = msg.from else {
        tracing::error!("Message has no user information");
        return Ok(());
    };

    tracing::info!(
        user_id = user.id.0,
        username = user.username.as_deref().unwrap_or("none"),
        "Sending welcome message to user"
    );

    bot.send_message(msg.chat.id, make_start_message(user))
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    tracing::info!("Welcome message sent successfully");

    Ok(())
}

async fn handle_link_flow(env: &Env, bot: Bot, msg: Message, group: Groups) -> ResponseResult<()> {
    let Some(user) = msg.from else {
        tracing::error!("Message has no user information");
        return Ok(());
    };

    tracing::info!(
        user_id = user.id.0,
        username = user.username.as_deref().unwrap_or("none"),
        group = %group,
        "Starting account linking flow",
    );

    bot.send_message(msg.chat.id, make_link_message(env, user, group))
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    tracing::info!("account linking message sent successfully");

    Ok(())
}
