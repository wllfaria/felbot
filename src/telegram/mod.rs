use teloxide::prelude::*;
use teloxide::types::User;
use teloxide::utils::command::BotCommands;

pub async fn init() {
    let bot = Bot::from_env();
    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Start => {
            let welcome_message = make_help_message(msg.from.unwrap());
            bot.send_message(msg.chat.id, welcome_message)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?
        }
    };

    Ok(())
}

fn make_help_message(user: User) -> String {
    let link_base_url = dotenvy::var("ACCOUNT_LINK_URL")
        .expect("missing required environment variable: ACCOUNT_LINK_URL");

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
