mod allowed_channels;
mod telegram;

pub use allowed_channels::channels;
use poise::{CreateReply, serenity_prelude as serenity};
pub use telegram::telegram;

pub fn create_embed(description: String) -> serenity::CreateEmbed {
    let author = serenity::CreateEmbedAuthor::new("felbot");
    let footer = serenity::CreateEmbedFooter::new("a carinha '-'")
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
