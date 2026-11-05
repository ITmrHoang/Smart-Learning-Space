//! `/api/teaching/*` — giáo viên soạn bài, lớp học, lộ trình, câu hỏi, đề (M7–M11).
//! Mọi handler kiểm tra quyền `teaching.manage` và quyền sở hữu trong service.

use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use sls_core::{
    AppContext, AppResult,
    modules::{
        classes::{
            dto::{
                AddMemberRequest, ClassInput, ClassProgress, ClassQuery, ClassResponse, CopyRequest, MemberQuery,
                MemberResponse, PathInput, PathResponse,
            },
            path, service as classes,
        },
        exams::{
            dto::{AttemptQuery, AttemptSummary, AttemptView, ExamInput, ExamQuery, ExamResponse, ExamSummary},
            service as exams,
        },
        lessons::{
            dto::{LessonInput, LessonQuery, LessonResponse, LessonSummary, RevisionSummary},
            service as lessons,
        },
        questions::{
            dto::{
                BulkCreateRequest, BulkCreateResponse, FolderInput, FolderResponse, ImportPreview, ImportQuery,
                QuestionInput, QuestionQuery, QuestionResponse, RegradeResponse, SaveQuestionResponse,
            },
            service as questions,
        },
    },
    pagination::Page,
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{Client, PathParam, QueryParams, ValidatedJson};

pub fn routes() -> Router<AppContext> {
    Router::new()
        // Bài học
        .route("/lessons", get(list_lessons).post(create_lesson))
        .route("/lessons/{id}", get(get_lesson).put(update_lesson).delete(archive_lesson))
        .route("/lessons/{id}/publish", post(publish_lesson))
        .route("/lessons/{id}/revisions", get(lesson_revisions))
        // Lớp học
        .route("/classes", get(list_classes).post(create_class))
        .route("/classes/{id}", get(get_class).put(update_class))
        .route("/classes/{id}/archive", post(archive_class))
        .route("/classes/{id}/restore", post(restore_class))
        .route("/classes/{id}/copy", post(copy_class))
        .route("/classes/{id}/invite-code", post(regenerate_code))
        .route("/classes/{id}/members", get(list_members).post(add_member))
        .route("/classes/{id}/members/{user_id}/{action}", post(change_member))
        .route("/classes/{id}/path", get(get_path).put(save_path))
        .route("/classes/{id}/path/publish", post(publish_path))
        .route("/classes/{id}/progress", get(class_progress))
        // Câu hỏi
        .route("/questions", get(list_questions).post(create_question))
        .route("/questions/bulk", post(bulk_create))
        .route("/questions/import/template", get(import_template))
        .route("/questions/import/preview", post(import_preview))
        .route("/questions/{id}", get(get_question).put(update_question).delete(archive_question))
        .route("/questions/{id}/regrade", post(regrade_question))
        .route("/question-folders", get(list_folders).post(create_folder))
        .route("/question-folders/{id}", axum::routing::put(update_folder).delete(delete_folder))
        // Đề
        .route("/exams", get(list_exams).post(create_exam))
        .route("/exams/{id}", get(get_exam).put(update_exam).delete(archive_exam))
        .route("/exams/{id}/preview", post(preview_exam))
        .route("/exams/{id}/attempts", get(exam_attempts))
}

// ---------- Bài học ----------

async fn list_lessons(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<LessonQuery>,
) -> AppResult<Json<Page<LessonSummary>>> {
    Ok(Json(lessons::list(&ctx, &actor, q).await?))
}

async fn create_lesson(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(input): ValidatedJson<LessonInput>,
) -> AppResult<(StatusCode, Json<LessonResponse>)> {
    Ok((StatusCode::CREATED, Json(lessons::create(&ctx, &actor, input).await?)))
}

async fn get_lesson(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<LessonResponse>> {
    Ok(Json(lessons::get(&ctx, &actor, id).await?))
}

async fn update_lesson(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(input): ValidatedJson<LessonInput>,
) -> AppResult<Json<LessonResponse>> {
    Ok(Json(lessons::update(&ctx, &actor, id, input).await?))
}

async fn archive_lesson(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    lessons::archive(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn publish_lesson(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<LessonResponse>> {
    Ok(Json(lessons::publish(&ctx, &actor, id, client.ip.as_deref()).await?))
}

async fn lesson_revisions(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<Vec<RevisionSummary>>> {
    Ok(Json(lessons::revisions(&ctx, &actor, id).await?))
}

// ---------- Lớp học ----------

async fn list_classes(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<ClassQuery>,
) -> AppResult<Json<Page<ClassResponse>>> {
    Ok(Json(classes::list_own(&ctx, &actor, q).await?))
}

async fn create_class(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    ValidatedJson(input): ValidatedJson<ClassInput>,
) -> AppResult<(StatusCode, Json<ClassResponse>)> {
    Ok((StatusCode::CREATED, Json(classes::create(&ctx, &actor, input, client.ip.as_deref()).await?)))
}

async fn get_class(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ClassResponse>> {
    Ok(Json(classes::get_own(&ctx, &actor, id).await?))
}

async fn update_class(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(input): ValidatedJson<ClassInput>,
) -> AppResult<Json<ClassResponse>> {
    Ok(Json(classes::update(&ctx, &actor, id, input).await?))
}

async fn archive_class(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ClassResponse>> {
    Ok(Json(classes::set_archived(&ctx, &actor, id, true).await?))
}

async fn restore_class(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ClassResponse>> {
    Ok(Json(classes::set_archived(&ctx, &actor, id, false).await?))
}

async fn copy_class(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<CopyRequest>,
) -> AppResult<(StatusCode, Json<ClassResponse>)> {
    Ok((StatusCode::CREATED, Json(classes::copy(&ctx, &actor, id, req).await?)))
}

async fn regenerate_code(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ClassResponse>> {
    Ok(Json(classes::regenerate_code(&ctx, &actor, id).await?))
}

async fn list_members(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    QueryParams(q): QueryParams<MemberQuery>,
) -> AppResult<Json<Vec<MemberResponse>>> {
    Ok(Json(classes::members(&ctx, &actor, id, q).await?))
}

async fn add_member(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<AddMemberRequest>,
) -> AppResult<(StatusCode, Json<MemberResponse>)> {
    Ok((StatusCode::CREATED, Json(classes::add_member(&ctx, &actor, id, req).await?)))
}

async fn change_member(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, user_id, action)): PathParam<(Uuid, Uuid, String)>,
) -> AppResult<StatusCode> {
    classes::change_member(&ctx, &actor, id, user_id, &action).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_path(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<PathResponse>> {
    Ok(Json(path::get_path(&ctx, &actor, id).await?))
}

async fn save_path(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(input): ValidatedJson<PathInput>,
) -> AppResult<Json<PathResponse>> {
    Ok(Json(path::save_path(&ctx, &actor, id, input).await?))
}

async fn publish_path(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<PathResponse>> {
    Ok(Json(path::publish_path(&ctx, &actor, id, client.ip.as_deref()).await?))
}

async fn class_progress(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ClassProgress>> {
    Ok(Json(path::class_progress(&ctx, &actor, id).await?))
}

// ---------- Câu hỏi ----------

async fn list_questions(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<QuestionQuery>,
) -> AppResult<Json<Page<QuestionResponse>>> {
    Ok(Json(questions::list(&ctx, &actor, q).await?))
}

async fn create_question(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(input): ValidatedJson<QuestionInput>,
) -> AppResult<(StatusCode, Json<QuestionResponse>)> {
    Ok((StatusCode::CREATED, Json(questions::create(&ctx, &actor, input).await?)))
}

async fn get_question(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<QuestionResponse>> {
    Ok(Json(questions::get(&ctx, &actor, id).await?))
}

async fn update_question(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(input): ValidatedJson<QuestionInput>,
) -> AppResult<Json<SaveQuestionResponse>> {
    Ok(Json(questions::update(&ctx, &actor, id, input).await?))
}

async fn archive_question(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    questions::archive(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn regrade_question(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<RegradeResponse>> {
    Ok(Json(questions::regrade(&ctx, &actor, id).await?))
}

async fn bulk_create(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(req): ValidatedJson<BulkCreateRequest>,
) -> AppResult<(StatusCode, Json<BulkCreateResponse>)> {
    Ok((StatusCode::CREATED, Json(questions::bulk_create(&ctx, &actor, req).await?)))
}

async fn import_template(actor: Actor) -> AppResult<impl IntoResponse> {
    actor.require(sls_core::security::Permission::TeachingManage)?;
    let bytes = questions::import_template()?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"mau-cau-hoi.xlsx\""),
        ],
        bytes,
    ))
}

/// Body là nội dung file .xlsx (không dùng multipart). Tối đa 1 MB như mọi request.
async fn import_preview(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<ImportQuery>,
    body: Bytes,
) -> AppResult<Json<ImportPreview>> {
    Ok(Json(questions::import_preview(&ctx, &actor, q.subject_id, &body).await?))
}

async fn list_folders(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<FolderResponse>>> {
    Ok(Json(questions::list_folders(&ctx, &actor).await?))
}

async fn create_folder(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(input): ValidatedJson<FolderInput>,
) -> AppResult<(StatusCode, Json<FolderResponse>)> {
    Ok((StatusCode::CREATED, Json(questions::create_folder(&ctx, &actor, input).await?)))
}

async fn update_folder(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(input): ValidatedJson<FolderInput>,
) -> AppResult<StatusCode> {
    questions::update_folder(&ctx, &actor, id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_folder(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    questions::delete_folder(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Đề ----------

async fn list_exams(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<ExamQuery>,
) -> AppResult<Json<Page<ExamSummary>>> {
    Ok(Json(exams::list(&ctx, &actor, q).await?))
}

async fn create_exam(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(input): ValidatedJson<ExamInput>,
) -> AppResult<(StatusCode, Json<ExamResponse>)> {
    Ok((StatusCode::CREATED, Json(exams::create(&ctx, &actor, input).await?)))
}

async fn get_exam(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ExamResponse>> {
    Ok(Json(exams::get(&ctx, &actor, id).await?))
}

async fn update_exam(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(input): ValidatedJson<ExamInput>,
) -> AppResult<Json<ExamResponse>> {
    Ok(Json(exams::update(&ctx, &actor, id, input).await?))
}

async fn archive_exam(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    exams::archive(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn preview_exam(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<AttemptView>> {
    Ok(Json(exams::preview(&ctx, &actor, id).await?))
}

async fn exam_attempts(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    QueryParams(q): QueryParams<AttemptQuery>,
) -> AppResult<Json<Vec<AttemptSummary>>> {
    Ok(Json(exams::exam_attempts(&ctx, &actor, id, q).await?))
}
