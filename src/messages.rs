#[derive(Debug, Clone)]
pub enum TelegramAction {
    InviteUser { id: i64, group_id: i64 },
    RemoveUser { id: i64, group_id: i64 },
}

#[derive(Debug, Clone)]
pub enum CronAction {
    Execute,
}
