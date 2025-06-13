use std::fmt::Debug;
use std::sync::Arc;

use reqwest::Client;
use serde::Deserialize;

use crate::api::error::{ApiError, Result};
use crate::env::Env;
use crate::utils::BoxFuture;

#[derive(Debug, Deserialize)]
pub struct DiscordTokenResponse {
    pub access_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
}

pub trait DiscordService: Debug + Send + Sync {
    fn get_access_token(
        &self,
        env: Arc<Env>,
        code: String,
    ) -> BoxFuture<Result<DiscordTokenResponse>>;
    fn get_user_info(&self, token: String) -> BoxFuture<Result<DiscordUser>>;
    fn get_oauth_url(&self, env: &Env, state: &str) -> String;
}

#[derive(Debug, Clone)]
pub struct DiscordServiceImpl {
    client: Client,
}

impl DiscordServiceImpl {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl DiscordService for DiscordServiceImpl {
    fn get_access_token(
        &self,
        env: Arc<Env>,
        code: String,
    ) -> BoxFuture<Result<DiscordTokenResponse>> {
        Box::pin(async move {
            tracing::debug!("Exchanging authorization code for access token");

            let form_data = [
                ("client_id", env.discord_client_id.as_str()),
                ("client_secret", env.discord_client_secret.as_str()),
                ("grant_type", "authorization_code"),
                ("code", &code),
                ("redirect_uri", env.discord_oauth_redirect.as_str()),
            ];

            let response = self
                .client
                .post("https://discord.com/api/oauth2/token")
                .form(&form_data)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to send token exchange request");
                    ApiError::Http(e)
                })?;

            let response = response.error_for_status().map_err(|e| {
                tracing::error!(error = %e, "Discord token exchange failed");
                ApiError::discord_api(format!("Token exchange failed: {e}"))
            })?;

            response.json().await.map_err(|e| {
                tracing::error!(error = %e, "Failed to parse token response");
                ApiError::discord_api(e.to_string())
            })
        })
    }

    fn get_user_info(&self, token: String) -> BoxFuture<Result<DiscordUser>> {
        Box::pin(async move {
            tracing::debug!("Fetching Discord user information");

            let response = self
                .client
                .get("https://discord.com/api/users/@me")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to send user info request");
                    ApiError::Http(e)
                })?;

            let response = response.error_for_status().map_err(|e| {
                let message = format!("User info request failed: {e}");
                tracing::error!(error = %e, "Discord user info request failed");
                ApiError::discord_api(message)
            })?;

            response.json().await.map_err(|e| {
                tracing::error!(error = %e, "Failed to parse user response");
                ApiError::discord_api(e.to_string())
            })
        })
    }

    fn get_oauth_url(&self, env: &Env, token: &str) -> String {
        format!(
            "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify&state={}",
            env.discord_client_id,
            urlencoding::encode(&env.discord_oauth_redirect),
            token
        )
    }
}
