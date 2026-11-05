use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Bản nháp bài học (+ danh mục, phiên bản đã xuất bản).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Lesson {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub video_url: Option<String>,
    pub duration_minutes: Option<i32>,
    pub published_revision_id: Option<Uuid>,
    pub has_changes: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// `array(SELECT tag_id FROM lesson_tags …)`
    pub tag_ids: Vec<Uuid>,
    /// Số phiên bản đang xuất bản.
    pub published_version: Option<i32>,
}

/// Một lần xuất bản.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LessonRevision {
    pub id: Uuid,
    pub lesson_id: Uuid,
    pub version: i32,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub video_url: Option<String>,
    pub duration_minutes: Option<i32>,
    pub published_at: DateTime<Utc>,
}

/// Thông tin rút gọn dùng khi dựng lộ trình (bài nào, của ai, đã xuất bản chưa).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LessonRef {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub duration_minutes: Option<i32>,
    pub published_revision_id: Option<Uuid>,
    pub archived: bool,
}
