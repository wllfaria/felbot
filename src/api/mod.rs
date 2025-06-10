mod error;
mod oauth;
mod state;

use axum::Router;
use axum::routing::get;
use error::ApiError;
use oauth::{oauth_callback, oauth_start};
use state::AppState;

use crate::env;

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
