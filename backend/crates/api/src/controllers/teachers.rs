//! Hồ sơ giáo viên (M5).

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use sls_core::{
    AppContext, AppResult,
    modules::teachers::{
        dto::{ApplicationListQuery, ApplicationResponse, RejectApplicationRequest, SubmitApplicationRequest},
        service as teachers,
    },
    pagination::Page,
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{Client, PathParam, QueryParams, ValidatedJson};

pub fn routes() -> Router<AppContext> {
    Router::new().route("/teacher-applications", post(submit)).route("/teacher-applications/me", get(mine))
}

pub fn admin_routes() -> Router<AppContext> {
    Router::new()
        .route("/teacher-applications", get(admin_list))
        .route("/teacher-applications/{id}/approve", post(approve))
        .route("/teacher-applications/{id}/reject", post(reject))
}

async fn submit(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(req): ValidatedJson<SubmitApplicationRequest>,
) -> AppResult<(StatusCode, Json<ApplicationResponse>)> {
    Ok((StatusCode::CREATED, Json(teachers::submit(&ctx, &actor, req).await?)))
}

async fn mine(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<ApplicationResponse>>> {
    Ok(Json(teachers::my_applications(&ctx, &actor).await?))
}

async fn admin_list(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<ApplicationListQuery>,
) -> AppResult<Json<Page<ApplicationResponse>>> {
    Ok(Json(teachers::admin_list(&ctx, &actor, q).await?))
}

async fn approve(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ApplicationResponse>> {
    Ok(Json(teachers::approve(&ctx, &actor, id, client.ip.as_deref()).await?))
}

async fn reject(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<RejectApplicationRequest>,
) -> AppResult<Json<ApplicationResponse>> {
    Ok(Json(teachers::reject(&ctx, &actor, id, req, client.ip.as_deref()).await?))
}
