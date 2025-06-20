use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::prelude::FromRow;
use sqlx::types::Uuid;

#[allow(dead_code)]
#[derive(Debug, FromRow)]
pub struct OAuthState {
    pub id: Uuid,
    pub state_token: String,
    pub telegram_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl OAuthState {
    pub async fn create(
        executor: &mut PgConnection,
        telegram_id: i64,
        token: &str,
    ) -> sqlx::Result<OAuthState> {
        let state = sqlx::query_as!(
            OAuthState,
            "INSERT INTO oauth_states (state_token, telegram_id) VALUES ($1, $2) RETURNING *",
            token,
            telegram_id
        )
        .fetch_one(executor)
        .await?;

        Ok(state)
    }

    pub async fn get_and_delete(
        executor: &mut PgConnection,
        token: &str,
    ) -> sqlx::Result<Option<OAuthState>> {
        let result = sqlx::query_as!(
            OAuthState,
            "DELETE FROM oauth_states WHERE state_token = $1 AND expires_at > NOW() RETURNING *",
            token
        )
        .fetch_optional(executor)
        .await?;

        Ok(result)
    }
}
