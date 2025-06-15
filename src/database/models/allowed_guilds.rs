use itertools::Itertools;
use sqlx::types::Uuid;
use sqlx::types::chrono::{DateTime, Utc};

#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AllowedGuild {
    pub id: Uuid,
    pub guild_id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AllowedGuild {
    pub async fn get_guilds(executor: &mut sqlx::PgConnection) -> Result<Vec<Self>, sqlx::Error> {
        let guilds = sqlx::query_as!(Self, "SELECT * FROM allowed_guilds")
            .fetch_all(executor)
            .await?;

        Ok(guilds)
    }

    pub async fn get_guild_ids(executor: &mut sqlx::PgConnection) -> Result<Vec<u64>, sqlx::Error> {
        let guild_ids = Self::get_guilds(executor)
            .await?
            .into_iter()
            .map(|guild| guild.guild_id as u64)
            .collect_vec();

        Ok(guild_ids)
    }
}
