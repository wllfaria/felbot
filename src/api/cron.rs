use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

use super::AppState;
use super::error::{ApiError, Result};
use crate::messages::CronAction;

#[derive(Debug, Serialize)]
pub struct CronResponse {
    ok: bool,
}

#[derive(Debug, Deserialize)]
pub struct CronQuery {
    secret: String,
}

pub async fn cron_start(
    State(state): State<AppState>,
    Query(params): Query<CronQuery>,
) -> Result<Json<CronResponse>> {
    if state.env.cron_secret != params.secret {
        return Err(ApiError::ForbiddenRequest {
            message: String::from("invalid cron secret"),
        });
    }

    if state.cron_sender.send(CronAction::Execute).is_err() {
        let message = String::from("failed start cron job manually");
        return Err(ApiError::InternalError { message });
    }

    Ok(Json(CronResponse { ok: true }))
}
