//! `/api/admin/users/*` — admin quản lý user (M3).

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use sls_core::{
    AppContext, AppResult,
    modules::users::{
        dto::{CreateUserRequest, UpdateUserRequest, UserListQuery, UserResponse},
        service as users,
    },
    pagination::Page,
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{Client, PathParam, QueryParams, ValidatedJson};

pub fn admin_routes() -> Router<AppContext> {
    Router::new()
        .route("/users", get(list).post(create))
        .route("/users/{id}", get(get_one).patch(update))
        .route("/users/{id}/lock", post(lock))
        .route("/users/{id}/unlock", post(unlock))
        .route("/users/{id}/send-password-reset", post(send_password_reset))
}

async fn list(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<UserListQuery>,
) -> AppResult<Json<Page<UserResponse>>> {
    Ok(Json(users::list(&ctx, &actor, q).await?))
}

async fn get_one(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<UserResponse>> {
    Ok(Json(users::get(&ctx, &actor, id).await?))
}

async fn create(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    ValidatedJson(req): ValidatedJson<CreateUserRequest>,
) -> AppResult<(StatusCode, Json<UserResponse>)> {
    let user = users::admin_create(&ctx, &actor, req, client.ip.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

async fn update(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    Ok(Json(users::admin_update(&ctx, &actor, id, req, client.ip.as_deref()).await?))
}

async fn lock(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<UserResponse>> {
    Ok(Json(users::lock(&ctx, &actor, id, client.ip.as_deref()).await?))
}

async fn unlock(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<UserResponse>> {
    Ok(Json(users::unlock(&ctx, &actor, id, client.ip.as_deref()).await?))
}

async fn send_password_reset(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    users::send_password_reset(&ctx, &actor, id, client.ip.as_deref()).await?;
    Ok(StatusCode::ACCEPTED)
}
