//! Thông báo trong app (M15): chuông + số chưa đọc. Module khác gọi `service::notify` để gửi.
//! Hiện frontend hỏi lại mỗi 60 giây; WebSocket (realtime) làm cùng phòng live.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    pagination::{Page, PageRequest},
    security::Actor,
};

/// Loại thông báo (giao diện chọn icon theo loại).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    SessionInvite,
    SessionUpdated,
    SessionCancelled,
    SessionReminder,
    EnrollmentApproved,
    ClassAdded,
    PathPublished,
    /// Hỏi đáp: có câu hỏi mới (gửi giáo viên).
    QuestionNew,
    /// Hỏi đáp: có trả lời mới (gửi người hỏi + người đã trả lời).
    QuestionReply,
    /// Có nội dung bị báo vi phạm (gửi giáo viên).
    ContentReported,
    /// Bài tập: học sinh nộp bài (gửi giáo viên).
    HomeworkSubmitted,
    /// Bài tập: đã chấm / có nhận xét (gửi học sinh).
    HomeworkGraded,
    /// Bài tập: giáo viên trả lại để làm lại.
    HomeworkReturned,
    /// Bài tập: sắp hết hạn nộp.
    HomeworkDue,
    /// Học kèm 1-1: có yêu cầu đặt lịch mới (gửi giáo viên).
    BookingRequested,
    /// Học kèm 1-1: giáo viên đồng ý.
    BookingApproved,
    /// Học kèm 1-1: giáo viên từ chối.
    BookingDeclined,
    /// Học kèm 1-1: một bên hủy lịch.
    BookingCancelled,
    /// Được mời làm trợ giảng một lớp học.
    AssistantAdded,
}

impl NotificationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SessionInvite => "session_invite",
            Self::SessionUpdated => "session_updated",
            Self::SessionCancelled => "session_cancelled",
            Self::SessionReminder => "session_reminder",
            Self::EnrollmentApproved => "enrollment_approved",
            Self::ClassAdded => "class_added",
            Self::PathPublished => "path_published",
            Self::QuestionNew => "question_new",
            Self::QuestionReply => "question_reply",
            Self::ContentReported => "content_reported",
            Self::HomeworkSubmitted => "homework_submitted",
            Self::HomeworkGraded => "homework_graded",
            Self::HomeworkReturned => "homework_returned",
            Self::HomeworkDue => "homework_due",
            Self::BookingRequested => "booking_requested",
            Self::BookingApproved => "booking_approved",
            Self::BookingDeclined => "booking_declined",
            Self::BookingCancelled => "booking_cancelled",
            Self::AssistantAdded => "assistant_added",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationResponse {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NotificationQuery {
    #[serde(default)]
    pub unread: bool,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadCount {
    pub unread: i64,
}

/// Gửi cùng một thông báo cho nhiều người (bỏ trùng).
pub async fn notify<'e, E: PgExecutor<'e>>(
    db: E,
    user_ids: &[Uuid],
    kind: NotificationKind,
    title: &str,
    body: &str,
    link: Option<&str>,
) -> AppResult<()> {
    if user_ids.is_empty() {
        return Ok(());
    }
    let mut ids = user_ids.to_vec();
    ids.sort();
    ids.dedup();
    sqlx::query(
        "INSERT INTO notifications (user_id, kind, title, body, link) SELECT unnest($1::uuid[]), $2, $3, $4, $5",
    )
    .bind(&ids)
    .bind(kind.as_str())
    .bind(title)
    .bind(body)
    .bind(link)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list(ctx: &AppContext, actor: &Actor, q: NotificationQuery) -> AppResult<Page<NotificationResponse>> {
    let page = PageRequest::new(q.page, q.page_size);
    let items = sqlx::query_as::<_, NotificationResponse>(
        "SELECT id, kind, title, body, link, read_at, created_at FROM notifications \
         WHERE user_id = $1 AND (NOT $2 OR read_at IS NULL) ORDER BY created_at DESC, id DESC LIMIT $3 OFFSET $4",
    )
    .bind(actor.id)
    .bind(q.unread)
    .bind(page.limit())
    .bind(page.offset())
    .fetch_all(&ctx.db)
    .await?;
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM notifications WHERE user_id = $1 AND (NOT $2 OR read_at IS NULL)",
    )
    .bind(actor.id)
    .bind(q.unread)
    .fetch_one(&ctx.db)
    .await?;
    Ok(Page::new(items, total, page))
}

pub async fn unread_count(ctx: &AppContext, actor: &Actor) -> AppResult<UnreadCount> {
    let unread =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM notifications WHERE user_id = $1 AND read_at IS NULL")
            .bind(actor.id)
            .fetch_one(&ctx.db)
            .await?;
    Ok(UnreadCount { unread })
}

pub async fn mark_read(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    let res = sqlx::query("UPDATE notifications SET read_at = COALESCE(read_at, now()) WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(actor.id)
        .execute(&ctx.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("Không tìm thấy thông báo."));
    }
    Ok(())
}

pub async fn mark_all_read(ctx: &AppContext, actor: &Actor) -> AppResult<()> {
    sqlx::query("UPDATE notifications SET read_at = now() WHERE user_id = $1 AND read_at IS NULL")
        .bind(actor.id)
        .execute(&ctx.db)
        .await?;
    Ok(())
}
