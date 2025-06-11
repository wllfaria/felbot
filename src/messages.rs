#[derive(Debug, Clone)]
pub enum TelegramAction {
    InviteUser { telegram_id: i64 },
    RemoveUser { telegram_id: i64 },
}
