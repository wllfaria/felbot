use axum::Router;
use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use axum::routing::get;
use serde::Deserialize;

use crate::env;

#[derive(Debug, Clone)]
struct AppState {
    discord_oauth_redirect: String,
    discord_client_id: String,
    discord_client_secret: String,
}

pub async fn init() {
    let port = env!("PORT");
    let discord_oauth_redirect = env!("DISCORD_OAUTH_REDIRECT");
    let discord_client_id = env!("DISCORD_CLIENT_ID");
    let discord_client_secret = env!("DISCORD_CLIENT_SECRET");

    let app_state = AppState {
        discord_oauth_redirect,
        discord_client_id,
        discord_client_secret,
    };

    let app = Router::new()
        .route("/oauth/start", get(oauth_start))
        .route("/oauth/callback", get(oauth_callback))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}"))
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to port {port}"));

    axum::serve(listener, app).await.expect("web server failed");
}

#[derive(Debug, Deserialize)]
struct OAuthStartQueryParams {
    telegram_id: String,
}

async fn oauth_start(
    Query(params): Query<OAuthStartQueryParams>,
    State(state): State<AppState>,
) -> Result<Redirect, String> {
    let oauth_state = uuid::Uuid::new_v4();

    let discord_oauth_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify&state={}",
        state.discord_client_id,
        urlencoding::encode(&state.discord_oauth_redirect),
        oauth_state
    );

    Ok(Redirect::to(&discord_oauth_url))
}

#[derive(Debug, Deserialize)]
struct OAuthCallbackQueryParams {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct DiscordTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    refresh_token: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
}

async fn oauth_callback(
    Query(params): Query<OAuthCallbackQueryParams>,
    State(state): State<AppState>,
) -> Result<Html<String>, String> {
    let client = reqwest::Client::new();

    let token_response = client
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", &state.discord_client_id),
            ("client_secret", &state.discord_client_secret),
        ])
        .send()
        .await?
        .json::<DiscordTokenResponse>()
        .await?;

    let discord_user = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token_response.access_token)
        .send()
        .await?
        .json::<DiscordUser>()
        .await?;
}
