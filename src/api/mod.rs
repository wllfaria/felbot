mod error;
pub mod models;
mod oauth;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use oauth::{oauth_callback, oauth_start};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;

use crate::env::Env;
use crate::messages::TelegramAction;

#[derive(Debug, Clone)]
pub struct AppState {
    pub telegram_sender: UnboundedSender<TelegramAction>,
    pub env: Arc<Env>,
    pub pool: PgPool,
}

pub async fn init(env: Arc<Env>, pool: PgPool, telegram_sender: UnboundedSender<TelegramAction>) {
    let app_state = AppState {
        telegram_sender,
        pool,
        env: env.clone(),
    };

    let app = Router::new()
        .route("/oauth/start", get(oauth_start))
        .route("/oauth/callback", get(oauth_callback))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", env.port))
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to port {}", env.port));

    let listener_addr = listener.local_addr().unwrap();
    tracing::info!("api ready and listening on {listener_addr}");
    axum::serve(listener, app).await.expect("web server failed");
}
