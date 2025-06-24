use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::telegram::Groups;

pub struct TelegramGroup {
    pub id: Uuid,
    pub owner: String,
    pub guild_id: Uuid,
    pub telegram_group_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TelegramGroup {
    pub async fn get(executor: &mut sqlx::PgConnection) -> sqlx::Result<Vec<Self>> {
        let groups = sqlx::query_as!(Self, "SELECT * FROM telegram_groups")
            .fetch_all(executor)
            .await?;

        Ok(groups)
    }

    pub async fn find_by_name(
        executor: &mut sqlx::PgConnection,
        group: Groups,
    ) -> sqlx::Result<Self> {
        let group = sqlx::query_as!(
            Self,
            "SELECT * FROM telegram_groups WHERE owner = $1",
            group.to_string()
        )
        .fetch_one(executor)
        .await?;

        Ok(group)
    }
}
