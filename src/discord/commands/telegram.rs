use poise::CreateReply;

use crate::discord::Context;
use crate::discord::commands::create_embed;
use crate::discord::error::Error;
use crate::discord::permissions::is_subscriber;

#[poise::command(
    slash_command,
    check = "is_subscriber",
    description_localized("pt-BR", "Inicia o processo de entrar no grupo do Telegram")
)]
pub async fn telegram(ctx: Context<'_>) -> Result<(), Error> {
    let user = ctx.author();

    tracing::info!(user_id = %user.id, username = %user.name, "Processing /telegram command");

    let embed = create_embed("Oi! eu vou te ajudar no processo de entrar no grupo do telegram!".into())
        .field("Como funciona?", "Pra entrar no grupo do telegram você precisa vincular sua conta do discord com a conta do telegram, mas relaxa que isso é facinho", false)
        // .field("E o que eu faço?", "Você precisa falar comigo lá no telegram, e eu vou te falar o que fazer.\n\n[Só clicar aqui](https://t.me/telefelps_bot)", false)
        .field("E o que eu faço?", "Você precisa falar comigo lá no telegram, e eu vou te falar o que fazer.\n\n[Só clicar aqui](https://t.me/felptestbot)", false)
        .field("E depois?", "Depois que você vincular sua conta, eu vou te mandar uma mensagem pra te colocar no grupo, isso talvez demore alguns minutinhos, mas vai acontecer. Ah é, lembrando que você precisa ser sub na twitch ou membro no tutubs", false);

    let reply = CreateReply::default().embed(embed).ephemeral(true);

    ctx.send(reply).await.map_err(|e| {
        tracing::error!(error = %e, user_id = %user.id, "Failed to send telegram command response");
        e
    })?;

    tracing::info!(user_id = %user.id, "Telegram command response sent successfully");
    Ok(())
}
