use itertools::Itertools;

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
    pub cron_secret: String,

    pub discord_token: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub discord_oauth_redirect: String,
    pub discord_guild_ids: Vec<u64>,
    pub discord_allowed_roles: Vec<u64>,
    pub discord_channel_ids: Vec<u64>,

    pub telegram_group_id: i64,
}

impl Env {
    pub fn new() -> Self {
        let port = env!("PORT");
        let database_url = env!("DATABASE_URL");
        let account_link_url = env!("ACCOUNT_LINK_URL");
        let cron_secret = env!("CRON_SECRET");

        let discord_token = env!("DISCORD_TOKEN");
        let discord_client_id = env!("DISCORD_CLIENT_ID");
        let discord_client_secret = env!("DISCORD_CLIENT_SECRET");
        let discord_oauth_redirect = env!("DISCORD_OAUTH_REDIRECT");

        let discord_channel_ids = env!("DISCORD_CHANNEL_IDS")
            .split(" ")
            .map(|channel| channel.parse().expect("CHANNEL ID must be integers"))
            .collect();

        let discord_guild_ids = env!("DISCORD_GUILD_IDS")
            .split(" ")
            .map(|guild| guild.parse().expect("GUILD_ID must be an ingeter"))
            .collect_vec();

        let discord_allowed_roles = env!("DISCORD_ALLOWED_ROLES")
            .split(" ")
            .map(|role| role.parse().expect("DISCORD ROLES must be integers"))
            .collect();

        let telegram_group_id = env!("TELEGRAM_GROUP_ID")
            .parse::<i64>()
            .expect("TELEGRAM_GROUP_ID must be an integer");

        Self {
            port,
            database_url,
            account_link_url,
            cron_secret,
            discord_token,
            discord_client_id,
            discord_client_secret,
            discord_oauth_redirect,
            discord_guild_ids,
            discord_allowed_roles,
            discord_channel_ids,
            telegram_group_id,
        }
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            port: Default::default(),
            database_url: Default::default(),
            account_link_url: Default::default(),
            cron_secret: Default::default(),
            discord_token: Default::default(),
            discord_client_id: Default::default(),
            discord_client_secret: Default::default(),
            discord_oauth_redirect: Default::default(),
            discord_guild_ids: Default::default(),
            discord_allowed_roles: Default::default(),
            discord_channel_ids: Default::default(),
            telegram_group_id: Default::default(),
        }
    }
}
