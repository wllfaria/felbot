use itertools::Itertools;
use poise::serenity_prelude::{self as serenity};
use uuid::Uuid;

use super::validate_guild;
use crate::database::models::allowed_channels::{AllowedChannel, AllowedChannelPayload};
use crate::database::models::allowed_guilds::AllowedGuild;
use crate::discord::commands::create_standard_reply;
use crate::discord::error::{InvalidChannelError, PermissionError, Result};
use crate::discord::permissions::is_admin;
use crate::discord::{Context, Error};

#[allow(clippy::result_large_err)]
fn parse_channel_id(id: &str) -> Result<i64> {
    id.parse::<i64>().map_err(|_| {
        let message = "ID do canal inválido".to_string();
        Error::InvalidChannel(InvalidChannelError::new(message))
    })
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
    let message = "Por favor, use um dos subcomandos: `/canais listar` ou `/canais novo`".into();
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
    description_localized("pt-BR", "Lista todos os canais permitidos para uso do bot")
)]
async fn list_channels(ctx: Context<'_>) -> Result<()> {
    // Safety: we check if we are on a guild in the is_admin check handler above
    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    let guid_id = ctx.guild_id().unwrap().get() as i64;
    let guild_id = AllowedGuild::get_guild_id(conn.as_mut(), guid_id).await?;

    let allowed_channels = list_channels_inner(conn.as_mut(), guild_id).await?;
    let formatted_channels = format_allowed_channels(allowed_channels)?;
    let reply = create_standard_reply(formatted_channels);

    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send list channels command response");
        e
    })?;

    Ok(())
}

async fn list_channels_inner(
    conn: &mut sqlx::PgConnection,
    guild_id: Uuid,
) -> Result<Vec<AllowedChannel>> {
    let allowed_channels = AllowedChannel::get_guild_channels(conn, guild_id).await?;
    Ok(allowed_channels)
}

#[allow(clippy::result_large_err)]
fn format_allowed_channels(allowed_channels: Vec<AllowedChannel>) -> Result<String> {
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
    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    // Safety: we check if we are on a guild in the is_admin check handler above
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let guild_id = AllowedGuild::get_guild_id(conn.as_mut(), guild_id).await?;

    let channel_id = parse_channel_id(&id)?;
    let channel_name = get_channel_name(ctx, channel_id).await?;

    let new_channel = add_channel_inner(conn.as_mut(), channel_id, channel_name, guild_id).await?;
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

async fn add_channel_inner(
    conn: &mut sqlx::PgConnection,
    id: i64,
    name: String,
    guild_id: Uuid,
) -> Result<AllowedChannel> {
    let exists = AllowedChannel::exists(conn, id, guild_id).await?;
    if exists {
        let message = "Canal já existe na lista".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    }

    let payload = AllowedChannelPayload::new(id, name, guild_id);
    let new_channel = AllowedChannel::create(conn, payload).await?;
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
    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    // Safety: we check if we are on a guild in the is_admin check handler above
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let guild_id = AllowedGuild::get_guild_id(conn.as_mut(), guild_id).await?;

    let channel_id = del_channel_inner(conn.as_mut(), id, guild_id).await?;
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

async fn del_channel_inner(
    conn: &mut sqlx::PgConnection,
    id: String,
    guild_id: Uuid,
) -> Result<i64> {
    let channel_id = parse_channel_id(&id)?;

    let exists = AllowedChannel::exists(conn, channel_id, guild_id).await?;
    if !exists {
        let message = "Canal não encontrado na lista".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    }

    AllowedChannel::delete(conn, channel_id).await?;
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
    use crate::database::models::allowed_guilds::AllowedGuildPayload;

    #[sqlx::test]
    async fn test_list_channels_only_for_guild(pool: sqlx::PgPool) {
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
                let channel_payload = AllowedChannelPayload::new(
                    i + j,
                    format!("Test Channel {j} - guild {i}"),
                    guild.id,
                );
                AllowedChannel::create(conn.as_mut(), channel_payload)
                    .await
                    .unwrap();
            }

            guilds.push(guild);
        }

        let channels = list_channels_inner(conn.as_mut(), guilds[0].id)
            .await
            .unwrap();

        assert!(channels.len() == 9);
    }

    #[sqlx::test]
    fn test_list_channels_empty_guild(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(1, "Test Guild".to_string()),
        )
        .await
        .unwrap();

        for j in 1..10 {
            AllowedChannel::create(
                conn.as_mut(),
                AllowedChannelPayload::new(j, format!("Test Channel {j}"), guild.id),
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
        let channels = list_channels_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(channels.is_empty());
    }

    #[sqlx::test]
    fn test_add_channel(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        // unrelated guild to check if we are not adding the channel to the wrong guild
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

        let channel = add_channel_inner(conn.as_mut(), 1000, "Teste".to_string(), guild.id)
            .await
            .unwrap();

        assert_eq!(channel.channel_id, 1000);
        assert_eq!(channel.name, "Teste");
        assert_eq!(channel.guild_id, guild.id);

        let channels = list_channels_inner(conn.as_mut(), guild.id).await.unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].channel_id, 1000);

        let unrelated_channels = list_channels_inner(conn.as_mut(), unrelated.id)
            .await
            .unwrap();
        assert!(unrelated_channels.is_empty());
    }

    #[sqlx::test]
    async fn test_delete_channel_not_found_on_guild(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(2, "Test Guild 2".to_string()),
        )
        .await
        .unwrap();

        // create a channel that should not be deleted, as it does not exist on the guild but is on
        // the same guild
        AllowedChannel::create(
            conn.as_mut(),
            AllowedChannelPayload::new(1000, "Test Channel".to_string(), guild.id),
        )
        .await
        .unwrap();

        let result = del_channel_inner(conn.as_mut(), "9999".to_string(), guild.id).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Canal não encontrado na lista"
        );

        let channels = list_channels_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(channels.len() == 1);
    }

    #[sqlx::test]
    async fn test_delete_channel_found_but_not_on_guild(pool: sqlx::PgPool) {
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

        // create a channel that exists but not on the guild we are trying to delete, so it should
        // error
        AllowedChannel::create(
            conn.as_mut(),
            AllowedChannelPayload::new(1000, "Test Channel".to_string(), unrelated.id),
        )
        .await
        .unwrap();

        let result = del_channel_inner(conn.as_mut(), "1000".to_string(), guild.id).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Canal não encontrado na lista"
        );

        let channels = list_channels_inner(conn.as_mut(), unrelated.id)
            .await
            .unwrap();
        assert!(channels.len() == 1);

        let channels = list_channels_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(channels.is_empty());
    }

    #[sqlx::test]
    async fn test_delete_channel(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(2, "Test Guild 2".to_string()),
        )
        .await
        .unwrap();

        AllowedChannel::create(
            conn.as_mut(),
            AllowedChannelPayload::new(1000, "Test Channel".to_string(), guild.id),
        )
        .await
        .unwrap();

        let channels = list_channels_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(!channels.is_empty());

        let result = del_channel_inner(conn.as_mut(), "1000".to_string(), guild.id).await;
        assert!(result.is_ok());

        let channels = list_channels_inner(conn.as_mut(), guild.id).await.unwrap();
        assert!(channels.is_empty());
    }

    #[sqlx::test]
    async fn test_add_duplicate_channel(pool: sqlx::PgPool) {
        let mut conn = pool.acquire().await.unwrap();

        let guild = AllowedGuild::create(
            conn.as_mut(),
            AllowedGuildPayload::new(1, "Test Guild".to_string()),
        )
        .await
        .unwrap();

        AllowedChannel::create(
            conn.as_mut(),
            AllowedChannelPayload::new(1000, "Test Channel".to_string(), guild.id),
        )
        .await
        .unwrap();

        let result =
            add_channel_inner(conn.as_mut(), 1000, "Test Channel".to_string(), guild.id).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Canal já existe na lista");
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
}
