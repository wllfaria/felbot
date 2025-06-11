use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

pub async fn trace_requests(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|mp| mp.as_str())
        .unwrap_or(uri.path());

    let request_id = Uuid::new_v4();
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        request_id = %request_id,
    );
    let _guard = span.enter();

    tracing::info!("Received http request");

    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status();

    match status {
        _ if status.is_server_error() => tracing::event!(
            tracing::Level::ERROR,
            status = %status,
            duration_ms = duration.as_millis(),
            "Request completed"
        ),
        _ if status.is_client_error() => tracing::event!(
            tracing::Level::WARN,
            status = %status,
            duration_ms = duration.as_millis(),
            "Request completed"
        ),
        _ => tracing::event!(
            tracing::Level::INFO,
            status = %status,
            duration_ms = duration.as_millis(),
            "Request completed"
        ),
    }

    response
}
