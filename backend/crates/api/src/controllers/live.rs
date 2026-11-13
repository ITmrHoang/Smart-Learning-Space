//! Đợt 6: phòng học trực tuyến (LiveKit), khách vào bằng link, lưu bình chọn / bảng sau buổi.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde_json::Value;
use sls_core::{
    AppContext, AppResult,
    modules::live::{
        self, BoardPageInput, DecideInput, GuestJoinInput, GuestLink, GuestQuery, GuestState, GuestTicket, GuestView,
        IdentityInput, LiveJoin, LiveRecords, LiveStatus, MediaInput, PollInput, PollView,
    },
    security::Actor,
};
use uuid::Uuid;

use crate::extract::{Client, JsonBody, PathParam, QueryParams};

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/live/status", get(status))
        .route("/sessions/{id}/live/join", post(join))
        .route("/sessions/{id}/live/media", post(set_media))
        .route("/sessions/{id}/live/remove", post(remove))
        .route("/sessions/{id}/live/end", post(end))
        .route("/sessions/{id}/live/board-turns/start", post(turn_start))
        .route("/sessions/{id}/live/board-turns/end", post(turn_end))
        .route("/sessions/{id}/live/guest-link", post(guest_link))
        .route("/sessions/{id}/live/guests", get(guests))
        .route("/sessions/{id}/live/guests/decide-all", post(decide_all))
        .route("/sessions/{id}/live/guests/{guest_id}/decide", post(decide))
        .route("/sessions/{id}/live/polls", post(save_poll))
        .route("/sessions/{id}/live/board", get(board_data).put(save_board))
        .route("/sessions/{id}/live/records", get(records))
        // Khách (không cần đăng nhập)
        .route("/live/guest/{id}/join", post(guest_request))
        .route("/live/guest/state/{guest_id}", get(guest_state))
}

async fn status(State(ctx): State<AppContext>) -> Json<LiveStatus> {
    Json(live::status(&ctx))
}

async fn join(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<LiveJoin>> {
    Ok(Json(live::join(&ctx, &actor, id).await?))
}

async fn set_media(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<MediaInput>,
) -> AppResult<StatusCode> {
    live::set_media(&ctx, &actor, id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<IdentityInput>,
) -> AppResult<StatusCode> {
    live::remove(&ctx, &actor, id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn end(State(ctx): State<AppContext>, actor: Actor, PathParam(id): PathParam<Uuid>) -> AppResult<StatusCode> {
    live::end(&ctx, &actor, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn turn_start(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<IdentityInput>,
) -> AppResult<StatusCode> {
    live::board_turn(&ctx, &actor, id, input, true).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn turn_end(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<IdentityInput>,
) -> AppResult<StatusCode> {
    live::board_turn(&ctx, &actor, id, input, false).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn guest_link(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<GuestLink>> {
    Ok(Json(live::guest_link(&ctx, &actor, id).await?))
}

async fn guests(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<Vec<GuestView>>> {
    Ok(Json(live::guests(&ctx, &actor, id).await?))
}

async fn decide(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam((id, guest_id)): PathParam<(Uuid, Uuid)>,
    JsonBody(input): JsonBody<DecideInput>,
) -> AppResult<Json<Vec<GuestView>>> {
    Ok(Json(live::decide(&ctx, &actor, id, Some(guest_id), input).await?))
}

async fn decide_all(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<DecideInput>,
) -> AppResult<Json<Vec<GuestView>>> {
    Ok(Json(live::decide(&ctx, &actor, id, None, input).await?))
}

async fn save_poll(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<PollInput>,
) -> AppResult<(StatusCode, Json<PollView>)> {
    Ok((StatusCode::CREATED, Json(live::save_poll(&ctx, &actor, id, input).await?)))
}

async fn save_board(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<BoardPageInput>,
) -> AppResult<StatusCode> {
    live::save_board(&ctx, &actor, id, input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn board_data(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<Vec<Value>>> {
    Ok(Json(live::board_data(&ctx, &actor, id).await?))
}

async fn records(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<LiveRecords>> {
    Ok(Json(live::records(&ctx, &actor, id).await?))
}

async fn guest_request(
    State(ctx): State<AppContext>,
    Client(client): Client,
    PathParam(id): PathParam<Uuid>,
    JsonBody(input): JsonBody<GuestJoinInput>,
) -> AppResult<(StatusCode, Json<GuestTicket>)> {
    Ok((StatusCode::CREATED, Json(live::guest_request(&ctx, id, input, client.ip.as_deref()).await?)))
}

async fn guest_state(
    State(ctx): State<AppContext>,
    PathParam(guest_id): PathParam<Uuid>,
    QueryParams(q): QueryParams<GuestQuery>,
) -> AppResult<Json<GuestState>> {
    Ok(Json(live::guest_state(&ctx, guest_id, &q.token).await?))
}
