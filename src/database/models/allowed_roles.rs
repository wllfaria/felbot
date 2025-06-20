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
    pub guild_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AllowedRolePayload {
    pub role_id: i64,
    pub name: String,
    pub is_admin: bool,
    pub guild_id: Uuid,
}

impl AllowedRolePayload {
    pub fn new(role_id: i64, name: String, is_admin: bool, guild_id: Uuid) -> Self {
        Self {
            role_id,
            name,
            is_admin,
            guild_id,
        }
    }
}

impl AllowedRole {
    pub async fn exists(
        executor: &mut PgConnection,
        id: i64,
        guild_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let role = sqlx::query_as!(
            Self,
            "SELECT * FROM allowed_roles WHERE role_id = $1 AND guild_id = $2",
            id,
            guild_id
        )
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
            "INSERT INTO allowed_roles (role_id, name, is_admin, guild_id)
            VALUES ($1, $2, $3, $4)
            RETURNING *",
            payload.role_id,
            payload.name,
            payload.is_admin,
            payload.guild_id,
        )
        .fetch_one(executor)
        .await?;

        Ok(role)
    }

    pub async fn get_roles(
        executor: &mut sqlx::PgConnection,
        guild_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let roles = sqlx::query_as!(
            Self,
            "SELECT * FROM allowed_roles WHERE guild_id = $1",
            guild_id
        )
        .fetch_all(executor)
        .await?;

        Ok(roles)
    }

    pub async fn get_guild_admin_ids(
        executor: &mut PgConnection,
        guild_id: Uuid,
    ) -> Result<Vec<u64>, sqlx::Error> {
        let admin_roles = sqlx::query_as!(
            Self,
            "SELECT * FROM allowed_roles WHERE is_admin = TRUE AND guild_id = $1",
            guild_id
        )
        .fetch_all(executor)
        .await?
        .into_iter()
        .map(|role| role.role_id as u64)
        .collect_vec();

        Ok(admin_roles)
    }

    pub async fn get_guild_role_ids(
        executor: &mut sqlx::PgConnection,
        guild_id: Uuid,
    ) -> Result<Vec<u64>, sqlx::Error> {
        let role_ids = sqlx::query_as!(
            Self,
            "SELECT * FROM allowed_roles WHERE guild_id = $1",
            guild_id
        )
        .fetch_all(executor)
        .await?
        .into_iter()
        .map(|role| role.role_id as u64)
        .collect_vec();

        Ok(role_ids)
    }

    pub async fn delete(
        executor: &mut sqlx::PgConnection,
        id: i64,
        guild_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM allowed_roles WHERE role_id = $1 AND guild_id = $2",
            id,
            guild_id
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
