use itertools::Itertools;
use sqlx::types::Uuid;
use sqlx::types::chrono::{DateTime, Utc};

#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AllowedChannel {
    pub id: Uuid,
    pub channel_id: i64,
    pub name: String,
    pub guild_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct AllowedChannelPayload {
    pub channel_id: i64,
    pub name: String,
    pub guild_id: Uuid,
}

impl AllowedChannelPayload {
    pub fn new(channel_id: i64, name: String, guild_id: Uuid) -> Self {
        Self {
            channel_id,
            name,
            guild_id,
        }
    }
}

impl AllowedChannel {
    pub async fn exists(
        executor: &mut sqlx::PgConnection,
        channel_id: i64,
        guild_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM allowed_channels WHERE channel_id = $1 AND guild_id = $2)",
            channel_id,
            guild_id
        )
        .fetch_one(executor)
        .await?;

        Ok(exists.unwrap_or_default())
    }

    pub async fn get_guild_channels(
        executor: &mut sqlx::PgConnection,
        guild_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let channels = sqlx::query_as!(
            Self,
            "SELECT * FROM allowed_channels WHERE guild_id = $1",
            guild_id
        )
        .fetch_all(executor)
        .await?;

        Ok(channels)
    }

    pub async fn get_guild_channel_ids(
        executor: &mut sqlx::PgConnection,
        guild_id: Uuid,
    ) -> Result<Vec<u64>, sqlx::Error> {
        let channel_ids = Self::get_guild_channels(executor, guild_id)
            .await?
            .into_iter()
            .map(|channel| channel.channel_id as u64)
            .collect_vec();

        Ok(channel_ids)
    }

    pub async fn create(
        executor: &mut sqlx::PgConnection,
        payload: AllowedChannelPayload,
    ) -> Result<Self, sqlx::Error> {
        let channel = sqlx::query_as!(
            Self,
            "INSERT INTO allowed_channels (channel_id, name, guild_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (channel_id) DO UPDATE SET name = $2
            RETURNING *",
            payload.channel_id,
            payload.name,
            payload.guild_id
        )
        .fetch_one(executor)
        .await?;

        Ok(channel)
    }

    pub async fn delete(
        executor: &mut sqlx::PgConnection,
        channel_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM allowed_channels WHERE channel_id = $1",
            channel_id
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
