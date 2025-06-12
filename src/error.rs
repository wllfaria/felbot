use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use derive_more::{Display, Error, From};

use crate::api::error::ApiError;

#[derive(Debug, Display, Error, From)]
pub enum AppError {
    #[from]
    Api(ApiError),
    #[from]
    Database(sqlx::Error),
    #[from]
    Discord(poise::serenity_prelude::Error),
    #[from]
    Telegram(teloxide::RequestError),
}

pub type Result<T> = std::result::Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Api(api_error) => api_error.into_response(),
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred").into_response()
            }
            AppError::Discord(e) => {
                tracing::error!(error = %e, "Discord API error");
                (StatusCode::BAD_GATEWAY, "Discord service unavailable").into_response()
            }
            AppError::Telegram(e) => {
                tracing::error!(error = %e, "Telegram API error");
                (StatusCode::BAD_GATEWAY, "Telegram service unavailable").into_response()
            }
        }
    }
}
