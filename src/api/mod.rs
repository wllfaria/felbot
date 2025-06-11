mod error;
mod middleware;
pub mod models;
mod oauth;

use std::sync::Arc;

use axum::routing::get;
use axum::{Router, middleware as axum_middleware};
use middleware::trace_requests;
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
    tracing::info!("Initializing API service");

    let app_state = AppState {
        telegram_sender,
        pool,
        env: env.clone(),
    };

    let app = Router::new()
        .route("/oauth/start", get(oauth_start))
        .route("/oauth/callback", get(oauth_callback))
        .layer(axum_middleware::from_fn(trace_requests))
        .with_state(app_state);

    let bind_addr = format!("0.0.0.0:{}", env.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(bind_addr = %bind_addr, error = %e, "Failed to bind to address");
            panic!("Failed to bind to port {}: {}", env.port, e);
        });

    let listener_addr = listener.local_addr().unwrap();
    tracing::info!(address = %listener_addr, "API service ready and listening");

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "API service failed");
    }
}
