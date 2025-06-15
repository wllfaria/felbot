use itertools::Itertools;
use sqlx::PgConnection;
use sqlx::types::Uuid;
use sqlx::types::chrono::{DateTime, Utc};

#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AllowedRole {
    pub id: Uuid,
    pub role_id: i64,
    pub name: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AllowedRole {
    pub async fn get_admin_ids(executor: &mut PgConnection) -> Result<Vec<u64>, sqlx::Error> {
        let admin_roles =
            sqlx::query_as!(Self, "SELECT * FROM allowed_roles WHERE is_admin = TRUE")
                .fetch_all(executor)
                .await?
                .into_iter()
                .map(|role| role.role_id as u64)
                .collect_vec();

        Ok(admin_roles)
    }

    pub async fn get_role_ids(executor: &mut sqlx::PgConnection) -> Result<Vec<u64>, sqlx::Error> {
        let role_ids = sqlx::query_as!(Self, "SELECT * FROM allowed_roles")
            .fetch_all(executor)
            .await?
            .into_iter()
            .map(|role| role.role_id as u64)
            .collect_vec();

        Ok(role_ids)
    }
}
