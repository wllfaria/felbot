use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;

#[derive(Debug, FromRow)]
pub struct UserLink {
    pub id: Uuid,
    pub discord_id: i64,
    pub telegram_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub added_to_group_at: Option<DateTime<Utc>>,
    pub last_subscription_check: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct UserLinkPayload {
    pub discord_id: i64,
    pub telegram_id: i64,
}

impl UserLinkPayload {
    pub fn new(discord_id: i64, telegram_id: i64) -> Self {
        Self {
            discord_id,
            telegram_id,
        }
    }
}

impl UserLink {
    pub async fn create_link(
        executor: &mut PgConnection,
        new_link: UserLinkPayload,
    ) -> sqlx::Result<UserLink> {
        let user_link = sqlx::query_as!(
            UserLink,
            r#"
            INSERT INTO user_links (discord_id, telegram_id)
            VALUES ($1, $2)
            RETURNING *
            "#,
            new_link.discord_id,
            new_link.telegram_id,
        )
        .fetch_one(executor)
        .await?;

        Ok(user_link)
    }

    pub async fn find_by_discord_id(
        executor: &mut PgConnection,
        discord_id: i64,
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
        telegram_id: i64,
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

    pub async fn delete_by_discord_id(
        executor: &mut PgConnection,
        discord_id: i64,
    ) -> sqlx::Result<()> {
        sqlx::query!("DELETE FROM user_links WHERE discord_id = $1", discord_id)
            .execute(executor)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_create_and_find_user_link(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let payload = UserLinkPayload::new(123456, 789012);
        let created = UserLink::create_link(&mut conn, payload).await.unwrap();

        assert_eq!(created.discord_id, 123456);
        assert_eq!(created.telegram_id, 789012);
        assert!(created.added_to_group_at.is_none());

        let found = UserLink::find_by_discord_id(&mut conn, 123456)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().telegram_id, 789012);
    }

    #[sqlx::test]
    async fn test_find_by_telegram_id(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let payload = UserLinkPayload::new(123456, 789012);
        UserLink::create_link(&mut conn, payload).await.unwrap();

        let found = UserLink::find_by_telegram_id(&mut conn, 789012)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().discord_id, 123456);
    }

    #[sqlx::test]
    async fn test_duplicate_discord_id(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let payload1 = UserLinkPayload::new(123456, 789012);
        UserLink::create_link(&mut conn, payload1).await.unwrap();

        let payload2 = UserLinkPayload::new(123456, 999999);
        let result = UserLink::create_link(&mut conn, payload2).await;

        assert!(result.is_err()); // Should fail due to unique constraint
    }

    #[sqlx::test]
    async fn test_duplicate_telegram_id(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let payload1 = UserLinkPayload::new(123456, 789012);
        UserLink::create_link(&mut conn, payload1).await.unwrap();

        let payload2 = UserLinkPayload::new(999999, 789012);
        let result = UserLink::create_link(&mut conn, payload2).await;

        assert!(result.is_err()); // Should fail due to unique constraint
    }

    #[sqlx::test]
    async fn test_mark_added_to_group(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let payload = UserLinkPayload::new(123456, 789012);
        let created = UserLink::create_link(&mut conn, payload).await.unwrap();

        assert!(created.added_to_group_at.is_none());

        UserLink::mark_added_to_group(&mut conn, &created.id)
            .await
            .unwrap();

        let updated = UserLink::find_by_discord_id(&mut conn, 123456)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.added_to_group_at.is_some());
    }

    #[sqlx::test]
    async fn test_delete_by_discord_id(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let payload = UserLinkPayload::new(123456, 789012);
        UserLink::create_link(&mut conn, payload).await.unwrap();

        let found_before = UserLink::find_by_discord_id(&mut conn, 123456)
            .await
            .unwrap();
        assert!(found_before.is_some());

        UserLink::delete_by_discord_id(&mut conn, 123456)
            .await
            .unwrap();

        let found_after = UserLink::find_by_discord_id(&mut conn, 123456)
            .await
            .unwrap();
        assert!(found_after.is_none());
    }
}
