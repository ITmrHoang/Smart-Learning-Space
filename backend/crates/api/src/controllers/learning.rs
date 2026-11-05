//! Học sinh: tìm lớp học, ghi danh, học theo lộ trình, làm bài (M8, M9, M11).
//! - `GET /api/classes`, `GET /api/classes/{id}`: ai cũng xem được (đăng nhập thì biết trạng thái ghi danh).
//! - Còn lại cần đăng nhập; quyền vào lớp / bước / bài làm kiểm tra trong service.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post, put},
};
use sls_core::{
    AppContext, AppResult,
    modules::{
        classes::{
            dto::{
                ClassCard, ClassDetail, ClassQuery, EnrollResponse, JoinRequest, LearnView, MyClass, StartedAttempt,
                StepLesson,
            },
            path, service as classes,
        },
        exams::{
            dto::{AttemptSummary, AttemptView},
            service as exams,
        },
        questions::grading::Answer,
    },
    pagination::Page,
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{JsonBody, PathParam, QueryParams, ValidatedJson};

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/classes", get(catalog))
        .route("/classes/join", post(join))
        .route("/classes/{id}", get(detail))
        .route("/classes/{id}/enroll", post(enroll))
        .route("/classes/{id}/leave", post(leave))
        .route("/classes/{id}/learn", get(learn))
        .route("/classes/{id}/steps/{step_id}/lesson", get(step_lesson))
        .route("/classes/{id}/steps/{step_id}/complete", post(complete_step))
        .route("/classes/{id}/steps/{step_id}/attempts", get(step_attempts).post(start_exam))
        .route("/my/classes", get(my_classes))
        .route("/attempts/{id}", get(get_attempt))
        .route("/attempts/{id}/items/{position}", put(save_answer))
        .route("/attempts/{id}/items/{position}/check", post(check_item))
        .route("/attempts/{id}/submit", post(submit))
}

async fn catalog(
    State(ctx): State<AppContext>,
    actor: Option<Actor>,
    QueryParams(q): QueryParams<ClassQuery>,
) -> AppResult<Json<Page<ClassCard>>> {
    Ok(Json(classes::catalog(&ctx, actor.as_ref(), q).await?))
}

async fn detail(
    State(ctx): State<AppContext>,
    actor: Option<Actor>,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ClassDetail>> {
    Ok(Json(classes::detail(&ctx, actor.as_ref(), id).await?))
}

async fn enroll(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<EnrollResponse>> {
    Ok(Json(classes::enroll(&ctx, &actor, id).await?))
}

async fn join(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(req): ValidatedJson<JoinRequest>,
) -> AppResult<Json<EnrollResponse>> {
    Ok(Json(classes::join_by_code(&ctx, &actor, req).await?))
}

async fn leave(State(ctx): State<AppContext>, actor: Actor, PathParam(id): PathParam<Uuid>) -> AppResult<StatusCode> {
    classes::leave(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn my_classes(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<MyClass>>> {
    Ok(Json(classes::my_classes(&ctx, &actor).await?))
}

async fn learn(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<LearnView>> {
    Ok(Json(path::learn(&ctx, &actor, id).await?))
}

async fn step_lesson(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
) -> AppResult<Json<StepLesson>> {
    Ok(Json(path::step_lesson(&ctx, &actor, id, step_id).await?))
}

async fn complete_step(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
) -> AppResult<Json<LearnView>> {
    Ok(Json(path::complete_step(&ctx, &actor, id, step_id).await?))
}

async fn start_exam(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
) -> AppResult<(StatusCode, Json<StartedAttempt>)> {
    Ok((StatusCode::CREATED, Json(path::start_exam(&ctx, &actor, id, step_id).await?)))
}

async fn step_attempts(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
) -> AppResult<Json<Vec<AttemptSummary>>> {
    Ok(Json(path::step_attempts(&ctx, &actor, id, step_id).await?))
}

async fn get_attempt(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<AttemptView>> {
    Ok(Json(exams::get_attempt(&ctx, &actor, id).await?))
}

async fn save_answer(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, position)): PathParam<(Uuid, i32)>,
    JsonBody(answer): JsonBody<Answer>,
) -> AppResult<StatusCode> {
    exams::save_answer(&ctx, &actor, id, position, answer).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn check_item(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, position)): PathParam<(Uuid, i32)>,
) -> AppResult<Json<AttemptView>> {
    Ok(Json(exams::check_item(&ctx, &actor, id, position).await?))
}

async fn submit(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<AttemptView>> {
    Ok(Json(exams::submit(&ctx, &actor, id).await?))
}
