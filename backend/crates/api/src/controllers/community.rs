//! Đợt 4: báo cáo lớp học (M17), hỏi đáp (M18), thẻ ghi nhớ (M19), phụ huynh (M20).
//!
//! - `routes()`: gắn dưới `/api` (học sinh / mọi tài khoản; vài route phụ huynh không cần đăng nhập).
//! - `teaching_routes()`: gắn dưới `/api/teaching` (giáo viên).

use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post, put},
};
use serde::Deserialize;
use sls_core::{
    AppContext, AppResult,
    modules::{
        classes::{dto::DueDeck, path, report},
        discussions::{
            self, PostInput, QaCounts, ReportInput, ReportQuery, ReportView, ResolveInput, ThreadDetail, ThreadInput,
            ThreadQuery, ThreadSummary,
        },
        flashcards::{
            self, DeckDetail, DeckInput, DeckQuery, DeckSummary, FromQuestionsInput, ReviewInput, ReviewResult,
            StudyDeck,
        },
        guardians::{
            self, ConsentInfo, ConsentInput, GuardianInput, GuardianUpdate, GuardianView, ManualReportInput,
            SentReport, TokenQuery,
        },
    },
    pagination::Page,
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{JsonBody, PathParam, QueryParams};

#[derive(Debug, Deserialize)]
pub struct ActionBody {
    pub action: String,
}

pub fn routes() -> Router<AppContext> {
    Router::new()
        // Hỏi đáp
        .route("/classes/{id}/threads", get(list_threads).post(create_thread))
        .route("/classes/{id}/qa-counts", get(qa_counts))
        .route("/threads/{id}", get(get_thread).put(update_thread).delete(delete_thread))
        .route("/threads/{id}/posts", post(reply))
        .route("/threads/{id}/moderate", post(moderate_thread))
        .route("/posts/{id}", put(update_post).delete(delete_post))
        .route("/posts/{id}/moderate", post(moderate_post))
        .route("/content-reports", post(report_content))
        // Thẻ ghi nhớ
        .route("/classes/{id}/steps/{step_id}/deck", get(step_deck))
        .route("/classes/{id}/steps/{step_id}/reviews", post(review_card))
        .route("/my/reviews", get(my_reviews))
        // Phụ huynh (học sinh khai)
        .route("/my/guardians", get(my_guardians).post(add_guardian))
        .route("/my/guardians/{id}", put(update_guardian).delete(remove_guardian))
        .route("/my/guardians/{id}/resend", post(resend_consent))
        // Phụ huynh bấm link trong email (không cần đăng nhập)
        .route("/guardians/consent", get(consent_info).post(consent_respond))
        .route("/guardians/subscription", get(subscription_info))
        .route("/guardians/unsubscribe", post(unsubscribe))
        .route("/guardians/subscribe", post(subscribe))
}

pub fn teaching_routes() -> Router<AppContext> {
    Router::new()
        .route("/classes/{id}/report", get(class_report))
        .route("/classes/{id}/report.xlsx", get(class_report_xlsx))
        .route("/classes/{id}/content-reports", get(content_reports))
        .route("/content-reports/{id}/resolve", post(resolve_report))
        .route("/classes/{id}/students/{user_id}/guardian-report", post(send_guardian_report))
        .route("/decks", get(list_decks).post(create_deck))
        .route("/decks/{id}", get(get_deck).put(update_deck).delete(archive_deck))
        .route("/decks/{id}/from-questions", post(deck_from_questions))
}

// ---------- Báo cáo lớp học ----------

async fn class_report(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<report::ClassReport>> {
    Ok(Json(report::class_report(&ctx, &actor, id).await?))
}

async fn class_report_xlsx(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<impl IntoResponse> {
    let (name, bytes) = report::report_xlsx(&ctx, &actor, id).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\"")),
        ],
        bytes,
    ))
}

// ---------- Hỏi đáp ----------

async fn list_threads(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    QueryParams(q): QueryParams<ThreadQuery>,
) -> AppResult<Json<Page<ThreadSummary>>> {
    Ok(Json(discussions::list(&ctx, &actor, id, q).await?))
}

async fn create_thread(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<ThreadInput>,
) -> AppResult<(StatusCode, Json<ThreadDetail>)> {
    Ok((StatusCode::CREATED, Json(discussions::create(&ctx, &actor, id, input).await?)))
}

async fn qa_counts(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<QaCounts>> {
    Ok(Json(discussions::counts(&ctx, &actor, id).await?))
}

async fn get_thread(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ThreadDetail>> {
    Ok(Json(discussions::get(&ctx, &actor, id).await?))
}

async fn update_thread(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<ThreadInput>,
) -> AppResult<Json<ThreadDetail>> {
    Ok(Json(discussions::update(&ctx, &actor, id, input).await?))
}

async fn delete_thread(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    discussions::delete(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reply(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<PostInput>,
) -> AppResult<(StatusCode, Json<ThreadDetail>)> {
    Ok((StatusCode::CREATED, Json(discussions::reply(&ctx, &actor, id, input).await?)))
}

async fn moderate_thread(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(body): JsonBody<ActionBody>,
) -> AppResult<Json<ThreadDetail>> {
    Ok(Json(discussions::moderate_thread(&ctx, &actor, id, &body.action).await?))
}

async fn update_post(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<PostInput>,
) -> AppResult<Json<ThreadDetail>> {
    Ok(Json(discussions::update_post(&ctx, &actor, id, input).await?))
}

async fn delete_post(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<ThreadDetail>> {
    Ok(Json(discussions::delete_post(&ctx, &actor, id).await?))
}

async fn moderate_post(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(body): JsonBody<ActionBody>,
) -> AppResult<Json<ThreadDetail>> {
    Ok(Json(discussions::moderate_post(&ctx, &actor, id, &body.action).await?))
}

async fn report_content(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<ReportInput>,
) -> AppResult<StatusCode> {
    discussions::report(&ctx, &actor, input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn content_reports(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    QueryParams(q): QueryParams<ReportQuery>,
) -> AppResult<Json<Vec<ReportView>>> {
    Ok(Json(discussions::class_reports(&ctx, &actor, id, q).await?))
}

async fn resolve_report(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<ResolveInput>,
) -> AppResult<StatusCode> {
    discussions::resolve(&ctx, &actor, id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- Thẻ ghi nhớ ----------

async fn list_decks(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<DeckQuery>,
) -> AppResult<Json<Vec<DeckSummary>>> {
    Ok(Json(flashcards::list(&ctx, &actor, q).await?))
}

async fn create_deck(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<DeckInput>,
) -> AppResult<(StatusCode, Json<DeckDetail>)> {
    Ok((StatusCode::CREATED, Json(flashcards::create(&ctx, &actor, input).await?)))
}

async fn get_deck(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<DeckDetail>> {
    Ok(Json(flashcards::get(&ctx, &actor, id).await?))
}

async fn update_deck(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<DeckInput>,
) -> AppResult<Json<DeckDetail>> {
    Ok(Json(flashcards::update(&ctx, &actor, id, input).await?))
}

async fn archive_deck(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    flashcards::archive(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn deck_from_questions(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<FromQuestionsInput>,
) -> AppResult<Json<DeckDetail>> {
    Ok(Json(flashcards::add_from_questions(&ctx, &actor, id, input).await?))
}

async fn step_deck(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
) -> AppResult<Json<StudyDeck>> {
    Ok(Json(path::step_deck(&ctx, &actor, id, step_id).await?))
}

async fn review_card(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, step_id)): PathParam<(Uuid, Uuid)>,
    JsonBody(input): JsonBody<ReviewInput>,
) -> AppResult<Json<ReviewResult>> {
    Ok(Json(path::review_card(&ctx, &actor, id, step_id, input).await?))
}

async fn my_reviews(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<DueDeck>>> {
    Ok(Json(path::my_reviews(&ctx, &actor).await?))
}

// ---------- Phụ huynh ----------

async fn my_guardians(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<GuardianView>>> {
    Ok(Json(guardians::list_mine(&ctx, &actor).await?))
}

async fn add_guardian(
    State(ctx): State<AppContext>,
    actor: Actor,
    JsonBody(input): JsonBody<GuardianInput>,
) -> AppResult<(StatusCode, Json<GuardianView>)> {
    Ok((StatusCode::CREATED, Json(guardians::add(&ctx, &actor, input).await?)))
}

async fn update_guardian(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<GuardianUpdate>,
) -> AppResult<Json<GuardianView>> {
    Ok(Json(guardians::update(&ctx, &actor, id, input).await?))
}

async fn remove_guardian(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<StatusCode> {
    guardians::remove(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn resend_consent(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<GuardianView>> {
    Ok(Json(guardians::resend(&ctx, &actor, id).await?))
}

async fn consent_info(
    State(ctx): State<AppContext>,
    QueryParams(q): QueryParams<TokenQuery>,
) -> AppResult<Json<ConsentInfo>> {
    Ok(Json(guardians::consent_status(&ctx, q).await?))
}

async fn consent_respond(
    State(ctx): State<AppContext>,
    JsonBody(input): JsonBody<ConsentInput>,
) -> AppResult<Json<ConsentInfo>> {
    Ok(Json(guardians::respond(&ctx, input).await?))
}

async fn subscription_info(
    State(ctx): State<AppContext>,
    QueryParams(q): QueryParams<TokenQuery>,
) -> AppResult<Json<ConsentInfo>> {
    Ok(Json(guardians::subscription_info(&ctx, q).await?))
}

async fn unsubscribe(State(ctx): State<AppContext>, JsonBody(q): JsonBody<TokenQuery>) -> AppResult<Json<ConsentInfo>> {
    Ok(Json(guardians::set_subscription(&ctx, q, false).await?))
}

async fn subscribe(State(ctx): State<AppContext>, JsonBody(q): JsonBody<TokenQuery>) -> AppResult<Json<ConsentInfo>> {
    Ok(Json(guardians::set_subscription(&ctx, q, true).await?))
}

async fn send_guardian_report(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, user_id)): PathParam<(Uuid, Uuid)>,
    JsonBody(input): JsonBody<ManualReportInput>,
) -> AppResult<Json<SentReport>> {
    Ok(Json(guardians::send_manual(&ctx, &actor, id, user_id, input).await?))
}
