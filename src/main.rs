mod api;
mod discord;
mod telegram;

#[macro_export]
macro_rules! env {
    ($name:expr) => {
        dotenvy::var($name).expect(&format!("missing required environment variable: {}", $name))
    };
}

#[tokio::main]
async fn main() {
    let mut discord_handle = tokio::spawn(discord::init());
    let mut telegram_handle = tokio::spawn(telegram::init());
    let mut api_handle = tokio::spawn(api::init());

    tokio::select! {
        result = &mut discord_handle => {
            println!("Discord bot finished: {result:?}");
            telegram_handle.abort();
            api_handle.abort();
        }
        result = &mut telegram_handle => {
            println!("Telegram bot finished: {result:?}");
            api_handle.abort();
            discord_handle.abort();
        }
        result = &mut api_handle => {
            println!("API finished: {result:?}");
            discord_handle.abort();
            telegram_handle.abort();
        }
        _ = tokio::signal::ctrl_c() => {
            discord_handle.abort();
            api_handle.abort();
            telegram_handle.abort();
        }
    }
}
