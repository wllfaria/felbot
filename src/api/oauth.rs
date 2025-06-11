use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;

use super::AppState;
use super::error::{ApiError, Result};
use super::models::oauth_state::OAuthState;
use super::models::user_links::{UserLink, UserLinkPayload};
use crate::messages::TelegramAction;
use crate::templates::oauth_success_page;

#[derive(Debug, Deserialize)]
pub struct OAuthStartQueryParams {
    pub telegram_id: i64,
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
    id: i64,
    username: String,
}

#[tracing::instrument(skip(state), fields(telegram_id = params.telegram_id))]
pub async fn oauth_start(
    Query(params): Query<OAuthStartQueryParams>,
    State(state): State<AppState>,
) -> Result<Redirect> {
    tracing::info!("Starting OAuth flow");

    let mut tx = state.pool.acquire().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to acquire database connection");
        e
    })?;

    let telegram_id = params.telegram_id;
    let link_exists = UserLink::find_by_telegram_id(tx.as_mut(), telegram_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check existing telegram link");
            e
        })?
        .is_some();

    if link_exists {
        let message = "Telegram account is already linked to a Discord account".to_string();
        tracing::warn!(telegram_id = %telegram_id, "{}", message);
        return Err(ApiError::ForbiddenRequest { message });
    }

    let token = uuid::Uuid::new_v4().to_string();
    OAuthState::create(tx.as_mut(), telegram_id, &token)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create OAuth state");
            e
        })?;

    let discord_oauth_url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify&state={}",
        state.env.discord_client_id,
        urlencoding::encode(&state.env.discord_oauth_redirect),
        token
    );

    tracing::info!(redirect_url = %discord_oauth_url, "Redirecting to Discord OAuth");
    Ok(Redirect::to(&discord_oauth_url))
}

#[tracing::instrument(skip(state), fields(state_token = %params.state))]
pub async fn oauth_callback(
    Query(params): Query<OAuthCallbackQueryParams>,
    State(state): State<AppState>,
) -> Result<Html<String>> {
    tracing::info!("Processing OAuth callback");

    let mut tx = state.pool.acquire().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to acquire database connection");
        e
    })?;

    let Some(oauth_state) = OAuthState::get_and_delete(tx.as_mut(), &params.state)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to retrieve OAuth state");
            e
        })?
    else {
        tracing::warn!("Invalid or expired OAuth state token");
        return Err(ApiError::ForbiddenRequest {
            message: "Invalid or expired authorization request".to_string(),
        });
    };

    let telegram_id = oauth_state.telegram_id;
    tracing::info!(telegram_id = %telegram_id, "Found valid OAuth state");

    let client = reqwest::Client::new();

    tracing::debug!("Exchanging authorization code for access token");
    let token_response = client
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("client_id", state.env.discord_client_id.as_str()),
            ("client_secret", state.env.discord_client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", params.code.as_str()),
            ("redirect_uri", state.env.discord_oauth_redirect.as_str()),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to send token exchange request");
            e
        })?
        .error_for_status()
        .map_err(|e| {
            tracing::error!(error = %e, "Discord token exchange failed");
            ApiError::DiscordApi {
                message: format!("Token exchange failed: {e}"),
            }
        })?
        .json::<DiscordTokenResponse>()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to parse token response");
            e
        })?;

    tracing::debug!("Fetching Discord user information");
    let discord_user = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token_response.access_token)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to send user info request");
            e
        })?
        .error_for_status()
        .map_err(|e| {
            tracing::error!(error = %e, "Discord user info request failed");
            ApiError::DiscordApi {
                message: format!("User info request failed: {e}"),
            }
        })?
        .json::<DiscordUser>()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to parse user response");
            e
        })?;

    let discord_id = discord_user.id;
    tracing::info!(discord_id = %discord_id, username = %discord_user.username, "Retrieved Discord user info");

    let link_exists = UserLink::find_by_discord_id(tx.as_mut(), discord_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check existing Discord link");
            e
        })?
        .is_some();

    if link_exists {
        tracing::warn!(discord_id = %discord_id, "Discord account already linked");
        return Err(ApiError::ForbiddenRequest {
            message: "Discord account is already linked to a Telegram account".to_string(),
        });
    }

    tracing::info!("Creating user link");
    let user_link =
        UserLink::create_link(tx.as_mut(), UserLinkPayload::new(discord_id, telegram_id))
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create user link");
                e
            })?;

    let action = TelegramAction::InviteUser { telegram_id };

    if let Err(e) = state.telegram_sender.send(action) {
        tracing::error!(error = %e, telegram_id = %telegram_id, "Failed to send telegram invite action");
    } else {
        tracing::info!(telegram_id = %telegram_id, "Sent telegram invite action");
    }

    UserLink::mark_added_to_group(tx.as_mut(), &user_link.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to mark user as added to group");
            e
        })?;

    tracing::info!(
        discord_id = %discord_id,
        telegram_id = %telegram_id,
        username = %discord_user.username,
        "Successfully linked accounts"
    );

    let success_html = oauth_success_page(&discord_user.username);
    Ok(Html(success_html.into_string()))
}
