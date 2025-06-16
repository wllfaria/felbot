use crate::discord::Context;
use crate::discord::commands::create_standard_reply;
use crate::discord::error::Result;
use crate::discord::permissions::is_admin;
use crate::messages::CronAction;

#[poise::command(slash_command, rename = "checar_membros", check = "is_admin")]
pub async fn verify_members(ctx: Context<'_>) -> Result<()> {
    match ctx.data().cron_sender.send(CronAction::Execute) {
        Ok(_) => {
            let message = "Verificação de membros iniciada com sucesso".to_string();
            let reply = create_standard_reply(message);
            ctx.send(reply).await.map_err(|e| {
                tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send verify members command response");
                e
            })?;
        }
        Err(_) => {
            let message = "Falha ao iniciar verificação de membros".to_string();
            let reply = create_standard_reply(message);
            ctx.send(reply).await.map_err(|e| {
                tracing::error!(error = %e, user_id = %ctx.author().id, "Failed to send verify members command response");
                e
            })?;
        }
    }

    Ok(())
}
