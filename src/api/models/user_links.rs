use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;

#[derive(Debug, FromRow)]
pub struct UserLink {
    pub id: Uuid,
    pub discord_id: String,
    pub discord_username: String,
    pub telegram_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub added_to_group_at: Option<DateTime<Utc>>,
    pub last_subscription_check: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct UserLinkPayload {
    pub discord_id: String,
    pub discord_username: String,
    pub telegram_id: String,
}

impl UserLink {
    pub async fn create_link(
        executor: &mut PgConnection,
        new_link: UserLinkPayload,
    ) -> sqlx::Result<UserLink> {
        let user_link = sqlx::query_as!(
            UserLink,
            r#"
            INSERT INTO user_links (discord_id, discord_username, telegram_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
            new_link.discord_id,
            new_link.discord_username,
            new_link.telegram_id,
        )
        .fetch_one(executor)
        .await?;

        Ok(user_link)
    }

    pub async fn find_by_discord_id(
        executor: &mut PgConnection,
        discord_id: &str,
    ) -> sqlx::Result<Option<UserLink>> {
        let user_link = sqlx::query_as!(
            UserLink,
            "SELECT * FROM user_links WHERE discord_id = $1",
            discord_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(user_link)
    }

    pub async fn find_by_telegram_id(
        executor: &mut PgConnection,
        telegram_id: &str,
    ) -> sqlx::Result<Option<UserLink>> {
        let user_link = sqlx::query_as!(
            UserLink,
            "SELECT * FROM user_links WHERE telegram_id = $1",
            telegram_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(user_link)
    }

    pub async fn mark_added_to_group(executor: &mut PgConnection, id: &Uuid) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE user_links SET added_to_group_at = NOW() WHERE id = $1",
            id
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn get_all_users(executor: &mut PgConnection) -> sqlx::Result<Vec<UserLink>> {
        let users = sqlx::query_as!(UserLink, "SELECT * FROM user_links")
            .fetch_all(executor)
            .await?;

        Ok(users)
    }
}
