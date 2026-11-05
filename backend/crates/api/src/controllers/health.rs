use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;
use sls_core::{AppContext, context::HealthChecks};

pub fn routes() -> Router<AppContext> {
    Router::new().route("/health", get(health))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    checks: HealthChecks,
}

/// `GET /api/health` — 200 khi mọi phụ thuộc hoạt động, 503 nếu có phụ thuộc lỗi.
async fn health(State(ctx): State<AppContext>) -> (StatusCode, Json<HealthResponse>) {
    let checks = ctx.health().await;
    let ok = checks.all_ok();
    (
        if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE },
        Json(HealthResponse { status: if ok { "ok" } else { "degraded" }, version: env!("CARGO_PKG_VERSION"), checks }),
    )
}
