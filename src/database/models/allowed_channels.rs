use itertools::Itertools;
use sqlx::types::Uuid;
use sqlx::types::chrono::{DateTime, Utc};

#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AllowedChannel {
    pub id: Uuid,
    pub channel_id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct AllowedChannelPayload {
    pub channel_id: i64,
    pub name: String,
}

impl AllowedChannelPayload {
    pub fn new(channel_id: i64, name: String) -> Self {
        Self { channel_id, name }
    }
}

impl AllowedChannel {
    pub async fn get_channels(executor: &mut sqlx::PgConnection) -> Result<Vec<Self>, sqlx::Error> {
        let channels = sqlx::query_as!(Self, "SELECT * FROM allowed_channels")
            .fetch_all(executor)
            .await?;

        Ok(channels)
    }

    pub async fn get_channel_ids(
        executor: &mut sqlx::PgConnection,
    ) -> Result<Vec<u64>, sqlx::Error> {
        let channel_ids = Self::get_channels(executor)
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
            "INSERT INTO allowed_channels (channel_id, name)
            VALUES ($1, $2)
            ON CONFLICT (channel_id) DO UPDATE SET name = $2
            RETURNING *",
            payload.channel_id,
            payload.name,
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
