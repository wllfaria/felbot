mod error;
mod models;
mod oauth;

use std::time::Duration;

use axum::Router;
use axum::routing::get;
use oauth::{oauth_callback, oauth_start};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;

use crate::env;
use crate::telegram::TelegramAction;

#[derive(Debug, Clone)]
pub struct AppState {
    pub telegram_sender: UnboundedSender<TelegramAction>,
    pub discord_oauth_redirect: String,
    pub discord_client_id: String,
    pub discord_client_secret: String,
    pub pool: PgPool,
}

pub async fn init(telegram_sender: UnboundedSender<TelegramAction>) {
    let port = env!("PORT");
    let database_url = env!("DATABASE_URL");
    let discord_oauth_redirect = env!("DISCORD_OAUTH_REDIRECT");
    let discord_client_id = env!("DISCORD_CLIENT_ID");
    let discord_client_secret = env!("DISCORD_CLIENT_SECRET");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    let app_state = AppState {
        discord_oauth_redirect,
        discord_client_secret,
        discord_client_id,
        telegram_sender,
        pool,
    };

    let app = Router::new()
        .route("/oauth/start", get(oauth_start))
        .route("/oauth/callback", get(oauth_callback))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}"))
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to port {port}"));

    let listener_addr = listener.local_addr().unwrap();
    tracing::info!("api ready and listening on {listener_addr}");
    axum::serve(listener, app).await.expect("web server failed");
}
