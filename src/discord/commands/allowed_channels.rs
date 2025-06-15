use itertools::Itertools;
use poise::CreateReply;
use poise::serenity_prelude::{self as serenity};

use crate::database::models::allowed_channels::{AllowedChannel, AllowedChannelPayload};
use crate::database::models::allowed_guilds::AllowedGuild;
use crate::discord::error::{PermissionError, Result};
use crate::discord::permissions::is_admin;
use crate::discord::{Context, Error};

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
    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;
    let allowed_channels = AllowedChannel::get_channels(conn.as_mut()).await?;

    let formatted_channels = allowed_channels
        .into_iter()
        .map(|channel| format!("{} - {}", channel.channel_id, channel.name))
        .join("\n");

    let formatted_channels = format!("Lista de canais permitidos:\n\n{}", formatted_channels);

    let author = serenity::CreateEmbedAuthor::new("felbot");
    let footer = serenity::CreateEmbedFooter::new("a carinha '-'").icon_url("https://yt3.googleusercontent.com/c0u2JGrq6Ke9i15R66z2u3RR0fY8RHFAkrocO8cGkRu2FLhke2DH_e_zjiW17_RnBHDzQw4KlA=s160-c-k-c0x00ffffff-no-rj");
    let embed = serenity::CreateEmbed::new()
        .color((255, 62, 117))
        .description(formatted_channels)
        .author(author)
        .footer(footer);

    let reply = CreateReply::default().embed(embed).ephemeral(true);

    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send list channels command response");
        e
    })?;

    Ok(())
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

    let channel_id = id.parse::<i64>().unwrap();
    let channel_name = get_channel_name(ctx, channel_id).await?;

    let payload = AllowedChannelPayload::new(channel_id, channel_name);
    let new_channel = AllowedChannel::create(conn.as_mut(), payload).await?;

    let author = serenity::CreateEmbedAuthor::new("felbot");
    let footer = serenity::CreateEmbedFooter::new("a carinha '-'").icon_url("https://yt3.googleusercontent.com/c0u2JGrq6Ke9i15R66z2u3RR0fY8RHFAkrocO8cGkRu2FLhke2DH_e_zjiW17_RnBHDzQw4KlA=s160-c-k-c0x00ffffff-no-rj");

    let description = format!(
        "Canal adicionado com sucesso!\n\n**ID:** {}\n**Nome:** {}",
        new_channel.channel_id, new_channel.name
    );
    let embed = serenity::CreateEmbed::new()
        .color((255, 62, 117))
        .description(description)
        .author(author)
        .footer(footer);

    let reply = CreateReply::default().embed(embed).ephemeral(true);
    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send add channel command response");
        e
    })?;

    Ok(())
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

    let channel_id = id.parse::<i64>().unwrap();
    let channel_name = get_channel_name(ctx, channel_id).await?;
    AllowedChannel::delete(conn.as_mut(), channel_id).await?;

    let author = serenity::CreateEmbedAuthor::new("felbot");
    let footer = serenity::CreateEmbedFooter::new("a carinha '-'").icon_url("https://yt3.googleusercontent.com/c0u2JGrq6Ke9i15R66z2u3RR0fY8RHFAkrocO8cGkRu2FLhke2DH_e_zjiW17_RnBHDzQw4KlA=s160-c-k-c0x00ffffff-no-rj");
    let description =
        format!("Canal removido com sucesso!\n\nID: {channel_id}\nNome: {channel_name}");
    let embed = serenity::CreateEmbed::new()
        .color((255, 62, 117))
        .description(description)
        .author(author)
        .footer(footer);

    let reply = CreateReply::default().embed(embed).ephemeral(true);
    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send add channel command response");
        e
    })?;

    Ok(())
}

async fn get_channel_name(ctx: Context<'_>, channel_id: i64) -> Result<String> {
    let channel = serenity::ChannelId::new(channel_id as u64)
        .to_channel(ctx)
        .await?;

    match channel {
        serenity::Channel::Guild(guild_channel) => {
            let mut conn = ctx.data().pool.acquire().await?;

            let allowed_guild_ids = AllowedGuild::get_guild_ids(conn.as_mut()).await?;
            if !allowed_guild_ids.contains(&guild_channel.guild_id.get()) {
                let message = "Esse canal não é um canal do servidor".to_string();
                return Err(Error::Permission(PermissionError::new(message)));
            }
            Ok(guild_channel.name)
        }
        _ => {
            let message = "Esse canal não é um canal do servidor".to_string();
            Err(Error::Permission(PermissionError::new(message)))
        }
    }
}
