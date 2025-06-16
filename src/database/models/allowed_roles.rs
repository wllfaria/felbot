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

pub struct AllowedRolePayload {
    pub role_id: i64,
    pub name: String,
    pub is_admin: bool,
}

impl AllowedRolePayload {
    pub fn new(role_id: i64, name: String, is_admin: bool) -> Self {
        Self {
            role_id,
            name,
            is_admin,
        }
    }
}

impl AllowedRole {
    pub async fn exists(executor: &mut PgConnection, id: i64) -> Result<bool, sqlx::Error> {
        let role = sqlx::query_as!(Self, "SELECT * FROM allowed_roles WHERE role_id = $1", id)
            .fetch_optional(executor)
            .await?;

        Ok(role.is_some())
    }

    pub async fn create(
        executor: &mut PgConnection,
        payload: AllowedRolePayload,
    ) -> Result<Self, sqlx::Error> {
        let role = sqlx::query_as!(
            Self,
            "INSERT INTO allowed_roles (role_id, name, is_admin)
            VALUES ($1, $2, $3)
            RETURNING *",
            payload.role_id,
            payload.name,
            payload.is_admin,
        )
        .fetch_one(executor)
        .await?;

        Ok(role)
    }

    pub async fn get_roles(executor: &mut sqlx::PgConnection) -> Result<Vec<Self>, sqlx::Error> {
        let roles = sqlx::query_as!(Self, "SELECT * FROM allowed_roles")
            .fetch_all(executor)
            .await?;

        Ok(roles)
    }

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

    pub async fn delete(executor: &mut sqlx::PgConnection, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM allowed_roles WHERE role_id = $1", id)
            .execute(executor)
            .await?;

        Ok(())
    }
}
