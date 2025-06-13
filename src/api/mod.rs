mod cron;
pub mod error;
mod middleware;
mod oauth;

use std::sync::Arc;

use axum::routing::get;
use axum::{Router, middleware as axum_middleware};
use cron::cron_start;
use middleware::trace_requests;
use oauth::{oauth_callback, oauth_start};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;

use crate::env::Env;
use crate::messages::{CronAction, TelegramAction};
use crate::services::discord::{DiscordService, DiscordServiceImpl};

#[derive(Debug, Clone)]
pub struct AppState<D>
where
    D: DiscordService + ?Sized,
{
    pub telegram_sender: UnboundedSender<TelegramAction>,
    pub cron_sender: UnboundedSender<CronAction>,
    pub env: Arc<Env>,
    pub pool: PgPool,
    pub discord_service: Arc<D>,
}

pub async fn init(
    env: Arc<Env>,
    pool: PgPool,
    telegram_sender: UnboundedSender<TelegramAction>,
    cron_sender: UnboundedSender<CronAction>,
) {
    tracing::info!("Initializing API service");

    let discord_service = Arc::new(DiscordServiceImpl::new());

    let app_state = AppState {
        telegram_sender,
        cron_sender,
        pool,
        env: env.clone(),
        discord_service,
    };

    let app = Router::new()
        .route("/oauth/start", get(oauth_start))
        .route("/oauth/callback", get(oauth_callback))
        .route("/cron", get(cron_start))
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
