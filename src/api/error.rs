use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use derive_more::{Display, Error, From};

use crate::templates::oauth_error_page;

#[derive(Debug, Display, Error, From)]
pub enum ApiError {
    #[display("Discord API error: {message}")]
    DiscordApi { message: String },

    #[display("HTTP request failed: {_0}")]
    #[from]
    Http(reqwest::Error),

    #[display("Forbidden: {message}")]
    ForbiddenRequest { message: String },

    #[from]
    Database(sqlx::Error),

    #[display("Internal server error: {message}")]
    InternalError { message: String },

    #[display("Bad request: {message}")]
    BadRequest { message: String },
}

impl ApiError {
    pub fn discord_api(message: String) -> Self {
        Self::DiscordApi { message }
    }

    pub fn bad_request(message: String) -> Self {
        Self::BadRequest { message }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ApiError::DiscordApi { .. } => (StatusCode::BAD_GATEWAY, self.to_string()),
            ApiError::Http(_) => (
                StatusCode::BAD_GATEWAY,
                "External service unavailable".to_string(),
            ),
            ApiError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error occurred".to_string(),
            ),
            ApiError::ForbiddenRequest { .. } => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::BadRequest { .. } => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::InternalError { .. } => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        match &self {
            ApiError::Database(e) => {
                tracing::error!(error = %e, "Database error in API request");
            }
            ApiError::DiscordApi { message } => {
                tracing::error!(message = %message, "Discord API error");
            }
            ApiError::Http(e) => {
                tracing::error!(error = %e, "HTTP client error");
            }
            ApiError::ForbiddenRequest { message } => {
                tracing::warn!(message = %message, "Forbidden request");
            }
            ApiError::InternalError { message } => {
                tracing::error!(message = %message, "Internal error");
            }
            ApiError::BadRequest { message } => {
                tracing::error!(message = %message, "Bad request");
            }
        }

        let body = Html(oauth_error_page(&error_message).into_string());

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;
