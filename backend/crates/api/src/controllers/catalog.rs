//! Master data (M4).
//! - `GET /api/master-data`, `GET /api/subjects/{id}/tags`: ai cũng xem được (đăng nhập thì thấy thêm mục riêng của mình).
//! - `POST /api/grades`, `POST /api/subjects/{id}/tags`: giáo viên thêm mục riêng; admin thêm mục dùng chung.
//! - `/api/admin/...`: admin quản lý, đưa lên dùng chung, gộp.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, patch, post},
};
use sls_core::{
    AppContext, AppResult,
    modules::catalog::{
        dto::{
            CreateItemRequest, CreateSubjectRequest, GradeResponse, MasterDataResponse, MergeRequest,
            PendingItemResponse, SubjectResponse, TagResponse, UpdateItemRequest,
        },
        service as catalog,
    },
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{Client, PathParam, ValidatedJson};

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/master-data", get(master_data))
        .route("/grades", post(create_grade))
        .route("/subjects/{id}/tags", get(list_tags).post(create_tag))
}

pub fn admin_routes() -> Router<AppContext> {
    Router::new()
        .route("/subjects", get(admin_subjects).post(create_subject))
        .route("/subjects/{id}", patch(update_subject))
        .route("/subjects/{id}/tags", get(admin_tags))
        .route("/grades", get(admin_grades))
        .route("/grades/{id}", patch(update_grade))
        .route("/grades/{id}/promote", post(promote_grade))
        .route("/grades/{id}/merge", post(merge_grade))
        .route("/tags/{id}", patch(update_tag))
        .route("/tags/{id}/promote", post(promote_tag))
        .route("/tags/{id}/merge", post(merge_tag))
        .route("/master-data/pending", get(pending))
}

fn created_status(created: bool) -> StatusCode {
    if created { StatusCode::CREATED } else { StatusCode::OK }
}

async fn master_data(State(ctx): State<AppContext>, actor: Option<Actor>) -> AppResult<Json<MasterDataResponse>> {
    Ok(Json(catalog::master_data(&ctx, actor.as_ref()).await?))
}

async fn list_tags(
    State(ctx): State<AppContext>,
    actor: Option<Actor>,
    PathParam(subject_id): PathParam<Uuid>,
) -> AppResult<Json<Vec<TagResponse>>> {
    Ok(Json(catalog::list_tags(&ctx, actor.as_ref(), subject_id).await?))
}

async fn create_grade(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    ValidatedJson(req): ValidatedJson<CreateItemRequest>,
) -> AppResult<(StatusCode, Json<GradeResponse>)> {
    let (grade, created) = catalog::create_grade(&ctx, &actor, req, client.ip.as_deref()).await?;
    Ok((created_status(created), Json(grade)))
}

async fn create_tag(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(subject_id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateItemRequest>,
) -> AppResult<(StatusCode, Json<TagResponse>)> {
    let (tag, created) = catalog::create_tag(&ctx, &actor, subject_id, req, client.ip.as_deref()).await?;
    Ok((created_status(created), Json(tag)))
}

async fn admin_subjects(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<SubjectResponse>>> {
    Ok(Json(catalog::admin_list_subjects(&ctx, &actor).await?))
}

async fn create_subject(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    ValidatedJson(req): ValidatedJson<CreateSubjectRequest>,
) -> AppResult<(StatusCode, Json<SubjectResponse>)> {
    Ok((StatusCode::CREATED, Json(catalog::create_subject(&ctx, &actor, req, client.ip.as_deref()).await?)))
}

async fn update_subject(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateItemRequest>,
) -> AppResult<Json<SubjectResponse>> {
    Ok(Json(catalog::update_subject(&ctx, &actor, id, req, client.ip.as_deref()).await?))
}

async fn admin_tags(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(subject_id): PathParam<Uuid>,
) -> AppResult<Json<Vec<TagResponse>>> {
    Ok(Json(catalog::admin_list_tags(&ctx, &actor, subject_id).await?))
}

async fn admin_grades(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<GradeResponse>>> {
    Ok(Json(catalog::admin_list_grades(&ctx, &actor).await?))
}

async fn update_grade(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateItemRequest>,
) -> AppResult<Json<GradeResponse>> {
    Ok(Json(catalog::update_grade(&ctx, &actor, id, req, client.ip.as_deref()).await?))
}

async fn promote_grade(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<GradeResponse>> {
    Ok(Json(catalog::promote_grade(&ctx, &actor, id, client.ip.as_deref()).await?))
}

async fn merge_grade(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<MergeRequest>,
) -> AppResult<StatusCode> {
    catalog::merge_grade(&ctx, &actor, id, req.into_id, client.ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_tag(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateItemRequest>,
) -> AppResult<Json<TagResponse>> {
    Ok(Json(catalog::update_tag(&ctx, &actor, id, req, client.ip.as_deref()).await?))
}

async fn promote_tag(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<TagResponse>> {
    Ok(Json(catalog::promote_tag(&ctx, &actor, id, client.ip.as_deref()).await?))
}

async fn merge_tag(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<MergeRequest>,
) -> AppResult<StatusCode> {
    catalog::merge_tag(&ctx, &actor, id, req.into_id, client.ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn pending(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<PendingItemResponse>>> {
    Ok(Json(catalog::pending(&ctx, &actor).await?))
}
