use itertools::Itertools;
use poise::serenity_prelude::RoleId;
use uuid::Uuid;

use super::validate_guild;
use crate::database::models::allowed_guilds::AllowedGuild;
use crate::database::models::allowed_roles::{AllowedRole, AllowedRolePayload};
use crate::discord::Context;
use crate::discord::commands::create_standard_reply;
use crate::discord::error::{Error, InvalidGuildError, InvalidRoleError, PermissionError, Result};
use crate::discord::permissions::is_admin;

#[allow(clippy::result_large_err)]
fn parse_role_id(id: &str) -> Result<i64> {
    id.parse::<i64>().map_err(|_| {
        let message = "ID do cargo inválido".to_string();
        Error::InvalidRole(InvalidRoleError::new(message))
    })
}

async fn validate_role(ctx: Context<'_>, role_id: RoleId) -> Result<(String, u64)> {
    let Some(guild) = ctx.guild() else {
        let message = "Esse comando só pode ser usado em servidores".to_string();
        return Err(Error::InvalidGuild(InvalidGuildError::new(message)));
    };

    let Some(role) = guild.roles.get(&role_id) else {
        let message = "Esse cargo não é um cargo do servidor".to_string();
        return Err(Error::InvalidRole(InvalidRoleError::new(message)));
    };

    let name = role.name.clone();
    Ok((name, guild.id.get()))
}

#[poise::command(
    slash_command,
    rename = "cargos",
    check = "is_admin",
    subcommands("list_roles", "add_role", "del_role"),
    description_localized("pt-BR", "Gerenciar cargos permitidos para comandos do bot")
)]
pub async fn roles(ctx: Context<'_>) -> Result<()> {
    let message =
        "Por favor, use um dos subcomandos: `/cargos listar`, `/cargos novo` ou `/cargos remover`"
            .into();
    let reply = create_standard_reply(message);

    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send list channels command response");
        e
    })?;

    Ok(())
}

#[poise::command(
    slash_command,
    rename = "listar",
    check = "is_admin",
    description_localized("pt-BR", "Lista todos os cargos permitidos para uso do bot")
)]
async fn list_roles(ctx: Context<'_>) -> Result<()> {
    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    // Safety: we check if we are on a guild in the is_admin check handler above
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let guild_id = AllowedGuild::get_guild_id(conn.as_mut(), guild_id).await?;

    let (admin_roles, non_admin_roles) = list_roles_inner(conn.as_mut(), guild_id).await?;
    let formatted_roles = format_roles(admin_roles, non_admin_roles);
    let reply = create_standard_reply(formatted_roles);

    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send list roles command response");
        e
    })?;

    Ok(())
}

async fn list_roles_inner(
    conn: &mut sqlx::PgConnection,
    guild_id: Uuid,
) -> Result<(Vec<AllowedRole>, Vec<AllowedRole>)> {
    let allowed_roles = AllowedRole::get_roles(conn.as_mut(), guild_id).await?;

    if allowed_roles.is_empty() {
        return Ok((vec![], vec![]));
    }

    let admin_roles = allowed_roles
        .iter()
        .filter(|role| role.is_admin)
        .cloned()
        .collect::<Vec<_>>();

    let non_admin_roles = allowed_roles
        .into_iter()
        .filter(|role| !role.is_admin)
        .collect::<Vec<_>>();

    Ok((admin_roles, non_admin_roles))
}

fn format_roles(admin_roles: Vec<AllowedRole>, non_admin_roles: Vec<AllowedRole>) -> String {
    let formatted_roles = format!(
        "Lista de cargos permitidos:\n\n[ADMINS]\n{}\n\n[SUBS]\n{}",
        admin_roles
            .iter()
            .map(|role| format!("{} - {}", role.role_id, role.name))
            .join("\n"),
        non_admin_roles
            .iter()
            .map(|role| format!("{} - {}", role.role_id, role.name))
            .join("\n")
    );

    formatted_roles
}

#[poise::command(
    slash_command,
    rename = "novo",
    check = "is_admin",
    description_localized("pt-BR", "Adiciona um novo cargo à lista de cargos permitidos")
)]
async fn add_role(
    ctx: Context<'_>,
    #[description = "ID do cargo para adicionar"] id: String,
    #[description = "É um cargo de administrador?"] admin: Option<bool>,
) -> Result<()> {
    let role_id = parse_role_id(&id)?;
    let role_name = get_role_name(ctx, role_id).await?;
    let is_admin = admin.unwrap_or_default();

    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    // Safety: we check if we are on a guild in the is_admin check handler above
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let guild_id = AllowedGuild::get_guild_id(conn.as_mut(), guild_id).await?;

    let new_role = add_role_inner(conn.as_mut(), role_id, role_name, is_admin, guild_id).await?;
    let description = format!(
        "Cargo adicionado com sucesso!\n\n**ID:** {}\n**Nome:** {}",
        new_role.role_id, new_role.name
    );
    let reply = create_standard_reply(description);
    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send add role command response");
        e
    })?;

    Ok(())
}

async fn add_role_inner(
    conn: &mut sqlx::PgConnection,
    role_id: i64,
    name: String,
    is_admin: bool,
    guild_id: Uuid,
) -> Result<AllowedRole> {
    let exists = AllowedRole::exists(conn, role_id, guild_id).await?;
    if exists {
        let message = "Cargo já existe na lista".to_string();
        return Err(Error::InvalidRole(InvalidRoleError::new(message)));
    }

    let payload = AllowedRolePayload::new(role_id, name, is_admin, guild_id);
    let new_role = AllowedRole::create(conn, payload).await?;
    Ok(new_role)
}

#[poise::command(
    slash_command,
    rename = "remover",
    check = "is_admin",
    description_localized("pt-BR", "Remove um cargo da lista de cargos permitidos")
)]
pub async fn del_role(
    ctx: Context<'_>,
    #[description = "ID do cargo para remover"] id: String,
) -> Result<()> {
    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    // Safety: we check if we are on a guild in the is_admin check handler above
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let guild_id = AllowedGuild::get_guild_id(conn.as_mut(), guild_id).await?;

    let role_id = del_role_inner(conn.as_mut(), id, guild_id).await?;
    let role_name = get_role_name(ctx, role_id).await?;
    let description = format!("Cargo removido com sucesso!\n\nID: {role_id}\nNome: {role_name}");
    let reply = create_standard_reply(description);
    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send add role command response");
        e
    })?;

    Ok(())
}

async fn del_role_inner(conn: &mut sqlx::PgConnection, id: String, guild_id: Uuid) -> Result<i64> {
    let role_id = parse_role_id(&id)?;

    let exists = AllowedRole::exists(conn, role_id, guild_id).await?;
    if !exists {
        let message = "Cargo não encontrado na lista".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    }

    AllowedRole::delete(conn, role_id, guild_id).await?;
    Ok(role_id)
}

async fn get_role_name(ctx: Context<'_>, role_id: i64) -> Result<String> {
    let (role_name, guild_id) = validate_role(ctx, RoleId::new(role_id as u64)).await?;
    validate_guild(&ctx.data().pool, guild_id).await?;
    Ok(role_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::allowed_guilds::{AllowedGuild, AllowedGuildPayload};

    #[sqlx::test]
    async fn test_list_roles_only_for_guild(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let mut guilds = vec![];
        for i in 0..2 {
            let guild = AllowedGuild::create(
                conn.as_mut(),
                AllowedGuildPayload::new(i, format!("Test Guild {i}")),
            )
            .await
            .unwrap();

            for j in 1..10 {
                let role_payload = AllowedRolePayload::new(
                    i + j,
                    format!("Test role {j} - guild {i}"),
                    false,
                    guild.id,
                );
                AllowedRole::create(conn.as_mut(), role_payload)
                    .await
                    .unwrap();
            }

            guilds.push(guild);
        }

        let roles = list_roles_inner(conn.as_mut(), guilds[0].id).await.unwrap();
        assert!(roles.0.is_empty());
        assert!(roles.1.len() == 9);
    }

    #[sqlx::test]
    fn test_list_roles_empty_guild(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(1, "Test Guild".to_string()),
        )
        .await
        .unwrap();

        for j in 1..10 {
            AllowedRole::create(
                conn.as_mut(),
                AllowedRolePayload::new(j, format!("Test role {j}"), false, guild.id),
            )
            .await
            .unwrap();
        }

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(2, "Test Guild 2".to_string()),
        )
        .await
        .unwrap();

        let mut conn = pool.acquire().await.unwrap();
        let roles = list_roles_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(roles.0.is_empty());
        assert!(roles.1.is_empty());
    }

    #[sqlx::test]
    fn test_add_role(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        // unrelated guild to check if we are not adding the role to the wrong guild
        let unrelated = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(1, "Test Guild".to_string()),
        )
        .await
        .unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(2, "Test Guild 2".to_string()),
        )
        .await
        .unwrap();

        let channel = add_role_inner(conn.as_mut(), 1000, "Teste".to_string(), false, guild.id)
            .await
            .unwrap();

        assert_eq!(channel.role_id, 1000);
        assert_eq!(channel.name, "Teste");
        assert_eq!(channel.guild_id, guild.id);

        let roles = list_roles_inner(conn.as_mut(), guild.id).await.unwrap();
        assert_eq!(roles.1.len(), 1);
        assert_eq!(roles.1[0].role_id, 1000);
        assert!(roles.0.is_empty());

        let unrelated_channels = list_roles_inner(conn.as_mut(), unrelated.id).await.unwrap();
        assert!(unrelated_channels.0.is_empty());
        assert!(unrelated_channels.1.is_empty());
    }

    #[sqlx::test]
    async fn test_delete_role_not_found_on_guild(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(2, "Test Guild 2".to_string()),
        )
        .await
        .unwrap();

        // create a role that should not be deleted, as it does not exist on the guild but is on
        // the same guild
        AllowedRole::create(
            conn.as_mut(),
            AllowedRolePayload::new(1000, "Test Channel".to_string(), false, guild.id),
        )
        .await
        .unwrap();

        let result = del_role_inner(conn.as_mut(), "9999".to_string(), guild.id).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cargo não encontrado na lista"
        );

        let roles = list_roles_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(roles.1.len() == 1);
        assert!(roles.0.is_empty());
    }

    #[sqlx::test]
    async fn test_delete_role_found_but_not_on_guild(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let unrelated = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(1, "Test Guild 2".to_string()),
        )
        .await
        .unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(2, "Test Guild 2".to_string()),
        )
        .await
        .unwrap();

        // create a role that exists but not on the guild we are trying to delete, so it should
        // error
        AllowedRole::create(
            conn.as_mut(),
            AllowedRolePayload::new(1000, "Test Channel".to_string(), false, unrelated.id),
        )
        .await
        .unwrap();

        let result = del_role_inner(conn.as_mut(), "1000".to_string(), guild.id).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cargo não encontrado na lista"
        );

        let roles = list_roles_inner(conn.as_mut(), unrelated.id).await.unwrap();
        assert!(roles.1.len() == 1);
        assert!(roles.0.is_empty());

        let roles = list_roles_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(roles.1.is_empty());
        assert!(roles.0.is_empty());
    }

    #[sqlx::test]
    async fn test_delete_role(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(2, "Test Guild 2".to_string()),
        )
        .await
        .unwrap();

        AllowedRole::create(
            conn.as_mut(),
            AllowedRolePayload::new(1000, "Test Channel".to_string(), false, guild.id),
        )
        .await
        .unwrap();

        let roles = list_roles_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(!roles.1.is_empty());
        assert!(roles.0.is_empty());

        let result = del_role_inner(conn.as_mut(), "1000".to_string(), guild.id).await;
        assert!(result.is_ok());

        let roles = list_roles_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(roles.0.is_empty());
        assert!(roles.1.is_empty());
    }

    #[sqlx::test]
    async fn test_add_duplicate_role(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(1, "Test Guild".to_string()),
        )
        .await
        .unwrap();

        AllowedRole::create(
            conn.as_mut(),
            AllowedRolePayload::new(1000, "Test Channel".to_string(), false, guild.id),
        )
        .await
        .unwrap();

        let result = add_role_inner(
            conn.as_mut(),
            1000,
            "Test Channel".to_string(),
            false,
            guild.id,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Cargo já existe na lista");
    }

    #[test]
    fn test_parse_role_id_valid() {
        let result = parse_role_id("12345");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12345);
    }

    #[test]
    fn test_parse_role_id_invalid() {
        let result = parse_role_id("not-a-number");
        assert!(result.is_err());
    }
}
