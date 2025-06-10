#[derive(Debug, Clone)]
pub struct AppState {
    pub discord_oauth_redirect: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
}
