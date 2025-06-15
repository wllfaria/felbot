use itertools::Itertools;
use poise::serenity_prelude::{self as serenity};

use crate::database::models::allowed_channels::{AllowedChannel, AllowedChannelPayload};
use crate::database::models::allowed_guilds::AllowedGuild;
use crate::discord::commands::create_standard_reply;
use crate::discord::error::{InvalidChannelError, InvalidGuildError, PermissionError, Result};
use crate::discord::permissions::is_admin;
use crate::discord::{Context, Error};

#[allow(clippy::result_large_err)]
fn parse_channel_id(id: &str) -> Result<i64> {
    id.parse::<i64>().map_err(|_| {
        let message = "ID do canal inválido".to_string();
        Error::InvalidChannel(InvalidChannelError::new(message))
    })
}

async fn validate_guild(pool: &sqlx::PgPool, guild_id: u64) -> Result<()> {
    let mut conn = pool.acquire().await?;
    let allowed_guild_ids = AllowedGuild::get_guild_ids(conn.as_mut()).await?;

    if !allowed_guild_ids.contains(&guild_id) {
        let message = "Esse canal não é um canal de um servidor permitido".to_string();
        return Err(Error::InvalidGuild(InvalidGuildError::new(message)));
    }

    Ok(())
}

async fn validate_channel(ctx: Context<'_>, channel_id: i64) -> Result<(String, u64)> {
    let channel = serenity::ChannelId::new(channel_id as u64)
        .to_channel(ctx)
        .await?;

    match channel {
        serenity::Channel::Guild(guild_channel) => {
            let name = guild_channel.name.clone();
            let guild_id = guild_channel.guild_id.get();
            Ok((name, guild_id))
        }
        _ => {
            let message = "Esse canal não é um canal do servidor".to_string();
            Err(Error::InvalidChannel(InvalidChannelError::new(message)))
        }
    }
}

#[poise::command(
    slash_command,
    rename = "canais",
    subcommands("list_channels", "add_channel", "del_channel"),
    check = "is_admin",
    description_localized("pt-BR", "Gerenciar canais permitidos para comandos do bot")
)]
pub async fn channels(ctx: Context<'_>) -> Result<()> {
    ctx.say("Please use one of the subcommands: `/canais listar` or `/canais novo`")
        .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    rename = "listar",
    check = "is_admin",
    description_localized("pt-BR", "Lista todos os canais permitidos para uso do bot")
)]
async fn list_channels(ctx: Context<'_>) -> Result<()> {
    let formatted_channels = list_channels_inner(&ctx.data().pool).await?;
    let reply = create_standard_reply(formatted_channels);

    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send list channels command response");
        e
    })?;

    Ok(())
}

async fn list_channels_inner(pool: &sqlx::PgPool) -> Result<String> {
    let mut conn = pool.acquire().await?;
    let allowed_channels = AllowedChannel::get_channels(conn.as_mut()).await?;

    if allowed_channels.is_empty() {
        return Ok("Nenhum canal na lista de canais permitidos".to_string());
    }

    let formatted_channels = allowed_channels
        .into_iter()
        .map(|channel| format!("{} - {}", channel.channel_id, channel.name))
        .join("\n");

    let formatted_channels = format!("Lista de canais permitidos:\n\n{}", formatted_channels);
    Ok(formatted_channels)
}

#[poise::command(
    slash_command,
    rename = "novo",
    check = "is_admin",
    description_localized("pt-BR", "Adiciona um novo canal à lista de canais permitidos")
)]
async fn add_channel(
    ctx: Context<'_>,
    #[description = "ID do canal para adicionar"] id: String,
) -> Result<()> {
    let channel_id = parse_channel_id(&id)?;
    let channel_name = get_channel_name(ctx, channel_id).await?;
    let new_channel = add_channel_inner(&ctx.data().pool, channel_id, channel_name).await?;
    let description = format!(
        "Canal adicionado com sucesso!\n\n**ID:** {}\n**Nome:** {}",
        new_channel.channel_id, new_channel.name
    );
    let reply = create_standard_reply(description);
    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send add channel command response");
        e
    })?;

    Ok(())
}

async fn add_channel_inner(pool: &sqlx::PgPool, id: i64, name: String) -> Result<AllowedChannel> {
    let mut conn = pool.acquire().await?;
    let exists = AllowedChannel::exists(conn.as_mut(), id).await?;
    if exists {
        let message = "Canal já existe na lista".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    }

    let payload = AllowedChannelPayload::new(id, name);
    let new_channel = AllowedChannel::create(conn.as_mut(), payload).await?;
    Ok(new_channel)
}

#[poise::command(
    slash_command,
    rename = "remover",
    check = "is_admin",
    description_localized("pt-BR", "Remove um canal da lista de canais permitidos")
)]
pub async fn del_channel(
    ctx: Context<'_>,
    #[description = "ID do canal para remover"] id: String,
) -> Result<()> {
    let channel_id = del_channel_inner(&ctx.data().pool, id).await?;
    let channel_name = get_channel_name(ctx, channel_id).await?;
    let description =
        format!("Canal removido com sucesso!\n\nID: {channel_id}\nNome: {channel_name}");
    let reply = create_standard_reply(description);
    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send add channel command response");
        e
    })?;

    Ok(())
}

async fn del_channel_inner(pool: &sqlx::PgPool, id: String) -> Result<i64> {
    let channel_id = parse_channel_id(&id)?;
    let mut conn = pool.acquire().await?;

    let exists = AllowedChannel::exists(conn.as_mut(), channel_id).await?;
    if !exists {
        let message = "Canal não encontrado na lista".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    }

    AllowedChannel::delete(conn.as_mut(), channel_id).await?;
    Ok(channel_id)
}

async fn get_channel_name(ctx: Context<'_>, channel_id: i64) -> Result<String> {
    let (channel_name, guild_id) = validate_channel(ctx, channel_id).await?;
    validate_guild(&ctx.data().pool, guild_id).await?;
    Ok(channel_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_data(pool: &sqlx::PgPool) -> Result<i64> {
        // Create test channel
        let mut conn = pool.acquire().await?;
        let test_id = 999999;
        let payload = AllowedChannelPayload::new(test_id, "Test Channel".to_string());
        let _ = AllowedChannel::create(conn.as_mut(), payload).await?;
        Ok(test_id)
    }

    async fn cleanup_test_data(pool: &sqlx::PgPool, id: i64) -> Result<()> {
        let mut conn = pool.acquire().await?;
        AllowedChannel::delete(conn.as_mut(), id).await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_channels_with_data(pool: sqlx::PgPool) {
        let test_id = setup_test_data(&pool).await.unwrap();
        let channels = list_channels_inner(&pool).await.unwrap();
        assert!(channels.contains(&format!("{} - Test Channel", test_id)));
        cleanup_test_data(&pool, test_id).await.unwrap();
    }

    #[sqlx::test]
    fn test_list_channels(pool: sqlx::PgPool) {
        let channels = list_channels_inner(&pool).await.unwrap();
        assert!(!channels.is_empty());
    }

    #[sqlx::test]
    fn test_add_channel(pool: sqlx::PgPool) {
        let channel_id = add_channel_inner(&pool, 1000, "Teste".to_string())
            .await
            .unwrap();
        assert_eq!(channel_id.channel_id, 1000);
        assert_eq!(channel_id.name, "Teste");
    }

    #[test]
    fn test_parse_channel_id_valid() {
        let result = parse_channel_id("12345");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12345);
    }

    #[test]
    fn test_parse_channel_id_invalid() {
        let result = parse_channel_id("not-a-number");
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn test_channel_not_found(pool: sqlx::PgPool) {
        let non_existent_id = 9999999;
        let result = del_channel_inner(&pool, non_existent_id.to_string()).await;
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn test_add_duplicate_channel(pool: sqlx::PgPool) {
        let test_id = 12345;
        let test_name = "Duplicate Test".to_string();

        let _ = add_channel_inner(&pool, test_id, test_name.clone())
            .await
            .unwrap();

        let result = add_channel_inner(&pool, test_id, test_name.clone()).await;
        assert!(result.is_err());

        let _ = del_channel_inner(&pool, test_id.to_string()).await.unwrap();
    }
}
