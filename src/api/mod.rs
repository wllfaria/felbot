mod error;
mod models;
mod oauth;

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use oauth::{oauth_callback, oauth_start};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;

use crate::env::Env;
use crate::telegram::TelegramAction;

#[derive(Debug, Clone)]
pub struct AppState {
    pub telegram_sender: UnboundedSender<TelegramAction>,
    pub env: Arc<Env>,
    pub pool: PgPool,
}

pub async fn init(env: Arc<Env>, telegram_sender: UnboundedSender<TelegramAction>) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&env.database_url)
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

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
