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

#[cfg(test)]
#[derive(Debug)]
pub struct AllowedGuildPayload {
    pub name: String,
    pub guild_id: i64,
}

#[cfg(test)]
impl AllowedGuildPayload {
    pub fn new(guild_id: i64, name: String) -> Self {
        Self { name, guild_id }
    }
}

impl AllowedGuild {
    #[cfg(test)]
    pub async fn create(
        executor: &mut sqlx::PgConnection,
        payload: AllowedGuildPayload,
    ) -> Result<Self, sqlx::Error> {
        let guild = sqlx::query_as!(
            Self,
            "INSERT INTO allowed_guilds (guild_id, name)
            VALUES ($1, $2)
            RETURNING *",
            payload.guild_id,
            payload.name
        )
        .fetch_one(executor)
        .await?;

        Ok(guild)
    }

    pub async fn get_guilds(executor: &mut sqlx::PgConnection) -> Result<Vec<Self>, sqlx::Error> {
        let guilds = sqlx::query_as!(Self, "SELECT * FROM allowed_guilds")
            .fetch_all(executor)
            .await?;

        Ok(guilds)
    }

    pub async fn get_guild_id(
        executor: &mut sqlx::PgConnection,
        guild_id: i64,
    ) -> Result<Uuid, sqlx::Error> {
        let guild = sqlx::query_as!(
            Self,
            "SELECT * FROM allowed_guilds WHERE guild_id = $1",
            guild_id
        )
        .fetch_one(executor)
        .await?;

        Ok(guild.id)
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
