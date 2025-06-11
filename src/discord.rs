use std::sync::Arc;

use poise::CreateReply;
use poise::serenity_prelude::{self as serenity};

use crate::env::Env;

struct Data {
    env: Arc<Env>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn init(env: Arc<Env>) {
    tracing::info!("Initializing Discord service");

    let intents = serenity::GatewayIntents::non_privileged();

    let options = poise::FrameworkOptions {
        commands: vec![telegram()],
        ..Default::default()
    };

    let env_clone = env.clone();
    let framework = poise::Framework::builder()
        .options(options)
        .setup(move |ctx, ready, framework| Box::pin(setup(ctx, ready, framework, env_clone)))
        .build();

    let mut client = serenity::ClientBuilder::new(&env.discord_token, intents)
        .framework(framework)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create Discord client");
            e
        })
        .expect("Failed to create Discord client");

    tracing::info!("Discord client created, starting connection");

    if let Err(e) = client.start().await {
        tracing::error!(error = %e, "Discord client failed");
    }
}

async fn setup(
    ctx: &serenity::Context,
    ready: &serenity::Ready,
    framework: &poise::Framework<Data, Error>,
    env: Arc<Env>,
) -> Result<Data, Error> {
    tracing::info!(
        bot_username = %ready.user.name,
        bot_id = %ready.user.id,
        guild_count = ready.guilds.len(),
        "Discord bot connected and ready"
    );

    poise::builtins::register_globally(ctx, &framework.options().commands)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to register Discord commands globally");
            e
        })?;

    tracing::info!(
        command_count = framework.options().commands.len(),
        "Discord commands registered globally"
    );
    Ok(Data { env })
}

#[poise::command(slash_command)]
async fn telegram(ctx: Context<'_>) -> Result<(), Error> {
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

async fn validate_command_permissions(ctx: Context<'_>) -> Result<(), String> {
    let env = &ctx.data().env;

    let channel_id = ctx.channel_id();
    if channel_id.get() != env.discord_channel_id {
        return Err("Esse commando não pode ser usado nesse canal".to_string());
    }

    let member = match ctx.author_member().await {
        Some(member) => member,
        None => return Err("Não consegui verificar seus cargos".to_string()),
    };

    let user_has_allowed_role = member
        .roles
        .iter()
        .any(|role_id| env.discord_allowed_roles.contains(&role_id.get()));

    if !user_has_allowed_role {
        return Err(
            "Opa, pra entrar no grupo vc precisa ser sub na twitch ou membro no youtube"
                .to_string(),
        );
    }

    Ok(())
}
