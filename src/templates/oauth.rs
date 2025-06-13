use maud::{Markup, html};

use crate::templates::base_layout;

pub fn oauth_success_page(username: &str) -> Markup {
    let content = html! {
        div class="success" { "Account Linked" }
        p { "Your Discord account " strong { (username) } " has been successfully linked." }
        p class="info" { "You can close this window and return to Telegram." }
        script {
            "setTimeout(() => window.close(), 3000);"
        }
    };

    base_layout("Account Linked", content)
}

pub fn oauth_error_page(error_message: &str) -> Markup {
    let content = html! {
        div class="error" { "Error" }
        p class="message" { (error_message) }
    };

    base_layout("Error", content)
}
