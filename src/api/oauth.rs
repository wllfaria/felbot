use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;

use super::AppState;
use super::error::{ApiError, Result};
use super::models::oauth_state::OAuthState;
use super::models::user_links::{UserLink, UserLinkPayload};
use crate::templates::oauth_success_page;

#[derive(Debug, Deserialize)]
pub struct OAuthStartQueryParams {
    pub telegram_id: String,
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
) -> Result<Redirect> {
    let mut tx = state.pool.acquire().await?;
    let telegram_id = params.telegram_id;

    let link_exists = UserLink::find_by_telegram_id(tx.as_mut(), &telegram_id)
        .await?
        .is_some();

    if link_exists {
        return Err(ApiError::DiscordApi {
            message: "Telegram account is already linked to a Discord account".to_string(),
        });
    }

    let token = uuid::Uuid::new_v4().to_string();
    OAuthState::create(tx.as_mut(), &telegram_id, &token).await?;

    let discord_oauth_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify&state={}",
        state.discord_client_id,
        urlencoding::encode(&state.discord_oauth_redirect),
        token
    );

    Ok(Redirect::to(&discord_oauth_url))
}

pub async fn oauth_callback(
    Query(params): Query<OAuthCallbackQueryParams>,
    State(state): State<AppState>,
) -> Result<Html<String>> {
    let mut tx = state.pool.acquire().await?;

    let Some(oauth_state) = OAuthState::get_and_delete(tx.as_mut(), &params.state).await? else {
        return Err(ApiError::ForbiddenRequest {
            message: "Discord account is already linked to a Telegram account".to_string(),
        });
    };

    let client = reqwest::Client::new();

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

    let link_exists = UserLink::find_by_discord_id(tx.as_mut(), &discord_user.id)
        .await?
        .is_some();

    if link_exists {
        return Err(ApiError::DiscordApi {
            message: "Discord account is already linked to a Telegram account".to_string(),
        });
    }

    let new_link = UserLinkPayload {
        discord_id: discord_user.id,
        discord_username: discord_user.username.clone(),
        telegram_id: oauth_state.telegram_id,
    };

    let user_link = UserLink::create_link(tx.as_mut(), new_link).await?;

    // TODO: add user to telegram group

    UserLink::mark_added_to_group(tx.as_mut(), &user_link.id).await?;

    let success_html = oauth_success_page(&discord_user.username);
    Ok(Html(success_html.into_string()))
}
