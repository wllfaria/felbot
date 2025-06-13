mod command_telegram;

use std::sync::Arc;

use command_telegram::telegram;
use poise::serenity_prelude::{self as serenity};

use crate::env::Env;

pub struct Data {
    env: Arc<Env>,
}
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

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
