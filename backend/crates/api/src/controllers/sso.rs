//! SSO (M6 phần 1): xác nhận liên kết tài khoản, quản lý liên kết.
//! Các route chuyển hướng tới Google / Microsoft (`/auth/sso/{provider}/start|callback`) thêm ở phần 2.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
};
use axum_extra::extract::cookie::CookieJar;
use sls_core::{
    AppContext, AppResult,
    modules::{
        auth::dto::AuthResponse,
        sso::{
            dto::{ConfirmLinkRequest, IdentityResponse, LinkTokenRequest},
            service as sso,
        },
    },
    security::Actor,
};
use uuid::Uuid;

use super::auth::session_response;
use crate::extract::{Client, PathParam, ValidatedJson};

/// Gắn dưới `/api/auth`.
pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/sso/link/send-code", post(send_link_code))
        .route("/sso/link", post(confirm_link))
        .route("/identities", get(list_identities))
        .route("/identities/{provider}", delete(unlink))
}

/// Gắn dưới `/api/admin`.
pub fn admin_routes() -> Router<AppContext> {
    Router::new()
        .route("/users/{id}/identities", get(admin_list))
        .route("/users/{id}/identities/{provider}", delete(admin_unlink))
}

async fn send_link_code(
    State(ctx): State<AppContext>,
    Client(client): Client,
    ValidatedJson(req): ValidatedJson<LinkTokenRequest>,
) -> AppResult<StatusCode> {
    sso::send_link_code(&ctx, req, &client).await?;
    Ok(StatusCode::ACCEPTED)
}

async fn confirm_link(
    State(ctx): State<AppContext>,
    Client(client): Client,
    jar: CookieJar,
    ValidatedJson(req): ValidatedJson<ConfirmLinkRequest>,
) -> AppResult<(CookieJar, Json<AuthResponse>)> {
    let session = sso::confirm_link(&ctx, req, &client).await?;
    Ok(session_response(&ctx, jar, session))
}

async fn list_identities(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<IdentityResponse>>> {
    Ok(Json(sso::list_identities(&ctx, &actor).await?))
}

async fn unlink(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(provider): PathParam<String>,
) -> AppResult<StatusCode> {
    sso::unlink(&ctx, &actor, &provider, client.ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_list(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<Vec<IdentityResponse>>> {
    Ok(Json(sso::admin_list_identities(&ctx, &actor, id).await?))
}

async fn admin_unlink(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam((id, provider)): PathParam<(Uuid, String)>,
) -> AppResult<StatusCode> {
    sso::admin_unlink(&ctx, &actor, id, &provider, client.ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}
