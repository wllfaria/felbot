#[macro_export]
macro_rules! env {
    ($name:expr) => {
        dotenvy::var($name).expect(&format!("missing required environment variable: {}", $name))
    };
}

#[derive(Debug, Clone)]
pub struct Env {
    pub port: String,
    pub database_url: String,
    pub account_link_url: String,

    pub discord_token: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_oauth_redirect: String,
    pub discord_allowed_roles: Vec<String>,
    pub telegram_group_id: String,
}

impl Env {
    pub fn new() -> Self {
        let port = env!("PORT");
        let database_url = env!("DATABASE_URL");
        let account_link_url = env!("ACCOUNT_LINK_URL");

        let discord_token = env!("DISCORD_TOKEN");
        let discord_client_id = env!("DISCORD_CLIENT_ID");
        let discord_client_secret = env!("DISCORD_CLIENT_SECRET");
        let discord_oauth_redirect = env!("DISCORD_OAUTH_REDIRECT");

        let discord_allowed_roles = env!("DISCORD_ALLOWED_ROLES")
            .split(" ")
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        let telegram_group_id = env!("TELEGRAM_GROUP_ID");

        Self {
            port,
            database_url,
            account_link_url,
            discord_token,
            discord_client_id,
            discord_client_secret,
            discord_oauth_redirect,
            discord_allowed_roles,
            telegram_group_id,
        }
    }
}
