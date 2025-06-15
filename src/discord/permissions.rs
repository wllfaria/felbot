use super::Context;
use super::error::{Error, PermissionError};
use crate::database::models::allowed_channels::AllowedChannel;
use crate::database::models::allowed_guilds::AllowedGuild;
use crate::database::models::allowed_roles::AllowedRole;

async fn is_on_guild(ctx: Context<'_>) -> Result<bool, Error> {
    let Some(guild_id) = ctx.guild_id() else {
        let message = "Esse comando não pode ser usado nesse servidor".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    };

    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    let allowed_guild_ids = AllowedGuild::get_guild_ids(conn.as_mut()).await?;

    let user_is_on_allowed_guild = allowed_guild_ids.contains(&guild_id.get());
    if !user_is_on_allowed_guild {
        let message = "Esse comando não pode ser usado nesse servidor".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    }

    Ok(true)
}

async fn is_on_channel(ctx: Context<'_>) -> Result<bool, Error> {
    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    let allowed_channel_ids = AllowedChannel::get_channel_ids(conn.as_mut()).await?;
    let user_is_on_allowed_channel = allowed_channel_ids.contains(&ctx.channel_id().get());

    if !user_is_on_allowed_channel {
        let message = "Esse comando não pode ser usado nesse canal".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    }

    Ok(true)
}

pub async fn is_admin(ctx: Context<'_>) -> Result<bool, Error> {
    is_on_guild(ctx).await?;
    is_on_channel(ctx).await?;

    let Some(member) = ctx.author_member().await else {
        let message = "Não consegui verificar seus cargos".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    };

    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    let admin_roles = AllowedRole::get_admin_ids(conn.as_mut()).await?;

    let user_has_allowed_role = member
        .roles
        .iter()
        .any(|role_id| admin_roles.contains(&role_id.get()));

    Ok(user_has_allowed_role)
}

pub async fn is_subscriber(ctx: Context<'_>) -> Result<bool, Error> {
    is_on_guild(ctx).await?;
    is_on_channel(ctx).await?;

    let Some(member) = ctx.author_member().await else {
        let message = "Não consegui verificar seus cargos".to_string();
        return Err(Error::Permission(PermissionError::new(message)));
    };

    let pool = &ctx.data().pool;
    let mut conn = pool.acquire().await?;

    let allowed_role_ids = AllowedRole::get_role_ids(conn.as_mut()).await?;

    let user_has_allowed_role = member
        .roles
        .iter()
        .any(|role_id| allowed_role_ids.contains(&role_id.get()));

    Ok(user_has_allowed_role)
}
