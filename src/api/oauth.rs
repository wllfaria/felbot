use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;

use super::{ApiError, ApiResult, AppState};
use crate::templates::oauth_success_page;

#[derive(Debug, Deserialize)]
pub struct OAuthStartQueryParams {
    pub telegram_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQueryParams {
    pub code: String,
    pub state: String,
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

pub async fn oauth_start(
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

pub async fn oauth_callback(
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
