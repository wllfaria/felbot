use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgConnection;
use validator::Validate;

use super::AppState;
use super::error::{ApiError, Result};
use super::models::oauth_state::OAuthState;
use super::models::user_links::{UserLink, UserLinkPayload};
use crate::env::Env;
use crate::messages::TelegramAction;
use crate::templates::oauth_success_page;

#[derive(Debug, Deserialize, Validate)]
pub struct OAuthStartQueryParams {
    #[validate(range(min = 1))]
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
    id: String,
    username: String,
}

#[tracing::instrument(skip(state), fields(telegram_id = params.telegram_id))]
pub async fn oauth_start(
    Query(params): Query<OAuthStartQueryParams>,
    State(state): State<AppState>,
) -> Result<Redirect> {
    tracing::info!("Starting OAuth flow");

    if params.validate().is_err() {
        let message = String::from("invalid discord id for oauth flow");
        tracing::warn!("{message}");
        return Err(ApiError::BadRequest { message });
    };

    let mut tx = match state.pool.acquire().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(error = %e, "Failed to acquire database connection");
            return Err(ApiError::Database(e));
        }
    };

    let link_exists = match UserLink::find_by_telegram_id(tx.as_mut(), params.telegram_id).await {
        Ok(link) => link.is_some(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to check existing telegram link");
            return Err(ApiError::Database(e));
        }
    };

    if link_exists {
        let message = "Telegram account is already linked to a Discord account".to_string();
        tracing::warn!(telegram_id = %params.telegram_id, "{}", message);
        return Err(ApiError::ForbiddenRequest { message });
    }

    let token = uuid::Uuid::new_v4().to_string();
    if let Err(e) = OAuthState::create(tx.as_mut(), params.telegram_id, &token).await {
        tracing::error!(error = %e, "Failed to create OAuth state");
        return Err(ApiError::Database(e));
    }

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

    let mut tx = match state.pool.acquire().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(error = %e, "Failed to acquire database connection");
            return Err(ApiError::Database(e));
        }
    };

    let oauth_state = get_oauth_state(tx.as_mut(), &params.state).await?;

    let telegram_id = oauth_state.telegram_id;
    tracing::info!(telegram_id = %telegram_id, "Found valid OAuth state");

    let client = reqwest::Client::new();
    let discord_token = get_discord_access_token(&state.env, &params.code, &client).await?;
    let discord_user = get_discord_user(&discord_token.access_token, &client).await?;

    tracing::info!(
        discord_id = %discord_user.id,
        username = %discord_user.username,
        "Retrieved Discord user info"
    );

    tracing::info!("Creating user link");

    let discord_id = match discord_user.id.parse::<i64>() {
        Ok(id) => id,
        Err(_) => return Err(ApiError::discord_api("Invalid discord id".into())),
    };

    if let Err(e) = can_link_accounts(tx.as_mut(), discord_id).await {
        tracing::warn!("{e}");
        return Err(e);
    }

    let user_link = create_user_link(tx.as_mut(), discord_id, telegram_id).await?;
    let action = TelegramAction::InviteUser { telegram_id };

    match state.telegram_sender.send(action) {
        Ok(_) => tracing::info!(telegram_id = %telegram_id, "Sent telegram invite action"),
        Err(e) => tracing::error!(
            error = %e,
            telegram_id = %telegram_id,
            "Failed to send telegram invite action"
        ),
    }

    if let Err(e) = UserLink::mark_added_to_group(tx.as_mut(), &user_link.id).await {
        tracing::error!(error = %e, "Failed to mark user as added to group");
        return Err(ApiError::Database(e));
    }

    tracing::info!(
        discord_id = %discord_id,
        telegram_id = %telegram_id,
        username = %discord_user.username,
        "Successfully linked accounts"
    );

    let success_html = oauth_success_page(&discord_user.username);
    Ok(Html(success_html.into_string()))
}

async fn get_oauth_state(conn: &mut PgConnection, token: &str) -> Result<OAuthState> {
    match OAuthState::get_and_delete(conn, token).await {
        Ok(Some(oauth_state)) => Ok(oauth_state),
        Ok(None) => {
            let message = "Invalid or expired authorization request".to_string();
            tracing::warn!("{message}");
            Err(ApiError::ForbiddenRequest { message })
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to retrieve OAuth state");
            Err(ApiError::Database(e))
        }
    }
}

async fn get_discord_access_token(
    env: &Env,
    code: &str,
    client: &Client,
) -> Result<DiscordTokenResponse> {
    tracing::debug!("Exchanging authorization code for access token");

    let form_data = [
        ("client_id", env.discord_client_id.as_str()),
        ("client_secret", env.discord_client_secret.as_str()),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", env.discord_oauth_redirect.as_str()),
    ];

    let token_result = client
        .post("https://discord.com/api/oauth2/token")
        .form(&form_data)
        .send()
        .await;

    let response = match token_result {
        Ok(response) => response,
        Err(e) => {
            tracing::error!(error = %e, "Failed to send token exchange request");
            return Err(ApiError::Http(e));
        }
    };

    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(e) => {
            tracing::error!(error = %e, "Discord token exchange failed");
            return Err(ApiError::discord_api(format!("Token exchange failed: {e}")));
        }
    };

    match response.json().await {
        Ok(response) => Ok(response),
        Err(e) => {
            tracing::error!(error = %e, "Failed to parse token response");
            Err(ApiError::discord_api(e.to_string()))
        }
    }
}

async fn get_discord_user(discord_token: &str, client: &Client) -> Result<DiscordUser> {
    tracing::debug!("Fetching Discord user information");
    let user_result = client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(discord_token)
        .send()
        .await;

    let response = match user_result {
        Ok(response) => response,
        Err(e) => {
            tracing::error!(error = %e, "Failed to send user info request");
            return Err(ApiError::Http(e));
        }
    };

    let response = match response.error_for_status() {
        Ok(response) => response,
        Err(e) => {
            let message = format!("User info request failed: {e}");
            tracing::error!(error = %e, "Discord user info request failed");
            return Err(ApiError::discord_api(message));
        }
    };

    match response.json().await {
        Ok(response) => Ok(response),
        Err(e) => {
            tracing::error!(error = %e, "Failed to parse user response");
            Err(ApiError::discord_api(e.to_string()))
        }
    }
}

async fn can_link_accounts(conn: &mut PgConnection, discord_id: i64) -> Result<bool> {
    match UserLink::find_by_discord_id(conn, discord_id).await? {
        Some(_) => {
            let message = "Discord account is already linked to a Telegram account".to_string();
            Err(ApiError::bad_request(message))
        }
        None => Ok(true),
    }
}

async fn create_user_link(
    conn: &mut PgConnection,
    discord_id: i64,
    telegram_id: i64,
) -> Result<UserLink> {
    can_link_accounts(conn, discord_id).await?;
    let payload = UserLinkPayload::new(discord_id, telegram_id);
    let user_link = UserLink::create_link(conn, payload).await?;
    Ok(user_link)
}

// fn user_has_required_roles(user_roles: &[u64], allowed_roles: &[u64]) -> bool {
//     user_roles.iter().any(|role| allowed_roles.contains(role))
// }
