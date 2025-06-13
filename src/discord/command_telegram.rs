use poise::CreateReply;
use poise::serenity_prelude::{self as serenity};

use super::{Context, Error, validate_command_permissions};

#[poise::command(slash_command)]
pub async fn telegram(ctx: Context<'_>) -> Result<(), Error> {
    let user = ctx.author();
    let guild_id = ctx.guild_id();

    tracing::info!(
        user_id = %user.id,
        username = %user.name,
        guild_id = ?guild_id,
        "Processing /telegram command"
    );

    if let Err(error_message) = validate_command_permissions(ctx).await {
        tracing::warn!(
            user_id = %user.id,
            error = %error_message,
            "Command validation failed"
        );

        let reply = CreateReply::default()
            .content(error_message)
            .ephemeral(true);

        ctx.send(reply).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %user.id, "Failed to send validation error response");
            e
        })?;

        return Ok(());
    }

    let author = serenity::CreateEmbedAuthor::new("felbot");
    let footer = serenity::CreateEmbedFooter::new("a carinha '-'").icon_url("https://yt3.googleusercontent.com/c0u2JGrq6Ke9i15R66z2u3RR0fY8RHFAkrocO8cGkRu2FLhke2DH_e_zjiW17_RnBHDzQw4KlA=s160-c-k-c0x00ffffff-no-rj");
    let embed = serenity::CreateEmbed::new()
        .color((255, 62, 117))
        .description("Oi! eu vou te guiar no processo de entrar no grupo do telegram!")
        .field("Como funciona?", "Pra entrar no grupo do telegram você precisa vincular sua conta do discord com a conta do telegram, mas relaxa que isso é facinho", false)
        .field("E o que eu faço?", "Você precisa falar comigo lá no telegram, e eu vou te falar o que fazer por la.\n\n[Só clicar aqui](https://t.me/telefelps_bot)", false)
        .field("E depois?", "Depois que você vincular sua conta, você vai ser adicionado no grupo automaticamente, isso talvez demore alguns minutos, mas vai acontecer. Ah é, lembrando que você precisa ser sub na twitch ou membro no tutubs", false)
        .author(author)
        .footer(footer);

    let reply = CreateReply::default().embed(embed).ephemeral(true);

    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %user.id, "Failed to send telegram command response");
        e
    })?;

    tracing::info!(user_id = %user.id, "Telegram command response sent successfully");
    Ok(())
}
