use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use derive_more::{Display, Error, From};

use crate::templates::oauth_error_page;

#[derive(Debug, Display, Error, From)]
pub enum ApiError {
    #[display("Missing required parameter: {}", parameter)]
    MissingParameter { parameter: String },

    #[display("Invalid or expired OAuth state")]
    InvalidState,

    #[display("Discord API error: {}", message)]
    DiscordApi { message: String },

    #[display("HTTP request failed: {}", _0)]
    #[from]
    Http(reqwest::Error),

    #[display("JSON parsing failed: {}", _0)]
    Json(#[error(source)] reqwest::Error),

    #[display("Database error: {}", message)]
    Database { message: String },

    #[display("Internal server error")]
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::MissingParameter { .. } => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::InvalidState => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::DiscordApi { .. } => (StatusCode::BAD_GATEWAY, self.to_string()),
            ApiError::Http(_) => (
                StatusCode::BAD_GATEWAY,
                "External service unavailable".to_string(),
            ),
            ApiError::Json(_) => (
                StatusCode::BAD_GATEWAY,
                "Invalid response from Discord".to_string(),
            ),
            ApiError::Database { .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Html(oauth_error_page(&error_message).into_string());

        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
