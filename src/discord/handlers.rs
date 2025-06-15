use poise::{CreateReply, FrameworkError, serenity_prelude as serenity};

use super::{Data, Error};

pub async fn error_handler(error: FrameworkError<'_, Data, Error>) {
    match error {
        FrameworkError::Setup { error, .. } => {
            tracing::error!(error = %error, "Error during setup");
        }

        FrameworkError::Command { error, ctx, .. } => {
            tracing::error!(
                error = %error,
                user = %ctx.author().name,
                command = %ctx.command().qualified_name,
                "Command error"
            );

            let author = serenity::CreateEmbedAuthor::new("Erro");
            let embed = serenity::CreateEmbed::new()
                .color((255, 0, 0)) // Red color for errors
                .description(format!(
                    "Ocorreu um erro ao processar o comando:\n\n{}",
                    error
                ))
                .author(author);

            let reply = CreateReply::default().embed(embed).ephemeral(true);

            if let Err(e) = ctx.send(reply).await {
                tracing::error!(error = %e, "Failed to send error message");
            }
        }

        FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            let command_name = ctx.command().qualified_name.clone();
            let user_name = ctx.author().name.clone();
            tracing::warn!(
                error = ?error,
                user = %user_name,
                command = %command_name,
                "Command check failed with error"
            );

            let description = match error {
                Some(error) => error.to_string(),
                None => "Você não tem permissão para usar esse comando.".to_string(),
            };

            let author = serenity::CreateEmbedAuthor::new("Permissão Negada");
            let footer = serenity::CreateEmbedFooter::new(format!("Comando: /{}", command_name));
            let embed = serenity::CreateEmbed::new()
                .color((255, 62, 117))
                .description(description)
                .author(author)
                .footer(footer);

            let reply = CreateReply::default().embed(embed).ephemeral(true);

            if let Err(e) = ctx.send(reply).await {
                tracing::error!(error = %e, "Failed to send permission error message");
            }
        }

        FrameworkError::ArgumentParse { error, ctx, .. } => {
            tracing::warn!(
                error = %error,
                user = %ctx.author().name,
                command = %ctx.command().qualified_name,
                "Failed to parse command arguments"
            );

            let author = serenity::CreateEmbedAuthor::new("Erro de Parâmetros");
            let embed = serenity::CreateEmbed::new()
                .color((255, 62, 117))
                .description(format!("Parâmetros incorretos:\n\n{}", error))
                .author(author);

            let reply = CreateReply::default().embed(embed).ephemeral(true);

            if let Err(e) = ctx.send(reply).await {
                tracing::error!(error = %e, "Failed to send argument error message");
            }
        }

        error => {
            tracing::error!(error = %error, "Discord framework error");
        }
    }
}
