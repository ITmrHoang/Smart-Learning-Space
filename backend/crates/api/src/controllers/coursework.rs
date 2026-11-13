//! Đợt 5: bài tập về nhà (M22), đặt lịch học kèm 1-1 (M23), hồ sơ giáo viên công khai (M25).
//!
//! - `routes()`: gắn dưới `/api` (học sinh / mọi tài khoản; trang công khai không cần đăng nhập).
//! - `teaching_routes()`: gắn dưới `/api/teaching` (giáo viên).

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use sls_core::{
    AppContext, AppError, AppResult,
    modules::{
        assignments::{self, AssignmentDetail, AssignmentInput, AssignmentQuery, AssignmentSummary},
        classes::homework::{
            self, GradeInput, HomeworkStepSummary, HomeworkView, StepSubmissions, SubmissionView, SubmitInput,
        },
        profiles::{self, MyProfile, ProfileInput, TeacherPublic},
        tutoring::{self, BookInput, BookingQuery, BookingView, OfferInput, OfferView, ReasonInput, Slot},
    },
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{JsonBody, PathParam, QueryParams};

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/classes/{id}/steps/{step_id}/homework", get(step_homework).put(submit_homework))
        // Học kèm 1-1 (học sinh)
        .route("/tutoring/offers", get(visible_offers))
        .route("/tutoring/offers/{id}", get(get_offer))
        .route("/tutoring/offers/{id}/slots", get(offer_slots))
        .route("/tutoring/offers/{id}/bookings", post(book))
        .route("/my/bookings", get(my_bookings))
        .route("/my/bookings/{id}/cancel", post(cancel_booking))
        // Hồ sơ giáo viên công khai (không cần đăng nhập)
        .route("/public/teachers/{key}", get(teacher_public))
}

/// Trang HTML cho máy tìm kiếm — gắn ở gốc (không nằm dưới `/api`); Caddy / Vite chuyển các đường dẫn này vào api.
pub fn seo_routes() -> Router<AppContext> {
    Router::new()
        .route("/p/giao-vien/{username}", get(seo_teacher))
        .route("/p/lop-hoc/{id}", get(seo_class))
        .route("/sitemap.xml", get(sitemap))
        .route("/robots.txt", get(robots))
}

pub fn teaching_routes() -> Router<AppContext> {
    Router::new()
        .route("/assignments", get(list_assignments).post(create_assignment))
        .route("/assignments/{id}", get(get_assignment).put(update_assignment).delete(archive_assignment))
        .route("/classes/{id}/homework", get(class_homework))
        .route("/classes/{id}/steps/{step_id}/submissions", get(step_submissions))
        .route("/submissions/{id}", get(submission_detail))
        .route("/submissions/{id}/grade", post(grade_submission))
        .route("/tutoring/offers", get(my_offers).post(create_offer))
        .route("/tutoring/offers/{id}", axum::routing::put(update_offer))
        .route("/tutoring/bookings", get(teacher_bookings))
        .route("/tutoring/bookings/{id}/approve", post(approve_booking))
        .route("/tutoring/bookings/{id}/decline", post(decline_booking))
        .route("/tutoring/bookings/{id}/cancel", post(cancel_booking))
        .route("/profile", get(my_profile).put(update_profile))
}

// ---------- Bài tập (học sinh) ----------

async fn step_homework(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
) -> AppResult<Json<HomeworkView>> {
    Ok(Json(homework::student_view(&ctx, &actor, id, step_id).await?))
}

async fn submit_homework(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
    JsonBody(input): JsonBody<SubmitInput>,
) -> AppResult<Json<HomeworkView>> {
    Ok(Json(homework::submit(&ctx, &actor, id, step_id, input).await?))
}

// ---------- Bài tập (giáo viên) ----------

async fn list_assignments(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<AssignmentQuery>,
) -> AppResult<Json<Vec<AssignmentSummary>>> {
    Ok(Json(assignments::list(&ctx, &actor, q).await?))
}

async fn create_assignment(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<AssignmentInput>,
) -> AppResult<(StatusCode, Json<AssignmentDetail>)> {
    Ok((StatusCode::CREATED, Json(assignments::create(&ctx, &actor, input).await?)))
}

async fn get_assignment(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<AssignmentDetail>> {
    Ok(Json(assignments::get(&ctx, &actor, id).await?))
}

async fn update_assignment(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<AssignmentInput>,
) -> AppResult<Json<AssignmentDetail>> {
    Ok(Json(assignments::update(&ctx, &actor, id, input).await?))
}

async fn archive_assignment(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    assignments::archive(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn class_homework(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<Vec<HomeworkStepSummary>>> {
    Ok(Json(homework::class_homework(&ctx, &actor, id).await?))
}

async fn step_submissions(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
) -> AppResult<Json<StepSubmissions>> {
    Ok(Json(homework::step_submissions(&ctx, &actor, id, step_id).await?))
}

async fn submission_detail(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<SubmissionView>> {
    Ok(Json(homework::submission_detail(&ctx, &actor, id).await?))
}

async fn grade_submission(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<GradeInput>,
) -> AppResult<Json<SubmissionView>> {
    Ok(Json(homework::grade(&ctx, &actor, id, input).await?))
}

// ---------- Học kèm 1-1 ----------

async fn my_offers(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<OfferView>>> {
    Ok(Json(tutoring::my_offers(&ctx, &actor).await?))
}

async fn create_offer(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<OfferInput>,
) -> AppResult<(StatusCode, Json<OfferView>)> {
    Ok((StatusCode::CREATED, Json(tutoring::create_offer(&ctx, &actor, input).await?)))
}

async fn update_offer(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<OfferInput>,
) -> AppResult<Json<OfferView>> {
    Ok(Json(tutoring::update_offer(&ctx, &actor, id, input).await?))
}

async fn teacher_bookings(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<BookingQuery>,
) -> AppResult<Json<Vec<BookingView>>> {
    Ok(Json(tutoring::teacher_bookings(&ctx, &actor, q).await?))
}

async fn approve_booking(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<BookingView>> {
    Ok(Json(tutoring::approve(&ctx, &actor, id).await?))
}

async fn decline_booking(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<ReasonInput>,
) -> AppResult<Json<BookingView>> {
    Ok(Json(tutoring::decline(&ctx, &actor, id, input).await?))
}

/// Giáo viên (`/api/teaching/...`) hoặc học sinh (`/api/my/...`) hủy lịch.
async fn cancel_booking(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<ReasonInput>,
) -> AppResult<Json<BookingView>> {
    Ok(Json(tutoring::cancel(&ctx, &actor, id, input).await?))
}

async fn visible_offers(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<OfferView>>> {
    Ok(Json(tutoring::visible_offers(&ctx, &actor, None).await?))
}

async fn get_offer(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<OfferView>> {
    Ok(Json(tutoring::get_offer(&ctx, &actor, id).await?))
}

async fn offer_slots(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<Vec<Slot>>> {
    Ok(Json(tutoring::slots(&ctx, &actor, id).await?))
}

async fn book(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<BookInput>,
) -> AppResult<(StatusCode, Json<BookingView>)> {
    Ok((StatusCode::CREATED, Json(tutoring::book(&ctx, &actor, id, input).await?)))
}

async fn my_bookings(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<BookingQuery>,
) -> AppResult<Json<Vec<BookingView>>> {
    Ok(Json(tutoring::my_bookings(&ctx, &actor, q).await?))
}

// ---------- Hồ sơ giáo viên ----------

async fn my_profile(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<MyProfile>> {
    Ok(Json(profiles::my_profile(&ctx, &actor).await?))
}

async fn update_profile(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<ProfileInput>,
) -> AppResult<Json<MyProfile>> {
    Ok(Json(profiles::update_profile(&ctx, &actor, input).await?))
}

async fn teacher_public(
    State(ctx): State<AppContext>,
    actor: Option<Actor>,
    PathParam(key): PathParam<String>,
) -> AppResult<Json<TeacherPublic>> {
    Ok(Json(profiles::teacher_public(&ctx, actor.as_ref(), &key).await?))
}

// ---------- Trang cho máy tìm kiếm ----------

/// Trang HTML; lỗi "không tìm thấy" cũng trả HTML (người mở link chia sẻ thấy trang dễ đọc).
fn html_page(result: AppResult<String>) -> Response {
    match result {
        Ok(body) => ([(header::CACHE_CONTROL, "public, max-age=300")], Html(body)).into_response(),
        Err(e) if e.status == StatusCode::NOT_FOUND => (
            StatusCode::NOT_FOUND,
            Html(format!(
                "<!doctype html><html lang=\"vi\"><head><meta charset=\"utf-8\">\
                 <title>Không tìm thấy — Smart Learning Space</title><meta name=\"robots\" content=\"noindex\"></head>\
                 <body><h1>Không tìm thấy</h1><p>{}</p><p><a href=\"/\">Về trang chủ</a></p></body></html>",
                profiles::esc(&e.message)
            )),
        )
            .into_response(),
        Err(e) => e.into_response(),
    }
}

async fn seo_teacher(State(ctx): State<AppContext>, PathParam(username): PathParam<String>) -> Response {
    html_page(profiles::teacher_page(&ctx, &username).await)
}

async fn seo_class(State(ctx): State<AppContext>, PathParam(id): PathParam<String>) -> Response {
    match Uuid::parse_str(&id) {
        Ok(id) => html_page(profiles::class_page(&ctx, id).await),
        Err(_) => html_page(Err(AppError::not_found("Không tìm thấy lớp học."))),
    }
}

async fn sitemap(State(ctx): State<AppContext>) -> AppResult<impl IntoResponse> {
    let body = profiles::sitemap(&ctx).await?;
    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8"), (header::CACHE_CONTROL, "public, max-age=3600")],
        body,
    ))
}

async fn robots(State(ctx): State<AppContext>) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], profiles::robots(&ctx))
}
