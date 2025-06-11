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
    pub discord_guild_id: u64,
    pub discord_allowed_roles: Vec<u64>,
    pub telegram_group_id: i64,
}

impl Env {
    pub fn new() -> Self {
        tracing::debug!("Loading environment configuration");

        let port = env!("PORT");
        let database_url = env!("DATABASE_URL");
        let account_link_url = env!("ACCOUNT_LINK_URL");

        let discord_token = env!("DISCORD_TOKEN");
        let discord_client_id = env!("DISCORD_CLIENT_ID");
        let discord_client_secret = env!("DISCORD_CLIENT_SECRET");
        let discord_oauth_redirect = env!("DISCORD_OAUTH_REDIRECT");

        let discord_guild_id = env!("DISCORD_GUILD_ID")
            .parse::<u64>()
            .expect("DISCORD_GUILD_ID must be an integer");

        let discord_allowed_roles = env!("DISCORD_ALLOWED_ROLES")
            .split(" ")
            .map(|role| role.parse().expect("DISCORD ROLES must be an integers"))
            .collect::<Vec<_>>();

        let telegram_group_id = env!("TELEGRAM_GROUP_ID")
            .parse::<i64>()
            .expect("TELEGRAM_GROUP_ID must be an integer");

        tracing::debug!(
            port = %port,
            discord_guild_id = %discord_guild_id,
            telegram_group_id = %telegram_group_id,
            allowed_roles_count = discord_allowed_roles.len(),
            "Environment configuration loaded"
        );

        Self {
            port,
            database_url,
            account_link_url,
            discord_token,
            discord_client_id,
            discord_client_secret,
            discord_oauth_redirect,
            discord_guild_id,
            discord_allowed_roles,
            telegram_group_id,
        }
    }
}
