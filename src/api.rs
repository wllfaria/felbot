use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::get;
use derive_more::{Display, Error, From};
use serde::Deserialize;

use crate::env;
use crate::templates::{oauth_error_page, oauth_success_page};

#[derive(Debug, Display, Error, From)]
enum ApiError {
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

type ApiResult<T> = Result<T, ApiError>;

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
    telegram_id: Option<String>,
}

async fn oauth_start(
    Query(params): Query<OAuthStartQueryParams>,
    State(state): State<AppState>,
) -> ApiResult<Redirect> {
    let _telegram_id = params.telegram_id.ok_or(ApiError::MissingParameter {
        parameter: "telegram_id".to_string(),
    })?;

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
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
}

async fn oauth_callback(
    Query(params): Query<OAuthCallbackQueryParams>,
    State(state): State<AppState>,
) -> ApiResult<Html<String>> {
    let client = reqwest::Client::new();

    // Exchange code for access token
    let token_response = client
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", state.discord_client_id.as_str()),
            ("client_secret", state.discord_client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", params.code.as_str()),
            ("redirect_uri", state.discord_oauth_redirect.as_str()),
        ])
        .send()
        .await?
        .error_for_status()
        .map_err(|e| ApiError::DiscordApi {
            message: format!("Token exchange failed: {e}"),
        })?
        .json::<DiscordTokenResponse>()
        .await?;

    // Get Discord user info
    let discord_user = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token_response.access_token)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| ApiError::DiscordApi {
            message: format!("User info request failed: {e}"),
        })?
        .json::<DiscordUser>()
        .await?;

    // TODO: Store the link between discord_user.id and telegram_id
    // For now, just return success page
    let success_html = oauth_success_page(&discord_user.username);
    Ok(Html(success_html.into_string()))
}
