mod allowed_channels;
mod allowed_roles;
mod telegram;
mod verify_members;

pub use allowed_channels::channels;
pub use allowed_roles::roles;
use chrono::Timelike;
use poise::{CreateReply, serenity_prelude as serenity};
pub use telegram::telegram;
pub use verify_members::verify_members;

use super::error::{Error, InvalidGuildError, Result};
use crate::database::models::allowed_guilds::AllowedGuild;

pub fn get_meiafelps_formatted_date() -> String {
    let now = chrono::Utc::now();

    let (pm, _) = now.hour12();
    let hour = now.hour() + 24;
    let minutes = now.minute();

    let suffix = if pm { "'-'" } else { "=m" };
    format!("{hour}:{minutes} {suffix}")
}

pub fn create_embed(description: String) -> serenity::CreateEmbed {
    let author = serenity::CreateEmbedAuthor::new("felbot");
    let footer_message = format!("Agora são {}", get_meiafelps_formatted_date());
    let footer = serenity::CreateEmbedFooter::new(footer_message)
        .icon_url("https://yt3.googleusercontent.com/c0u2JGrq6Ke9i15R66z2u3RR0fY8RHFAkrocO8cGkRu2FLhke2DH_e_zjiW17_RnBHDzQw4KlA=s160-c-k-c0x00ffffff-no-rj");

    serenity::CreateEmbed::new()
        .color((255, 62, 117))
        .description(description)
        .author(author)
        .footer(footer)
}

pub fn create_standard_reply(description: String) -> CreateReply {
    let embed = create_embed(description);
    CreateReply::default().embed(embed).ephemeral(true)
}

pub async fn validate_guild(pool: &sqlx::PgPool, guild_id: u64) -> Result<()> {
    let mut conn = pool.acquire().await?;
    let allowed_guild_ids = AllowedGuild::get_guild_ids(conn.as_mut()).await?;

    if !allowed_guild_ids.contains(&guild_id) {
        let message = "Esse canal não é um canal de um servidor permitido".to_string();
        return Err(Error::InvalidGuild(InvalidGuildError::new(message)));
    }

    Ok(())
}
