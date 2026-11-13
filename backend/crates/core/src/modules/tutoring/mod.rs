//! Đặt lịch học kèm 1-1 (M23, D15): giáo viên mở khung giờ rảnh hằng tuần, học sinh chọn một khoảng còn trống
//! để đặt, giáo viên đồng ý → tạo buổi học (module `sessions`, có lời mời + `.ics`) hoặc từ chối.
//!
//! - Khung giờ tính theo giờ Việt Nam, chia thành các khoảng bằng thời lượng một buổi.
//! - Khoảng còn trống = trong khung giờ, cách bây giờ ít nhất `min_notice_hours`, trong `horizon_days` ngày tới,
//!   không trùng buổi học khác của giáo viên và không trùng yêu cầu đang chờ / đã duyệt.
//! - Hai học sinh đặt cùng khoảng một lúc: chỉ mục duy nhất `(teacher_id, starts_at)` cho một người thắng.

use std::collections::HashSet;

use chrono::{DateTime, Datelike, NaiveTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        groups,
        notifications::{self, NotificationKind},
        sessions::{service as sessions, time},
        teachers::service as teachers,
    },
    security::{Actor, Permission},
};

/// Học sinh có tối đa bấy nhiêu yêu cầu đang chờ với một giáo viên.
pub const MAX_PENDING_PER_TEACHER: i64 = 3;

// ---------- Khung giờ (hàm thuần) ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    /// 1 = thứ Hai … 7 = Chủ nhật.
    #[validate(range(min = 1, max = 7, message = "Thứ từ 1 (thứ Hai) đến 7 (Chủ nhật)."))]
    pub weekday: u32,
    /// "HH:MM" giờ Việt Nam.
    #[validate(custom(function = "validate_hhmm"))]
    pub start: String,
    #[validate(custom(function = "validate_hhmm"))]
    pub end: String,
}

fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}

fn validate_hhmm(s: &str) -> Result<(), ValidationError> {
    parse_hhmm(s).map(|_| ()).ok_or_else(|| ValidationError::new("hhmm").with_message("Giờ dạng HH:MM.".into()))
}

/// Các khoảng có thể đặt (chưa trừ giờ bận), sắp theo giờ.
pub fn candidate_slots(
    windows: &[Window],
    duration_minutes: i64,
    now: DateTime<Utc>,
    min_notice_hours: i64,
    horizon_days: i64,
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let earliest = now + TimeDelta::hours(min_notice_hours);
    let last = now + TimeDelta::days(horizon_days);
    let today = time::to_local(now).date();
    let step = TimeDelta::minutes(duration_minutes);
    let mut out = Vec::new();
    for d in 0..=horizon_days {
        let date = today + TimeDelta::days(d);
        let weekday = date.weekday().number_from_monday();
        for w in windows.iter().filter(|w| w.weekday == weekday) {
            let (Some(start), Some(end)) = (parse_hhmm(&w.start), parse_hhmm(&w.end)) else { continue };
            let mut t = date.and_time(start);
            let end = date.and_time(end);
            while t + step <= end {
                let s = time::from_local(t);
                if s >= earliest && s <= last {
                    out.push((s, s + step));
                }
                t += step;
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Bỏ các khoảng trùng giờ bận.
pub fn free_slots(
    candidates: Vec<(DateTime<Utc>, DateTime<Utc>)>,
    busy: &[(DateTime<Utc>, DateTime<Utc>)],
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    candidates.into_iter().filter(|(s, e)| !busy.iter().any(|(bs, be)| s < be && e > bs)).collect()
}

// ---------- DTO ----------

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OfferInput {
    pub subject_id: Option<Uuid>,
    #[validate(length(min = 1, max = 200, message = "Tên từ 1 đến 200 ký tự."))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 2000, message = "Mô tả tối đa 2.000 ký tự."))]
    pub description: String,
    #[validate(range(min = 15, max = 240, message = "Thời lượng một buổi từ 15 đến 240 phút."))]
    pub duration_minutes: i64,
    #[validate(length(min = 1, max = 21, message = "Có từ 1 đến 21 khung giờ."), nested)]
    pub windows: Vec<Window>,
    #[serde(default)]
    #[validate(length(max = 500, message = "Link tối đa 500 ký tự."))]
    pub meeting_url: String,
    /// my_students (học sinh các lớp học của tôi) | everyone (mọi tài khoản)
    pub audience: String,
    #[validate(range(min = 0, max = 168, message = "Đặt trước từ 0 đến 168 giờ."))]
    pub min_notice_hours: i64,
    #[validate(range(min = 1, max = 60, message = "Mở đặt trước từ 1 đến 60 ngày."))]
    pub horizon_days: i64,
    #[serde(default = "yes")]
    pub active: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OfferView {
    pub id: Uuid,
    pub teacher_id: Uuid,
    pub teacher_name: String,
    pub teacher_username: String,
    pub subject_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub duration_minutes: i32,
    #[sqlx(json)]
    pub windows: Vec<Window>,
    /// Chỉ giáo viên thấy (học sinh nhận link trong buổi học sau khi được duyệt).
    pub meeting_url: Option<String>,
    pub audience: String,
    pub min_notice_hours: i32,
    pub horizon_days: i32,
    pub active: bool,
    /// Số yêu cầu đang chờ (giáo viên).
    pub pending_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BookInput {
    pub starts_at: DateTime<Utc>,
    #[serde(default)]
    #[validate(length(max = 1000, message = "Lời nhắn tối đa 1.000 ký tự."))]
    pub message: String,
}

#[derive(Debug, Deserialize, Validate, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReasonInput {
    #[serde(default)]
    #[validate(length(max = 1000, message = "Lý do tối đa 1.000 ký tự."))]
    pub reason: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BookingQuery {
    /// pending | upcoming | all (mặc định upcoming: đang chờ + đã duyệt chưa diễn ra)
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BookingView {
    pub id: Uuid,
    pub offer_id: Uuid,
    pub offer_title: String,
    pub teacher_id: Uuid,
    pub teacher_name: String,
    pub student_id: Uuid,
    pub student_name: String,
    pub student_username: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    /// pending | approved | declined | cancelled
    pub status: String,
    pub message: String,
    pub reply: String,
    pub session_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct OfferRow {
    id: Uuid,
    teacher_id: Uuid,
    title: String,
    duration_minutes: i32,
    windows: Json<Vec<Window>>,
    meeting_url: String,
    audience: String,
    min_notice_hours: i32,
    horizon_days: i32,
    active: bool,
}

fn offer_not_found() -> AppError {
    AppError::not_found("Không tìm thấy lịch học kèm.")
}

fn booking_not_found() -> AppError {
    AppError::not_found("Không tìm thấy yêu cầu đặt lịch.")
}

const OFFER_VIEW: &str = "SELECT o.id, o.teacher_id, u.full_name AS teacher_name, u.username AS teacher_username, \
     o.subject_id, o.title, o.description, o.duration_minutes, o.windows, \
     CASE WHEN o.teacher_id = $1 THEN o.meeting_url END AS meeting_url, o.audience, o.min_notice_hours, \
     o.horizon_days, o.active, \
     CASE WHEN o.teacher_id = $1 THEN (SELECT count(*) FROM tutoring_bookings b \
       WHERE b.offer_id = o.id AND b.status = 'pending' AND b.starts_at > now()) ELSE 0 END AS pending_count \
     FROM tutoring_offers o JOIN users u ON u.id = o.teacher_id";

const BOOKING_VIEW: &str = "SELECT b.id, b.offer_id, o.title AS offer_title, b.teacher_id, t.full_name AS teacher_name, \
     b.student_id, s.full_name AS student_name, s.username AS student_username, b.starts_at, b.ends_at, b.status, \
     b.message, b.reply, b.session_id, b.created_at \
     FROM tutoring_bookings b JOIN tutoring_offers o ON o.id = b.offer_id \
     JOIN users t ON t.id = b.teacher_id JOIN users s ON s.id = b.student_id";

fn check_input(input: &OfferInput) -> AppResult<()> {
    input.validate()?;
    if !matches!(input.audience.as_str(), "my_students" | "everyone") {
        return Err(AppError::field("audience", "Chọn ai được đặt lịch."));
    }
    for w in &input.windows {
        let (s, e) = (parse_hhmm(&w.start), parse_hhmm(&w.end));
        if s >= e {
            return Err(AppError::field("windows", "Giờ kết thúc phải sau giờ bắt đầu."));
        }
        if let (Some(s), Some(e)) = (s, e)
            && (e - s).num_minutes() < input.duration_minutes
        {
            return Err(AppError::field("windows", "Mỗi khung giờ phải dài ít nhất bằng một buổi học."));
        }
    }
    let url = input.meeting_url.trim();
    if !url.is_empty() && !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(AppError::field("meetingUrl", "Link họp phải bắt đầu bằng http:// hoặc https://."));
    }
    Ok(())
}

async fn find_offer(ctx: &AppContext, id: Uuid) -> AppResult<OfferRow> {
    sqlx::query_as::<_, OfferRow>(
        "SELECT id, teacher_id, title, duration_minutes, windows, meeting_url, audience, min_notice_hours, \
         horizon_days, active FROM tutoring_offers WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&ctx.db)
    .await?
    .ok_or_else(offer_not_found)
}

async fn offer_view(ctx: &AppContext, viewer: Uuid, id: Uuid) -> AppResult<OfferView> {
    sqlx::query_as::<_, OfferView>(crate::sql!("{OFFER_VIEW} WHERE o.id = $2"))
        .bind(viewer)
        .bind(id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or_else(offer_not_found)
}

/// Học sinh đang học một lớp học của giáo viên.
async fn is_student_of(ctx: &AppContext, teacher: Uuid, user: Uuid) -> AppResult<bool> {
    Ok(groups::ensure_own_students(ctx, teacher, &[user]).await.is_ok())
}

async fn can_book(ctx: &AppContext, actor: &Actor, offer: &OfferRow) -> AppResult<bool> {
    Ok(offer.active
        && offer.teacher_id != actor.id
        && (offer.audience == "everyone" || is_student_of(ctx, offer.teacher_id, actor.id).await?))
}

// ---------- Giáo viên ----------

pub async fn my_offers(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<OfferView>> {
    actor.require(Permission::TeachingManage)?;
    Ok(sqlx::query_as::<_, OfferView>(crate::sql!(
        "{OFFER_VIEW} WHERE o.teacher_id = $1 ORDER BY o.active DESC, o.created_at"
    ))
    .bind(actor.id)
    .fetch_all(&ctx.db)
    .await?)
}

pub async fn create_offer(ctx: &AppContext, actor: &Actor, input: OfferInput) -> AppResult<OfferView> {
    actor.require(Permission::TeachingManage)?;
    check_input(&input)?;
    if let Some(subject) = input.subject_id {
        teachers::ensure_can_teach(ctx, actor, subject).await?;
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO tutoring_offers (teacher_id, subject_id, title, description, duration_minutes, windows, \
         meeting_url, audience, min_notice_hours, horizon_days, active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(actor.id)
    .bind(input.subject_id)
    .bind(input.title.trim())
    .bind(input.description.trim())
    .bind(input.duration_minutes as i32)
    .bind(Json(&input.windows))
    .bind(input.meeting_url.trim())
    .bind(&input.audience)
    .bind(input.min_notice_hours as i32)
    .bind(input.horizon_days as i32)
    .bind(input.active)
    .fetch_one(&ctx.db)
    .await?;
    offer_view(ctx, actor.id, id).await
}

/// Sửa / tạm dừng khung giờ. Yêu cầu đã gửi giữ nguyên.
pub async fn update_offer(ctx: &AppContext, actor: &Actor, id: Uuid, input: OfferInput) -> AppResult<OfferView> {
    actor.require(Permission::TeachingManage)?;
    check_input(&input)?;
    let offer = find_offer(ctx, id).await?;
    if offer.teacher_id != actor.id {
        return Err(offer_not_found());
    }
    if let Some(subject) = input.subject_id {
        teachers::ensure_can_teach(ctx, actor, subject).await?;
    }
    sqlx::query(
        "UPDATE tutoring_offers SET subject_id = $2, title = $3, description = $4, duration_minutes = $5, \
         windows = $6, meeting_url = $7, audience = $8, min_notice_hours = $9, horizon_days = $10, active = $11, \
         updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(input.subject_id)
    .bind(input.title.trim())
    .bind(input.description.trim())
    .bind(input.duration_minutes as i32)
    .bind(Json(&input.windows))
    .bind(input.meeting_url.trim())
    .bind(&input.audience)
    .bind(input.min_notice_hours as i32)
    .bind(input.horizon_days as i32)
    .bind(input.active)
    .execute(&ctx.db)
    .await?;
    offer_view(ctx, actor.id, id).await
}

fn status_filter(q: &BookingQuery) -> &'static str {
    match q.status.as_deref() {
        Some("pending") => "b.status = 'pending' AND b.starts_at > now()",
        Some("all") => "TRUE",
        _ => "b.status IN ('pending', 'approved') AND b.ends_at > now()",
    }
}

pub async fn teacher_bookings(ctx: &AppContext, actor: &Actor, q: BookingQuery) -> AppResult<Vec<BookingView>> {
    actor.require(Permission::TeachingManage)?;
    let order = if q.status.as_deref() == Some("all") { "b.starts_at DESC" } else { "b.starts_at" };
    Ok(sqlx::query_as::<_, BookingView>(crate::sql!(
        "{BOOKING_VIEW} WHERE b.teacher_id = $1 AND {} ORDER BY {order} LIMIT 200",
        status_filter(&q)
    ))
    .bind(actor.id)
    .fetch_all(&ctx.db)
    .await?)
}

async fn booking(ctx: &AppContext, id: Uuid) -> AppResult<BookingView> {
    sqlx::query_as::<_, BookingView>(crate::sql!("{BOOKING_VIEW} WHERE b.id = $1"))
        .bind(id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or_else(booking_not_found)
}

async fn own_pending(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<BookingView> {
    actor.require(Permission::TeachingManage)?;
    let b = booking(ctx, id).await?;
    if b.teacher_id != actor.id {
        return Err(booking_not_found());
    }
    if b.status != "pending" {
        return Err(AppError::conflict("BOOKING_NOT_PENDING", "Yêu cầu này đã được xử lý."));
    }
    Ok(b)
}

fn when(b: &BookingView) -> String {
    time::format_vn(b.starts_at)
}

/// Đồng ý: tạo buổi học (mời học sinh, gửi `.ics`), yêu cầu chuyển sang `approved`.
pub async fn approve(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<BookingView> {
    let b = own_pending(ctx, actor, id).await?;
    if b.starts_at <= Utc::now() {
        return Err(AppError::conflict("BOOKING_PAST", "Đã qua giờ học của yêu cầu này."));
    }
    let offer = find_offer(ctx, b.offer_id).await?;
    let description = if b.message.trim().is_empty() {
        format!("Học kèm 1-1 với {}.", b.student_name)
    } else {
        format!("Học kèm 1-1 với {}.\n\nHọc sinh muốn học: {}", b.student_name, b.message.trim())
    };
    let url = Some(offer.meeting_url.trim().to_string()).filter(|u| !u.is_empty());
    let session_id = sessions::create_booked(
        ctx,
        sessions::BookedSession {
            teacher_id: actor.id,
            student_id: b.student_id,
            title: &format!("{} — {}", b.offer_title, b.student_name),
            description: &description,
            starts_at: b.starts_at,
            duration_minutes: (b.ends_at - b.starts_at).num_minutes(),
            meeting_url: url,
        },
    )
    .await?;
    sqlx::query("UPDATE tutoring_bookings SET status = 'approved', session_id = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(session_id)
        .execute(&ctx.db)
        .await?;
    notifications::notify(
        &ctx.db,
        &[b.student_id],
        NotificationKind::BookingApproved,
        &format!("{} đã đồng ý lịch học kèm", b.teacher_name),
        &format!("{} · {}", b.offer_title, when(&b)),
        Some(&format!("/buoi-hoc/{session_id}")),
    )
    .await?;
    booking(ctx, id).await
}

pub async fn decline(ctx: &AppContext, actor: &Actor, id: Uuid, input: ReasonInput) -> AppResult<BookingView> {
    input.validate()?;
    let b = own_pending(ctx, actor, id).await?;
    sqlx::query("UPDATE tutoring_bookings SET status = 'declined', reply = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(input.reason.trim())
        .execute(&ctx.db)
        .await?;
    notifications::notify(
        &ctx.db,
        &[b.student_id],
        NotificationKind::BookingDeclined,
        &format!("{} không nhận lịch học kèm {}", b.teacher_name, when(&b)),
        if input.reason.trim().is_empty() { "Hãy chọn khung giờ khác." } else { input.reason.trim() },
        Some("/hoc-kem"),
    )
    .await?;
    booking(ctx, id).await
}

/// Hủy lịch (giáo viên hoặc học sinh của yêu cầu): đang chờ / đã duyệt chưa diễn ra. Đã duyệt → hủy cả buổi học.
pub async fn cancel(ctx: &AppContext, actor: &Actor, id: Uuid, input: ReasonInput) -> AppResult<BookingView> {
    input.validate()?;
    let b = booking(ctx, id).await?;
    let by_teacher = b.teacher_id == actor.id;
    if !by_teacher && b.student_id != actor.id {
        return Err(booking_not_found());
    }
    if !matches!(b.status.as_str(), "pending" | "approved") || b.starts_at <= Utc::now() {
        return Err(AppError::conflict("BOOKING_NOT_CANCELLABLE", "Không hủy được lịch này nữa."));
    }
    sqlx::query(
        "UPDATE tutoring_bookings SET status = 'cancelled', reply = $2, cancelled_by = $3, updated_at = now() \
         WHERE id = $1",
    )
    .bind(id)
    .bind(input.reason.trim())
    .bind(actor.id)
    .execute(&ctx.db)
    .await?;
    if let Some(session) = b.session_id {
        sessions::cancel_by_id(ctx, session).await?;
    }
    let (to, who, link) = if by_teacher {
        (b.student_id, b.teacher_name.clone(), "/hoc-kem")
    } else {
        (b.teacher_id, b.student_name.clone(), "/day-hoc/hoc-kem")
    };
    notifications::notify(
        &ctx.db,
        &[to],
        NotificationKind::BookingCancelled,
        &format!("{who} đã hủy lịch học kèm {}", when(&b)),
        if input.reason.trim().is_empty() { &b.offer_title } else { input.reason.trim() },
        Some(link),
    )
    .await?;
    booking(ctx, id).await
}

// ---------- Học sinh ----------

/// Khung giờ học kèm học sinh này đặt được: của giáo viên các lớp học đang học + khung mở cho mọi tài khoản.
pub async fn visible_offers(ctx: &AppContext, actor: &Actor, teacher_id: Option<Uuid>) -> AppResult<Vec<OfferView>> {
    Ok(sqlx::query_as::<_, OfferView>(crate::sql!(
        "{OFFER_VIEW} WHERE o.active AND o.teacher_id <> $1 AND ($2::uuid IS NULL OR o.teacher_id = $2) \
         AND (o.audience = 'everyone' OR EXISTS (SELECT 1 FROM class_members m JOIN classes c ON c.id = m.class_id \
           WHERE c.owner_id = o.teacher_id AND m.user_id = $1 AND m.role = 'student' AND m.status = 'active')) \
         ORDER BY u.full_name, o.title LIMIT 200"
    ))
    .bind(actor.id)
    .bind(teacher_id)
    .fetch_all(&ctx.db)
    .await?)
}

/// Khung giờ mở cho mọi tài khoản của một giáo viên (trang hồ sơ công khai, không cần đăng nhập).
pub async fn public_offers(ctx: &AppContext, teacher_id: Uuid) -> AppResult<Vec<OfferView>> {
    Ok(sqlx::query_as::<_, OfferView>(crate::sql!(
        "{OFFER_VIEW} WHERE o.active AND o.audience = 'everyone' AND o.teacher_id = $2 ORDER BY o.title"
    ))
    .bind(Uuid::nil())
    .bind(teacher_id)
    .fetch_all(&ctx.db)
    .await?)
}

pub async fn get_offer(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<OfferView> {
    let offer = find_offer(ctx, id).await?;
    if offer.teacher_id != actor.id && !can_book(ctx, actor, &offer).await? {
        return Err(offer_not_found());
    }
    offer_view(ctx, actor.id, id).await
}

async fn free(
    ctx: &AppContext,
    offer: &OfferRow,
    now: DateTime<Utc>,
) -> AppResult<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
    let candidates = candidate_slots(
        &offer.windows.0,
        offer.duration_minutes as i64,
        now,
        offer.min_notice_hours as i64,
        offer.horizon_days as i64,
    );
    let (Some(first), Some(last)) = (candidates.first(), candidates.last()) else { return Ok(Vec::new()) };
    let mut busy = sessions::busy(ctx, offer.teacher_id, first.0, last.1).await?;
    busy.extend(
        sqlx::query_as::<_, (DateTime<Utc>, DateTime<Utc>)>(
            "SELECT starts_at, ends_at FROM tutoring_bookings WHERE teacher_id = $1 \
             AND status IN ('pending', 'approved') AND ends_at > $2 AND starts_at < $3",
        )
        .bind(offer.teacher_id)
        .bind(first.0)
        .bind(last.1)
        .fetch_all(&ctx.db)
        .await?,
    );
    Ok(free_slots(candidates, &busy))
}

pub async fn slots(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<Vec<Slot>> {
    let offer = find_offer(ctx, id).await?;
    if offer.teacher_id != actor.id && !can_book(ctx, actor, &offer).await? {
        return Err(offer_not_found());
    }
    Ok(free(ctx, &offer, Utc::now())
        .await?
        .into_iter()
        .map(|(starts_at, ends_at)| Slot { starts_at, ends_at })
        .collect())
}

pub async fn book(ctx: &AppContext, actor: &Actor, id: Uuid, input: BookInput) -> AppResult<BookingView> {
    input.validate()?;
    let offer = find_offer(ctx, id).await?;
    if !can_book(ctx, actor, &offer).await? {
        return Err(offer_not_found());
    }
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tutoring_bookings WHERE teacher_id = $1 AND student_id = $2 \
         AND status = 'pending' AND starts_at > now()",
    )
    .bind(offer.teacher_id)
    .bind(actor.id)
    .fetch_one(&ctx.db)
    .await?;
    if pending >= MAX_PENDING_PER_TEACHER {
        return Err(AppError::conflict(
            "TOO_MANY_PENDING",
            format!("Bạn đang có {pending} yêu cầu chờ giáo viên trả lời. Hãy chờ giáo viên trả lời trước."),
        ));
    }
    let free: HashSet<DateTime<Utc>> = free(ctx, &offer, Utc::now()).await?.into_iter().map(|(s, _)| s).collect();
    if !free.contains(&input.starts_at) {
        return Err(AppError::conflict("SLOT_TAKEN", "Khung giờ này không còn trống. Hãy chọn giờ khác."));
    }
    let ends_at = input.starts_at + TimeDelta::minutes(offer.duration_minutes as i64);
    let inserted: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO tutoring_bookings (offer_id, teacher_id, student_id, starts_at, ends_at, message) \
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(offer.id)
    .bind(offer.teacher_id)
    .bind(actor.id)
    .bind(input.starts_at)
    .bind(ends_at)
    .bind(input.message.trim())
    .fetch_optional(&ctx.db)
    .await?;
    let Some(booking_id) = inserted else {
        return Err(AppError::conflict("SLOT_TAKEN", "Khung giờ này vừa có người đặt. Hãy chọn giờ khác."));
    };
    let student: String =
        sqlx::query_scalar("SELECT full_name FROM users WHERE id = $1").bind(actor.id).fetch_one(&ctx.db).await?;
    notifications::notify(
        &ctx.db,
        &[offer.teacher_id],
        NotificationKind::BookingRequested,
        &format!("{student} muốn học kèm {}", time::format_vn(input.starts_at)),
        &offer.title,
        Some("/day-hoc/hoc-kem"),
    )
    .await?;
    booking(ctx, booking_id).await
}

pub async fn my_bookings(ctx: &AppContext, actor: &Actor, q: BookingQuery) -> AppResult<Vec<BookingView>> {
    let order = if q.status.as_deref() == Some("all") { "b.starts_at DESC" } else { "b.starts_at" };
    Ok(sqlx::query_as::<_, BookingView>(crate::sql!(
        "{BOOKING_VIEW} WHERE b.student_id = $1 AND {} ORDER BY {order} LIMIT 200",
        status_filter(&q)
    ))
    .bind(actor.id)
    .fetch_all(&ctx.db)
    .await?)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn w(weekday: u32, start: &str, end: &str) -> Window {
        Window { weekday, start: start.into(), end: end.into() }
    }

    #[test]
    fn chia_khung_gio_theo_gio_viet_nam() {
        // Thứ Hai 16/11/2026 08:00 giờ VN = 01:00 UTC.
        let now = Utc.with_ymd_and_hms(2026, 11, 16, 1, 0, 0).unwrap();
        let windows = [w(1, "19:00", "21:00"), w(3, "14:00", "15:30")];
        let slots = candidate_slots(&windows, 45, now, 2, 7);
        let local: Vec<String> = slots.iter().map(|(s, _)| time::to_local(*s).format("%a %H:%M").to_string()).collect();
        // 19:00–21:00 chia 45 phút → 19:00, 19:45 (20:30 + 45 > 21:00); thứ Tư 14:00, 14:45 (vừa hết 15:30).
        // 19:00 thứ Hai tuần sau đã quá 7 ngày kể từ bây giờ → không có.
        assert_eq!(local, ["Mon 19:00", "Mon 19:45", "Wed 14:00", "Wed 14:45"]);
        assert_eq!(slots[0].1 - slots[0].0, TimeDelta::minutes(45));
        // Phải đặt trước ít nhất 12 giờ → 19:00 hôm nay (cách 11 giờ) bị bỏ.
        let soon = candidate_slots(&windows, 45, now, 12, 3);
        assert_eq!(time::to_local(soon[0].0).format("%a %H:%M").to_string(), "Wed 14:00");
    }

    #[test]
    fn bo_khoang_trung_gio_ban() {
        let now = Utc.with_ymd_and_hms(2026, 11, 16, 1, 0, 0).unwrap();
        let slots = candidate_slots(&[w(1, "19:00", "21:00")], 45, now, 0, 1);
        assert_eq!(slots.len(), 2);
        // Bận 19:30–19:50 → trùng cả hai khoảng 19:00–19:45 và 19:45–20:30.
        let busy = [(slots[0].0 + TimeDelta::minutes(30), slots[0].0 + TimeDelta::minutes(50))];
        assert!(free_slots(slots.clone(), &busy).is_empty());
        // Bận đúng tới 19:45 → còn khoảng sau.
        let busy = [(slots[0].0, slots[0].1)];
        assert_eq!(free_slots(slots.clone(), &busy), vec![slots[1]]);
    }
}
