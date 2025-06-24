use teloxide::types::User;

use super::Groups;
use crate::env::Env;

pub fn make_start_message(user: User) -> String {
    let username = user.username.unwrap_or(user.first_name);

    [
        &format!("<b>Oi @{username}, vou te ajudar a entrar no grupo que deseja</b>"),
        "",
        "Pra continuar, você precisa usar um dos comandos abaixo:",
        "",
        "/felps - Entrar no grupo do telegram do felps",
        "/carol - Entrar no grupo do telegram do carol",
        "",
        "Você pode clicar no comando na lista acima ou digitar o comando no chat!",
    ]
    .join("\n")
}

pub fn make_link_message(env: &Env, user: User, group: Groups) -> String {
    let link_base_url = &env.account_link_url;
    let user_id = user.id.0;
    let link_url = format!("{link_base_url}?telegram_id={user_id}&group={group}");

    let heading = match group {
        Groups::Felps => "<b>Legal, você já ta quase no grupo do felps</b>",
        Groups::Carol => "<b>Legal, você já ta quase no grupo da carol</b>",
    };

    [
        heading,
        "",
        "Só precisa vincular sua conta do telegram com sua conta do discord. Só clicar no link aqui em baixo e fazer login com o discord",
        "",
        &format!("<a href=\"{link_url}\">🔗 Linkar minha conta!</a>"),
    ].join("\n")
}

pub fn make_invite_message(link: String) -> String {
    [
        "<b>Oioi, falei que era facinho! aqui tá seu link de convite</b>",
        "",
        &format!("<a href=\"{link}\">Clique aqui pra entrar no grupo</a>"),
    ]
    .join("\n")
}
