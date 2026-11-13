//! Dạy trực tiếp trong ứng dụng (đợt 6, G5): phòng LiveKit cho buổi học có `room = live`.
//!
//! - Video / âm thanh / chia sẻ màn hình đi thẳng giữa trình duyệt và LiveKit, không qua api.
//! - api cấp token vào phòng theo vai trò: giáo viên chủ buổi phát mọi nguồn + quản trị phòng; học sinh chỉ xem,
//!   nghe, chat / giơ tay (kênh dữ liệu) — giáo viên mời bật mic / camera thì api đổi quyền qua RoomService;
//!   khách chỉ xem và nghe.
//! - Vai trò nằm trong `metadata` của token (người tham gia không tự sửa được) nên mọi máy trong phòng tin được
//!   tin nhắn "của giáo viên" (cấp quyền vẽ bảng, mở bình chọn…) theo danh tính người gửi do LiveKit xác thực.
//! - Khách (buổi `public_link`): link có khóa ngẫu nhiên → nhập tên → phòng chờ → giáo viên duyệt mới có token.
//! - Lưu lại sau buổi: kết quả bình chọn / quiz, ảnh từng trang bảng, lượt lên bảng của học sinh.

pub mod livekit;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
use validator::Validate;

use self::livekit::VideoGrant;
use crate::{
    config::LiveConfig,
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        files::{self, Purpose},
        sessions::{model::SessionRow, service as sessions},
    },
    security::{
        Actor,
        token::{random_token, sha256_hex},
    },
};

/// Token vào phòng sống 6 giờ (đủ một buổi dài); hết hạn thì vào lại.
const TOKEN_TTL_SECS: i64 = 6 * 3600;
const MAX_POLL_OPTIONS: usize = 8;
const MAX_BOARD_PAGES: i32 = 50;
/// Mỗi IP xin vào phòng chờ tối đa bấy nhiêu lần / 10 phút.
const GUEST_REQUESTS_PER_IP: u32 = 10;

// ---------- DTO ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStatus {
    /// Đã cấu hình LiveKit (tạo buổi học có phòng của hệ thống được).
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionInfo {
    pub id: Uuid,
    pub title: String,
    pub class_title: Option<String>,
    pub teacher_name: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub access: String,
}

/// Thông tin vào phòng: trình duyệt kết nối `url` bằng `token`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveJoin {
    pub url: String,
    pub token: String,
    pub room: String,
    pub identity: String,
    pub name: String,
    /// teacher | student | guest
    pub role: &'static str,
    pub session: LiveSessionInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInput {
    /// Danh tính trong phòng (`u:{id}` / `g:{id}`).
    pub identity: String,
    #[serde(default)]
    pub microphone: bool,
    #[serde(default)]
    pub camera: bool,
    #[serde(default)]
    pub screen: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityInput {
    pub identity: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestLink {
    /// Link gửi cho khách (chỉ hiện một lần; đổi link = link cũ hết hiệu lực).
    pub url: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GuestJoinInput {
    pub key: String,
    #[validate(length(min = 2, max = 60, message = "Tên từ 2 đến 60 ký tự."))]
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestTicket {
    pub guest_id: Uuid,
    /// Khóa riêng của khách để hỏi trạng thái phòng chờ.
    pub token: String,
    pub session: LiveSessionInfo,
}

#[derive(Debug, Deserialize)]
pub struct GuestQuery {
    pub token: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestState {
    /// waiting | admitted | denied | removed
    pub status: String,
    /// Đã được duyệt: thông tin vào phòng.
    pub live: Option<LiveJoin>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct GuestView {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecideInput {
    pub admit: bool,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PollInput {
    /// poll | quiz
    pub kind: String,
    #[validate(length(min = 1, max = 500, message = "Câu hỏi từ 1 đến 500 ký tự."))]
    pub question: String,
    pub options: Vec<String>,
    #[serde(default)]
    pub correct: Vec<i32>,
    pub counts: Vec<i32>,
    pub voters: i32,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PollView {
    pub id: Uuid,
    pub kind: String,
    pub question: String,
    #[sqlx(json)]
    pub options: Vec<String>,
    pub correct: Vec<i32>,
    pub counts: Vec<i32>,
    pub voters: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardPageInput {
    pub page: i32,
    /// Ảnh trang bảng (file `content` của giáo viên).
    pub file_id: Uuid,
    /// Dữ liệu nét vẽ (`hui-whiteboard`) để mở lại.
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardPageView {
    pub page: i32,
    pub image_url: String,
    pub saved_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveRecords {
    pub polls: Vec<PollView>,
    pub board: Vec<BoardPageView>,
    /// Giáo viên: số lần lên bảng của từng học sinh.
    pub board_turns: Vec<BoardTurnCount>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BoardTurnCount {
    pub user_id: Uuid,
    pub full_name: String,
    pub turns: i64,
}

// ---------- Tiện ích ----------

fn config(ctx: &AppContext) -> AppResult<&LiveConfig> {
    ctx.config.live.as_ref().ok_or_else(|| {
        AppError::new(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "LIVE_DISABLED",
            "Chưa cấu hình phòng học trực tuyến (LiveKit). Hãy báo quản trị viên.",
        )
    })
}

pub fn room_name(session_id: Uuid) -> String {
    format!("session-{session_id}")
}

fn info(s: &SessionRow) -> LiveSessionInfo {
    LiveSessionInfo {
        id: s.id,
        title: s.title.clone(),
        class_title: s.class_title.clone(),
        teacher_name: s.owner_name.clone(),
        starts_at: s.starts_at,
        ends_at: s.ends_at,
        access: s.access.clone(),
    }
}

/// `u:{uuid}` → ID người dùng (danh tính trong phòng).
fn user_of(identity: &str) -> Option<Uuid> {
    identity.strip_prefix("u:").and_then(|id| Uuid::parse_str(id).ok())
}

fn guest_of(identity: &str) -> Option<Uuid> {
    identity.strip_prefix("g:").and_then(|id| Uuid::parse_str(id).ok())
}

fn bad_identity() -> AppError {
    AppError::field("identity", "Người tham gia không hợp lệ.")
}

fn join_info(
    cfg: &LiveConfig,
    s: &SessionRow,
    identity: String,
    name: String,
    role: &'static str,
    grant: VideoGrant,
) -> LiveJoin {
    let metadata = json!({ "role": role }).to_string();
    LiveJoin {
        url: cfg.public_url.clone(),
        token: livekit::token(cfg, &identity, &name, &metadata, &grant, TOKEN_TTL_SECS),
        room: room_name(s.id),
        identity,
        name,
        role,
        session: info(s),
    }
}

// ---------- Vào phòng ----------

pub fn status(ctx: &AppContext) -> LiveStatus {
    LiveStatus { enabled: ctx.config.live.is_some() }
}

/// Giáo viên chủ buổi / học sinh được vào: token theo vai trò, ghi điểm danh học sinh.
pub async fn join(ctx: &AppContext, actor: &Actor, session_id: Uuid) -> AppResult<LiveJoin> {
    let cfg = config(ctx)?;
    let (s, is_owner) = sessions::enter_live(ctx, actor, session_id).await?;
    let name: String =
        sqlx::query_scalar("SELECT full_name FROM users WHERE id = $1").bind(actor.id).fetch_one(&ctx.db).await?;
    let room = room_name(s.id);
    let grant = if is_owner {
        sqlx::query("UPDATE sessions SET live_started_at = coalesce(live_started_at, now()) WHERE id = $1")
            .bind(s.id)
            .execute(&ctx.db)
            .await?;
        VideoGrant {
            room: Some(room),
            room_join: true,
            room_admin: true,
            can_publish: true,
            can_subscribe: true,
            can_publish_data: true,
            ..Default::default()
        }
    } else {
        // Học sinh: mặc định tắt mic / camera; giáo viên mời thì api đổi quyền.
        VideoGrant {
            room: Some(room),
            room_join: true,
            can_subscribe: true,
            can_publish_data: true,
            ..Default::default()
        }
    };
    Ok(join_info(cfg, &s, format!("u:{}", actor.id), name, if is_owner { "teacher" } else { "student" }, grant))
}

// ---------- Giáo viên điều khiển phòng ----------

/// Cho / thu quyền bật mic, camera, chia sẻ màn hình của một học sinh (ngay lập tức, không cần vào lại).
pub async fn set_media(ctx: &AppContext, actor: &Actor, session_id: Uuid, input: MediaInput) -> AppResult<()> {
    let cfg = config(ctx)?;
    let s = sessions::own_live(ctx, actor, session_id).await?;
    if user_of(&input.identity).is_none() || input.identity == format!("u:{}", actor.id) {
        return Err(bad_identity());
    }
    let mut sources = Vec::new();
    if input.microphone {
        sources.push("MICROPHONE");
    }
    if input.camera {
        sources.push("CAMERA");
    }
    if input.screen {
        sources.extend(["SCREEN_SHARE", "SCREEN_SHARE_AUDIO"]);
    }
    livekit::set_publish(cfg, &room_name(s.id), &input.identity, &sources, true).await
}

/// Mời ra khỏi phòng (khách thì không vào lại được nữa).
pub async fn remove(ctx: &AppContext, actor: &Actor, session_id: Uuid, input: IdentityInput) -> AppResult<()> {
    let cfg = config(ctx)?;
    let s = sessions::own_live(ctx, actor, session_id).await?;
    if let Some(guest) = guest_of(&input.identity) {
        sqlx::query(
            "UPDATE live_guests SET status = 'removed', decided_at = now(), decided_by = $3 \
             WHERE id = $1 AND session_id = $2",
        )
        .bind(guest)
        .bind(s.id)
        .bind(actor.id)
        .execute(&ctx.db)
        .await?;
    } else if user_of(&input.identity).is_none() {
        return Err(bad_identity());
    }
    livekit::remove(cfg, &room_name(s.id), &input.identity).await
}

/// Kết thúc buổi: đóng phòng, mọi người bị ngắt kết nối.
pub async fn end(ctx: &AppContext, actor: &Actor, session_id: Uuid) -> AppResult<()> {
    let cfg = config(ctx)?;
    let s = sessions::own_live(ctx, actor, session_id).await?;
    sqlx::query("UPDATE sessions SET live_ended_at = now() WHERE id = $1").bind(s.id).execute(&ctx.db).await?;
    sqlx::query("UPDATE live_board_turns SET ended_at = now() WHERE session_id = $1 AND ended_at IS NULL")
        .bind(s.id)
        .execute(&ctx.db)
        .await?;
    livekit::close_room(cfg, &room_name(s.id)).await
}

/// Ghi lượt lên bảng (`start = true` khi mời lên, `false` khi mời về chỗ).
pub async fn board_turn(
    ctx: &AppContext,
    actor: &Actor,
    session_id: Uuid,
    input: IdentityInput,
    start: bool,
) -> AppResult<()> {
    let s = sessions::own_live(ctx, actor, session_id).await?;
    let user = user_of(&input.identity).ok_or_else(bad_identity)?;
    if start {
        sqlx::query("INSERT INTO live_board_turns (session_id, user_id) VALUES ($1, $2)")
            .bind(s.id)
            .bind(user)
            .execute(&ctx.db)
            .await?;
    } else {
        sqlx::query(
            "UPDATE live_board_turns SET ended_at = now() WHERE session_id = $1 AND user_id = $2 AND ended_at IS NULL",
        )
        .bind(s.id)
        .bind(user)
        .execute(&ctx.db)
        .await?;
    }
    Ok(())
}

// ---------- Khách (buổi mở bằng link) ----------

/// Tạo (hoặc đổi) link cho khách; link cũ hết hiệu lực.
pub async fn guest_link(ctx: &AppContext, actor: &Actor, session_id: Uuid) -> AppResult<GuestLink> {
    let s = sessions::own_live(ctx, actor, session_id).await?;
    if s.access != "public_link" {
        return Err(AppError::conflict("NOT_PUBLIC", "Hãy đặt \"Ai được vào\" là \"Khách có link\" trước."));
    }
    let key = random_token();
    sqlx::query("UPDATE sessions SET guest_key_hash = $2 WHERE id = $1")
        .bind(s.id)
        .bind(sha256_hex(&key))
        .execute(&ctx.db)
        .await?;
    Ok(GuestLink { url: format!("{}/vao-hoc/{}?k={key}", ctx.config.app_base_url, s.id) })
}

/// Khách nhập tên → vào phòng chờ.
pub async fn guest_request(
    ctx: &AppContext,
    session_id: Uuid,
    input: GuestJoinInput,
    ip: Option<&str>,
) -> AppResult<GuestTicket> {
    input.validate()?;
    config(ctx)?;
    ctx.limiter
        .hit_and_check(
            &format!("live-guest:{}", ip.unwrap_or("?")),
            GUEST_REQUESTS_PER_IP,
            std::time::Duration::from_secs(600),
        )
        .await?;
    let s = sessions::find_row(ctx, session_id).await?;
    let key_hash: Option<String> =
        sqlx::query_scalar("SELECT guest_key_hash FROM sessions WHERE id = $1").bind(s.id).fetch_one(&ctx.db).await?;
    if s.access != "public_link" || key_hash.as_deref() != Some(sha256_hex(input.key.trim()).as_str()) {
        return Err(AppError::not_found("Link vào học không đúng hoặc đã được giáo viên đổi."));
    }
    sessions::live_window(&s, false)?;
    let (count, limit): (i64, i32) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM live_guests WHERE session_id = $1 AND status IN ('waiting', 'admitted')), \
         (SELECT guest_limit FROM sessions WHERE id = $1)",
    )
    .bind(s.id)
    .fetch_one(&ctx.db)
    .await?;
    if count >= limit as i64 {
        return Err(AppError::conflict("ROOM_FULL", "Phòng đã đủ số khách. Hãy liên hệ giáo viên."));
    }
    let token = random_token();
    let guest_id: Uuid = sqlx::query_scalar(
        "INSERT INTO live_guests (session_id, name, token_hash, ip) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(s.id)
    .bind(input.name.trim())
    .bind(sha256_hex(&token))
    .bind(ip)
    .fetch_one(&ctx.db)
    .await?;
    Ok(GuestTicket { guest_id, token, session: info(&s) })
}

/// Khách hỏi trạng thái phòng chờ; được duyệt thì nhận token (chỉ xem và nghe).
pub async fn guest_state(ctx: &AppContext, guest_id: Uuid, token: &str) -> AppResult<GuestState> {
    let cfg = config(ctx)?;
    let row: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT session_id, name, status FROM live_guests WHERE id = $1 AND token_hash = $2")
            .bind(guest_id)
            .bind(sha256_hex(token))
            .fetch_optional(&ctx.db)
            .await?;
    let (session_id, name, status) = row.ok_or_else(|| AppError::not_found("Không tìm thấy lượt vào phòng."))?;
    if status != "admitted" {
        return Ok(GuestState { status, live: None });
    }
    let s = sessions::find_row(ctx, session_id).await?;
    sessions::live_window(&s, false)?;
    let grant = VideoGrant { room: Some(room_name(s.id)), room_join: true, can_subscribe: true, ..Default::default() };
    let live = join_info(cfg, &s, format!("g:{guest_id}"), format!("{name} (Khách)"), "guest", grant);
    Ok(GuestState { status, live: Some(live) })
}

pub async fn guests(ctx: &AppContext, actor: &Actor, session_id: Uuid) -> AppResult<Vec<GuestView>> {
    let s = sessions::own_live(ctx, actor, session_id).await?;
    Ok(sqlx::query_as::<_, GuestView>(
        "SELECT id, name, status, created_at FROM live_guests WHERE session_id = $1 AND status IN ('waiting', 'admitted') \
         ORDER BY created_at",
    )
    .bind(s.id)
    .fetch_all(&ctx.db)
    .await?)
}

/// Duyệt / từ chối một khách; `guest_id = None` → duyệt mọi khách đang chờ.
pub async fn decide(
    ctx: &AppContext,
    actor: &Actor,
    session_id: Uuid,
    guest_id: Option<Uuid>,
    input: DecideInput,
) -> AppResult<Vec<GuestView>> {
    let s = sessions::own_live(ctx, actor, session_id).await?;
    sqlx::query(
        "UPDATE live_guests SET status = $3, decided_at = now(), decided_by = $4 \
         WHERE session_id = $1 AND status = 'waiting' AND ($2::uuid IS NULL OR id = $2)",
    )
    .bind(s.id)
    .bind(guest_id)
    .bind(if input.admit { "admitted" } else { "denied" })
    .bind(actor.id)
    .execute(&ctx.db)
    .await?;
    guests(ctx, actor, session_id).await
}

// ---------- Lưu lại sau buổi ----------

pub async fn save_poll(ctx: &AppContext, actor: &Actor, session_id: Uuid, input: PollInput) -> AppResult<PollView> {
    input.validate()?;
    let s = sessions::own_live(ctx, actor, session_id).await?;
    let n = input.options.len();
    if !(2..=MAX_POLL_OPTIONS).contains(&n) || input.counts.len() != n {
        return Err(AppError::field("options", format!("Có từ 2 đến {MAX_POLL_OPTIONS} phương án.")));
    }
    if !matches!(input.kind.as_str(), "poll" | "quiz")
        || input.correct.iter().any(|c| *c < 0 || *c as usize >= n)
        || (input.kind == "quiz" && input.correct.is_empty())
    {
        return Err(AppError::field("correct", "Quiz cần chọn đáp án đúng."));
    }
    if input.counts.iter().any(|c| *c < 0) || input.voters < 0 {
        return Err(AppError::field("counts", "Số phiếu không hợp lệ."));
    }
    let options: Vec<String> = input.options.iter().map(|o| o.trim().chars().take(200).collect()).collect();
    Ok(sqlx::query_as::<_, PollView>(
        "INSERT INTO live_polls (session_id, kind, question, options, correct, counts, voters) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING id, kind, question, options, correct, counts, voters, created_at",
    )
    .bind(s.id)
    .bind(&input.kind)
    .bind(input.question.trim())
    .bind(sqlx::types::Json(&options))
    .bind(&input.correct)
    .bind(&input.counts)
    .bind(input.voters)
    .fetch_one(&ctx.db)
    .await?)
}

/// Lưu một trang bảng (ảnh + dữ liệu nét vẽ). Lưu lại cùng trang thì ghi đè.
pub async fn save_board(ctx: &AppContext, actor: &Actor, session_id: Uuid, input: BoardPageInput) -> AppResult<()> {
    let s = sessions::own_live(ctx, actor, session_id).await?;
    if !(0..MAX_BOARD_PAGES).contains(&input.page) {
        return Err(AppError::field("page", format!("Bảng tối đa {MAX_BOARD_PAGES} trang.")));
    }
    files::owned_ready(ctx, actor.id, &[input.file_id], Purpose::Content).await?;
    sqlx::query(
        "INSERT INTO live_board_pages (session_id, page, file_id, data) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (session_id, page) DO UPDATE SET file_id = EXCLUDED.file_id, data = EXCLUDED.data, saved_at = now()",
    )
    .bind(s.id)
    .bind(input.page)
    .bind(input.file_id)
    .bind(if input.data.is_null() { json!([]) } else { input.data })
    .execute(&ctx.db)
    .await?;
    Ok(())
}

/// Giáo viên mở lại bảng đã lưu (dữ liệu nét vẽ từng trang).
pub async fn board_data(ctx: &AppContext, actor: &Actor, session_id: Uuid) -> AppResult<Vec<Value>> {
    let s = sessions::own_live(ctx, actor, session_id).await?;
    Ok(sqlx::query_scalar("SELECT data FROM live_board_pages WHERE session_id = $1 ORDER BY page")
        .bind(s.id)
        .fetch_all(&ctx.db)
        .await?)
}

/// Bình chọn + ảnh bảng của buổi (ai vào được buổi thì xem được); giáo viên xem thêm lượt lên bảng.
pub async fn records(ctx: &AppContext, actor: &Actor, session_id: Uuid) -> AppResult<LiveRecords> {
    let s = sessions::viewable(ctx, actor, session_id).await?;
    let polls = sqlx::query_as::<_, PollView>(
        "SELECT id, kind, question, options, correct, counts, voters, created_at FROM live_polls \
         WHERE session_id = $1 ORDER BY created_at",
    )
    .bind(s.id)
    .fetch_all(&ctx.db)
    .await?;
    let pages: Vec<(i32, Uuid, DateTime<Utc>)> =
        sqlx::query_as("SELECT page, file_id, saved_at FROM live_board_pages WHERE session_id = $1 ORDER BY page")
            .bind(s.id)
            .fetch_all(&ctx.db)
            .await?;
    let board = pages
        .into_iter()
        .map(|(page, file_id, saved_at)| BoardPageView {
            page,
            image_url: files::file_url(ctx, file_id, Purpose::Content.as_str()),
            saved_at,
        })
        .collect();
    let board_turns = if s.owner_id == actor.id {
        sqlx::query_as::<_, BoardTurnCount>(
            "SELECT t.user_id, u.full_name, count(*) AS turns FROM live_board_turns t JOIN users u ON u.id = t.user_id \
             WHERE t.session_id = $1 GROUP BY t.user_id, u.full_name ORDER BY turns DESC, u.full_name",
        )
        .bind(s.id)
        .fetch_all(&ctx.db)
        .await?
    } else {
        Vec::new()
    };
    Ok(LiveRecords { polls, board, board_turns })
}
