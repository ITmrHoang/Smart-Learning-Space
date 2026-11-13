//! File (M21): xin link upload, báo upload xong, mở file (chuyển hướng sang presigned URL của Garage).

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use sls_core::{
    AppContext, AppResult,
    modules::files::{self, FileView, OpenQuery, UploadInput, UploadTicket},
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{JsonBody, PathParam, QueryParams};

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/files", post(create_upload))
        .route("/files/{id}", get(open_file))
        .route("/files/{id}/complete", post(complete_upload))
}

async fn create_upload(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<UploadInput>,
) -> AppResult<(StatusCode, Json<UploadTicket>)> {
    Ok((StatusCode::CREATED, Json(files::create_upload(&ctx, &actor, input).await?)))
}

async fn complete_upload(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<FileView>> {
    Ok(Json(files::complete(&ctx, &actor, id).await?))
}

/// Không cần đăng nhập (để `<img>` trong nội dung hiện được); file bài làm phải có chữ ký trong link.
async fn open_file(
    State(ctx): State<AppContext>,
    PathParam(id): PathParam<Uuid>,
    QueryParams(q): QueryParams<OpenQuery>,
) -> AppResult<impl IntoResponse> {
    let url = files::open(&ctx, id, q).await?;
    // Presigned URL sống 1 giờ → cho trình duyệt nhớ chuyển hướng 50 phút.
    Ok((StatusCode::FOUND, [(header::LOCATION, url), (header::CACHE_CONTROL, "private, max-age=3000".to_string())]))
}
