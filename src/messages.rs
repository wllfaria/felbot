#[derive(Debug, Clone)]
pub enum TelegramAction {
    InviteUser {
        telegram_id: String,
        discord_id: String,
    },
    RemoveUser {
        telegram_id: String,
        discord_username: String,
        reason: String,
    },
}
