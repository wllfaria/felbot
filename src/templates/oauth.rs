use maud::{html, Markup};
use crate::templates::base_layout;

pub fn oauth_success_page(username: &str) -> Markup {
    let content = html! {
        div class="success" { "✅ Account Linked Successfully!" }
        p { "Discord account " strong { (username) } " has been linked to your Telegram account." }
        p class="info" { "You can now close this window and return to Telegram." }
        script {
            "setTimeout(() => window.close(), 3000);"
        }
    };
    
    base_layout("Account Linked Successfully", content)
}

pub fn oauth_error_page(error_message: &str) -> Markup {
    let content = html! {
        div class="error" { "❌ Error" }
        p class="message" { (error_message) }
        p { a href="javascript:history.back()" { "Go Back" } }
    };
    
    base_layout("Error", content)
}
