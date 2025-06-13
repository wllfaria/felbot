use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use serde::Deserialize;
use sqlx::PgConnection;
use validator::Validate;

use super::AppState;
use super::error::{ApiError, Result};
use crate::database::models::oauth_state::OAuthState;
use crate::database::models::user_links::{UserLink, UserLinkPayload};
use crate::messages::TelegramAction;
use crate::services::discord::DiscordService;
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

#[tracing::instrument(skip(state), fields(telegram_id = params.telegram_id))]
pub async fn oauth_start(
    Query(params): Query<OAuthStartQueryParams>,
    State(state): State<AppState<impl DiscordService>>,
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

    let discord_oauth_url = state.discord_service.get_oauth_url(&state.env, &token);
    tracing::info!(redirect_url = %discord_oauth_url, "Redirecting to Discord OAuth");
    Ok(Redirect::to(&discord_oauth_url))
}

#[tracing::instrument(skip(state), fields(state_token = %params.state))]
pub async fn oauth_callback(
    Query(params): Query<OAuthCallbackQueryParams>,
    State(state): State<AppState<impl DiscordService>>,
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

    let discord_token = state
        .discord_service
        .get_access_token(state.env.clone(), params.code)
        .await?;

    let discord_user = state
        .discord_service
        .get_user_info(discord_token.access_token)
        .await?;

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sqlx::PgPool;
    use tokio::sync::mpsc::UnboundedReceiver;

    use super::*;
    use crate::env::Env;
    use crate::messages::CronAction;
    use crate::services::discord::{DiscordService, DiscordTokenResponse, DiscordUser};
    use crate::utils::BoxFuture;

    struct TestContext<D: DiscordService> {
        params: Query<OAuthStartQueryParams>,
        state: State<AppState<D>>,
        _cron_receiver: UnboundedReceiver<CronAction>,
        _telegram_receiver: UnboundedReceiver<TelegramAction>,
    }

    #[derive(Debug, Clone)]
    struct MockDiscordService {
        discord_user: DiscordUser,
        should_fail_token: bool,
        should_fail_user_info: bool,
    }

    impl MockDiscordService {
        fn new() -> Self {
            Self {
                discord_user: DiscordUser {
                    id: "123".to_string(),
                    username: "test_user".to_string(),
                },
                should_fail_token: false,
                should_fail_user_info: false,
            }
        }

        fn with_failing_token(mut self) -> Self {
            self.should_fail_token = true;
            self
        }

        fn with_failing_user_info(mut self) -> Self {
            self.should_fail_user_info = true;
            self
        }
    }

    impl DiscordService for MockDiscordService {
        fn get_oauth_url(&self, env: &Env, token: &str) -> String {
            format!(
                "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope=identify&state={}",
                &env.discord_client_id,
                urlencoding::encode(&env.discord_oauth_redirect),
                token,
            )
        }

        fn get_access_token(
            &self,
            _: Arc<Env>,
            _: String,
        ) -> BoxFuture<Result<DiscordTokenResponse>> {
            let should_fail = self.should_fail_token;
            Box::pin(async move {
                if should_fail {
                    Err(ApiError::discord_api("Failed to get access token".into()))
                } else {
                    Ok(DiscordTokenResponse {
                        access_token: "sample_access_token".into(),
                    })
                }
            })
        }

        fn get_user_info(&self, _: String) -> BoxFuture<Result<DiscordUser>> {
            let should_fail = self.should_fail_user_info;
            let user = self.discord_user.clone();
            Box::pin(async move {
                if should_fail {
                    Err(ApiError::discord_api("Failed to get user info".into()))
                } else {
                    Ok(user)
                }
            })
        }
    }

    fn setup_test(
        pool: PgPool,
        params: OAuthStartQueryParams,
        discord_service: MockDiscordService,
    ) -> TestContext<MockDiscordService> {
        let params = Query(params);
        let (cron_sender, _cron_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (telegram_sender, _telegram_receiver) = tokio::sync::mpsc::unbounded_channel();
        let env = Arc::new(Env::empty());

        let state = State(AppState {
            telegram_sender,
            cron_sender,
            env,
            pool,
            discord_service: Arc::new(discord_service),
        });

        TestContext {
            params,
            state,
            _cron_receiver,
            _telegram_receiver,
        }
    }

    #[sqlx::test]
    async fn test_invalid_telegram_id(pool: PgPool) {
        let setup = setup_test(
            pool,
            OAuthStartQueryParams { telegram_id: -1 },
            MockDiscordService::new(),
        );

        let result = oauth_start(setup.params, setup.state).await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ApiError::BadRequest { .. })));
        assert_eq!(
            result.unwrap_err().to_string(),
            "Bad request: invalid discord id for oauth flow"
        );
    }

    #[sqlx::test]
    async fn test_successful_redirect(pool: PgPool) {
        let setup = setup_test(
            pool,
            OAuthStartQueryParams { telegram_id: 123 },
            MockDiscordService::new(),
        );

        let result = oauth_start(setup.params, setup.state).await.unwrap();
        assert!(matches!(result, Redirect { .. }));
    }

    #[sqlx::test]
    async fn test_already_linked_account(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let payload = UserLinkPayload::new(123, 456);
        UserLink::create_link(&mut conn, payload).await.unwrap();

        let setup = setup_test(
            pool,
            OAuthStartQueryParams { telegram_id: 456 },
            MockDiscordService::new(),
        );

        let result = oauth_start(setup.params, setup.state).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(ApiError::ForbiddenRequest { .. })));
    }

    #[sqlx::test]
    async fn test_successful_callback(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let token = "test_token".to_string();
        OAuthState::create(&mut conn, 123, &token).await.unwrap();

        let setup = setup_test(
            pool,
            OAuthStartQueryParams { telegram_id: 123 },
            MockDiscordService::new(),
        );

        let result = oauth_callback(
            Query(OAuthCallbackQueryParams {
                code: "test_code".to_string(),
                state: token,
            }),
            setup.state,
        )
        .await;

        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.0.contains("test_user"));
    }

    #[sqlx::test]
    async fn test_invalid_state(pool: PgPool) {
        let setup = setup_test(
            pool,
            OAuthStartQueryParams { telegram_id: 123 },
            MockDiscordService::new(),
        );

        let result = oauth_callback(
            Query(OAuthCallbackQueryParams {
                code: "test_code".to_string(),
                state: "invalid_token".to_string(),
            }),
            setup.state,
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ApiError::ForbiddenRequest { .. })));
    }

    #[sqlx::test]
    async fn test_discord_token_failure(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let token = "test_token".to_string();
        OAuthState::create(&mut conn, 123, &token).await.unwrap();

        let setup = setup_test(
            pool,
            OAuthStartQueryParams { telegram_id: 123 },
            MockDiscordService::new().with_failing_token(),
        );

        let result = oauth_callback(
            Query(OAuthCallbackQueryParams {
                code: "test_code".to_string(),
                state: token,
            }),
            setup.state,
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ApiError::DiscordApi { .. })));
    }

    #[sqlx::test]
    async fn test_user_info_failure(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let token = "test_token".to_string();
        OAuthState::create(&mut conn, 123, &token).await.unwrap();

        let setup = setup_test(
            pool,
            OAuthStartQueryParams { telegram_id: 123 },
            MockDiscordService::new().with_failing_user_info(),
        );

        let result = oauth_callback(
            Query(OAuthCallbackQueryParams {
                code: "test_code".to_string(),
                state: token,
            }),
            setup.state,
        )
        .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(ApiError::DiscordApi { .. })));
    }
}
