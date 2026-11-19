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
            assistants,
            dto::{
                AddMemberRequest, ClassInput, ClassProgress, ClassQuery, ClassResponse, CopyRequest, MemberQuery,
                MemberResponse, PathInput, PathResponse, QuickAssignInput, QuickAssignResponse,
            },
            path, service as classes,
        },
        exams::{
            dto::{
                AttemptQuery, AttemptSummary, AttemptView, ExamInput, ExamQuery, ExamResponse, ExamSummary,
                GradeItemInput, PapersInput, PapersResponse, QueueItem,
            },
            service as exams,
            sets::{self as exam_sets, ExamSetDetail, ExamSetInput, ExamSetSummary},
        },
        lessons::{
            dto::{LessonInput, LessonQuery, LessonResponse, LessonSummary, RevisionSummary},
            service as lessons,
        },
        questions::{
            dto::{
                BulkCreateRequest, BulkCreateResponse, FolderInput, FolderResponse, ImportPreview, ImportQuery,
                QuestionInput, QuestionQuery, QuestionResponse, RegradeResponse, SaveQuestionResponse,
                WordImportRequest,
            },
            service as questions,
        },
    },
    pagination::Page,
    security::Actor,
};
use uuid::Uuid;

use sls_core::modules::{
    groups,
    lesson_plans::{self as plans, LessonPlanDetail, LessonPlanInput, LessonPlanQuery, LessonPlanSummary},
    sessions::{dto as session_dto, service as sessions},
    snippets::{self, Snippet, SnippetInput},
};

use crate::extract::{Client, JsonBody, PathParam, QueryParams, ValidatedJson};

pub fn routes() -> Router<AppContext> {
    Router::new()
        .merge(group_routes())
        // Bài học
        .route("/lessons", get(list_lessons).post(create_lesson))
        .route("/lessons/{id}", get(get_lesson).put(update_lesson).delete(archive_lesson))
        .route("/lessons/{id}/publish", post(publish_lesson))
        .route("/lessons/{id}/revisions", get(lesson_revisions))
        // Giáo án (M34)
        .route("/lesson-plans", get(list_plans).post(create_plan))
        .route("/lesson-plans/{id}", get(get_plan).put(update_plan).delete(archive_plan))
        .route("/lesson-plans/{id}/copy", post(copy_plan))
        // Lớp học
        .route("/classes", get(list_classes).post(create_class))
        .route("/classes/{id}", get(get_class).put(update_class))
        .route("/classes/{id}/archive", post(archive_class))
        .route("/classes/{id}/restore", post(restore_class))
        .route("/classes/{id}/copy", post(copy_class))
        .route("/classes/{id}/invite-code", post(regenerate_code))
        .route("/classes/{id}/members", get(list_members).post(add_member))
        .route("/classes/{id}/members/{user_id}/{action}", post(change_member))
        .route("/classes/{id}/assistants", get(list_assistants).post(add_assistant))
        .route("/classes/{id}/assistants/{user_id}", axum::routing::delete(remove_assistant))
        .route("/classes/{id}/path", get(get_path).put(save_path))
        .route("/classes/{id}/path/publish", post(publish_path))
        .route("/classes/{id}/progress", get(class_progress))
        .route("/classes/{id}/quick-assign", post(quick_assign))
        // Bộ đề (M41)
        .route("/exam-sets", get(list_sets).post(create_set))
        .route("/exam-sets/{id}", get(get_set).put(update_set).delete(archive_set))
        // Câu hỏi
        .route("/questions", get(list_questions).post(create_question))
        .route("/questions/bulk", post(bulk_create))
        .route("/questions/import/template", get(import_template))
        .route("/questions/import/preview", post(import_preview))
        .route("/questions/import/word-template", get(import_word_template))
        .route("/questions/import/word/preview", post(import_word_preview))
        .route("/questions/{id}", get(get_question).put(update_question).delete(archive_question))
        .route("/questions/{id}/regrade", post(regrade_question))
        .route("/question-folders", get(list_folders).post(create_folder))
        .route("/question-folders/{id}", axum::routing::put(update_folder).delete(delete_folder))
        // Đề
        .route("/exams", get(list_exams).post(create_exam))
        .route("/exams/{id}", get(get_exam).put(update_exam).delete(archive_exam))
        .route("/exams/{id}/preview", post(preview_exam))
        .route("/exams/{id}/attempts", get(exam_attempts))
        .route("/exams/{id}/papers", get(exam_papers).post(generate_papers))
        // Chấm tự luận (M39)
        .route("/grading", get(grading_queue))
        .route("/attempts/{id}/items/{position}/grade", axum::routing::put(grade_item))
        .route("/comment-snippets", get(list_snippets).post(create_snippet))
        .route("/comment-snippets/{id}", axum::routing::delete(delete_snippet))
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

// ---------- Giáo án (M34) ----------

async fn list_plans(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<LessonPlanQuery>,
) -> AppResult<Json<Vec<LessonPlanSummary>>> {
    Ok(Json(plans::list(&ctx, &actor, q).await?))
}

async fn create_plan(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<LessonPlanInput>,
) -> AppResult<(StatusCode, Json<LessonPlanDetail>)> {
    Ok((StatusCode::CREATED, Json(plans::create(&ctx, &actor, input).await?)))
}

async fn get_plan(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<LessonPlanDetail>> {
    Ok(Json(plans::get(&ctx, &actor, id).await?))
}

async fn update_plan(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<LessonPlanInput>,
) -> AppResult<Json<LessonPlanDetail>> {
    Ok(Json(plans::update(&ctx, &actor, id, input).await?))
}

async fn copy_plan(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<(StatusCode, Json<LessonPlanDetail>)> {
    Ok((StatusCode::CREATED, Json(plans::copy(&ctx, &actor, id).await?)))
}

async fn archive_plan(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    plans::archive(&ctx, &actor, id).await?;
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

// ---------- Trợ giảng (M32) ----------

async fn list_assistants(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<Vec<assistants::AssistantView>>> {
    Ok(Json(assistants::list(&ctx, &actor, id).await?))
}

async fn add_assistant(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(req): ValidatedJson<AddMemberRequest>,
) -> AppResult<(StatusCode, Json<assistants::AssistantView>)> {
    Ok((StatusCode::CREATED, Json(assistants::add(&ctx, &actor, id, req).await?)))
}

async fn remove_assistant(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, user_id)): PathParam<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    assistants::remove(&ctx, &actor, id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
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

async fn quick_assign(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<QuickAssignInput>,
) -> AppResult<(StatusCode, Json<QuickAssignResponse>)> {
    Ok((StatusCode::CREATED, Json(path::quick_assign(&ctx, &actor, id, input, client.ip.as_deref()).await?)))
}

async fn list_sets(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<ExamSetSummary>>> {
    Ok(Json(exam_sets::list(&ctx, &actor).await?))
}

async fn get_set(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ExamSetDetail>> {
    Ok(Json(exam_sets::get(&ctx, &actor, id).await?))
}

async fn create_set(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<ExamSetInput>,
) -> AppResult<(StatusCode, Json<ExamSetDetail>)> {
    Ok((StatusCode::CREATED, Json(exam_sets::create(&ctx, &actor, input).await?)))
}

async fn update_set(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<ExamSetInput>,
) -> AppResult<Json<ExamSetDetail>> {
    Ok(Json(exam_sets::update(&ctx, &actor, id, input).await?))
}

async fn archive_set(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    exam_sets::archive(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
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

async fn import_word_template(actor: Actor) -> AppResult<impl IntoResponse> {
    actor.require(sls_core::security::Permission::TeachingManage)?;
    let bytes = questions::import_word_template()?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"mau-cau-hoi.docx\""),
        ],
        bytes,
    ))
}

/// File Word upload trước lên Garage (`POST /api/files`, mục đích `import`) — có thể tới 20 MB kèm ảnh.
async fn import_word_preview(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(req): JsonBody<WordImportRequest>,
) -> AppResult<Json<ImportPreview>> {
    Ok(Json(questions::import_word_preview(&ctx, &actor, req).await?))
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

// ---------- Mã đề (M40) ----------

async fn exam_papers(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<PapersResponse>> {
    Ok(Json(exams::papers(&ctx, &actor, id).await?))
}

async fn generate_papers(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<PapersInput>,
) -> AppResult<(StatusCode, Json<PapersResponse>)> {
    Ok((StatusCode::CREATED, Json(exams::generate_papers(&ctx, &actor, id, input).await?)))
}

// ---------- Chấm tự luận (M39) ----------

async fn grading_queue(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<QueueItem>>> {
    Ok(Json(exams::grading_queue(&ctx, &actor).await?))
}

async fn grade_item(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, position)): PathParam<(Uuid, i32)>,
    JsonBody(input): JsonBody<GradeItemInput>,
) -> AppResult<Json<AttemptView>> {
    Ok(Json(exams::grade_item(&ctx, &actor, id, position, input).await?))
}

async fn list_snippets(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<Snippet>>> {
    Ok(Json(snippets::list(&ctx, &actor).await?))
}

async fn create_snippet(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<SnippetInput>,
) -> AppResult<(StatusCode, Json<Snippet>)> {
    Ok((StatusCode::CREATED, Json(snippets::create(&ctx, &actor, input).await?)))
}

async fn delete_snippet(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    snippets::delete(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Nhóm học sinh (M13) ----------

pub fn group_routes() -> Router<AppContext> {
    Router::new()
        .route("/students", get(list_students))
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/{id}", get(get_group).put(update_group).delete(delete_group))
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{id}", axum::routing::put(update_session))
        .route("/sessions/{id}/cancel", post(cancel_session))
        .route("/sessions/{id}/attendance", axum::routing::put(set_attendance))
}

async fn list_students(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<groups::StudentQuery>,
) -> AppResult<Json<Vec<groups::StudentRef>>> {
    Ok(Json(groups::students(&ctx, &actor, q).await?))
}

async fn list_groups(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<groups::GroupResponse>>> {
    Ok(Json(groups::list(&ctx, &actor).await?))
}

async fn get_group(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<groups::GroupDetail>> {
    Ok(Json(groups::get(&ctx, &actor, id).await?))
}

async fn create_group(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(input): ValidatedJson<groups::GroupInput>,
) -> AppResult<(StatusCode, Json<groups::GroupDetail>)> {
    Ok((StatusCode::CREATED, Json(groups::create(&ctx, &actor, input).await?)))
}

async fn update_group(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(input): ValidatedJson<groups::GroupInput>,
) -> AppResult<Json<groups::GroupDetail>> {
    Ok(Json(groups::update(&ctx, &actor, id, input).await?))
}

async fn delete_group(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    groups::delete(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Lịch & buổi học (M14) ----------

async fn list_sessions(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<session_dto::RangeQuery>,
) -> AppResult<Json<Vec<session_dto::SessionSummary>>> {
    Ok(Json(sessions::list_owned(&ctx, &actor, q).await?))
}

async fn create_session(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(input): ValidatedJson<session_dto::CreateSessionInput>,
) -> AppResult<(StatusCode, Json<session_dto::CreateSessionResponse>)> {
    Ok((StatusCode::CREATED, Json(sessions::create(&ctx, &actor, input).await?)))
}

async fn update_session(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    ValidatedJson(input): ValidatedJson<session_dto::UpdateSessionInput>,
) -> AppResult<Json<session_dto::UpdateSessionResponse>> {
    Ok(Json(sessions::update(&ctx, &actor, id, input).await?))
}

async fn cancel_session(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<session_dto::CancelInput>,
) -> AppResult<Json<session_dto::UpdateSessionResponse>> {
    Ok(Json(sessions::cancel(&ctx, &actor, id, input).await?))
}

async fn set_attendance(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<session_dto::AttendanceInput>,
) -> AppResult<StatusCode> {
    sessions::set_attendance(&ctx, &actor, id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}
