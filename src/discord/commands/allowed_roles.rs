use itertools::Itertools;
use poise::serenity_prelude::RoleId;

use super::validate_guild;
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
    let formatted_roles = list_roles_inner(&ctx.data().pool).await?;
    let reply = create_standard_reply(formatted_roles);

    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send list roles command response");
        e
    })?;

    Ok(())
}

async fn list_roles_inner(pool: &sqlx::PgPool) -> Result<String> {
    let mut conn = pool.acquire().await?;
    let allowed_roles = AllowedRole::get_roles(conn.as_mut()).await?;

    if allowed_roles.is_empty() {
        return Ok("Nenhum cargo na lista de cargos permitidos".to_string());
    }

    let admin_roles = allowed_roles
        .iter()
        .filter(|role| role.is_admin)
        .collect::<Vec<_>>();

    let non_admin_roles = allowed_roles
        .iter()
        .filter(|role| !role.is_admin)
        .collect::<Vec<_>>();

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

    Ok(formatted_roles)
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

    let new_role = add_role_inner(&ctx.data().pool, role_id, role_name, is_admin).await?;
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
    pool: &sqlx::PgPool,
    role_id: i64,
    name: String,
    is_admin: bool,
) -> Result<AllowedRole> {
    let mut conn = pool.acquire().await?;
    let exists = AllowedRole::exists(conn.as_mut(), role_id).await?;
    if exists {
        let message = "Cargo já existe na lista".to_string();
        return Err(Error::InvalidRole(InvalidRoleError::new(message)));
    }

    let payload = AllowedRolePayload::new(role_id, name, is_admin);
    let new_role = AllowedRole::create(conn.as_mut(), payload).await?;
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
    let role_id = del_role_inner(&ctx.data().pool, id).await?;
    let role_name = get_role_name(ctx, role_id).await?;
    let description = format!("Cargo removido com sucesso!\n\nID: {role_id}\nNome: {role_name}");
    let reply = create_standard_reply(description);
    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send add role command response");
        e
    })?;

    Ok(())
}

async fn del_role_inner(pool: &sqlx::PgPool, id: String) -> Result<i64> {
    let role_id = parse_role_id(&id)?;
    let mut conn = pool.acquire().await?;

    let exists = AllowedRole::exists(conn.as_mut(), role_id).await?;
    if !exists {
        let message = "Cargo não encontrado na lista".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    }

    AllowedRole::delete(conn.as_mut(), role_id).await?;
    Ok(role_id)
}

async fn get_role_name(ctx: Context<'_>, role_id: i64) -> Result<String> {
    let (role_name, guild_id) = validate_role(ctx, RoleId::new(role_id as u64)).await?;
    validate_guild(&ctx.data().pool, guild_id).await?;
    Ok(role_name)
}
