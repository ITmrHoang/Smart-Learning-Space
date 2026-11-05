use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::model::{Lesson, LessonRevision};

/// Link video: chỉ nhận http(s).
pub fn validate_http_url(value: &str) -> Result<(), ValidationError> {
    let v = value.trim();
    if (v.starts_with("https://") || v.starts_with("http://")) && !v.chars().any(char::is_whitespace) {
        Ok(())
    } else {
        Err(ValidationError::new("url").with_message("Link phải bắt đầu bằng http:// hoặc https://".into()))
    }
}

/// Tạo / sửa bản nháp bài học.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LessonInput {
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    #[validate(length(min = 1, max = 200, message = "Tên bài từ 1 đến 200 ký tự."))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 500, message = "Tóm tắt tối đa 500 ký tự."))]
    pub summary: String,
    #[serde(default)]
    #[validate(length(max = 100000, message = "Nội dung quá dài (tối đa 100.000 ký tự)."))]
    pub content: String,
    #[validate(length(max = 500), custom(function = "validate_http_url"))]
    pub video_url: Option<String>,
    #[validate(range(min = 1, max = 600, message = "Thời lượng từ 1 đến 600 phút."))]
    pub duration_minutes: Option<i32>,
    #[serde(default)]
    #[validate(length(max = 20, message = "Tối đa 20 danh mục."))]
    pub tag_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LessonQuery {
    pub q: Option<String>,
    pub subject_id: Option<Uuid>,
    /// `draft` (chưa xuất bản lần nào) | `published` | `changed` (có thay đổi chưa xuất bản)
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonResponse {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub video_url: Option<String>,
    pub duration_minutes: Option<i32>,
    pub tag_ids: Vec<Uuid>,
    /// Phiên bản đang xuất bản; `null` = chưa xuất bản lần nào.
    pub published_version: Option<i32>,
    /// Bản nháp có thay đổi chưa xuất bản.
    pub has_changes: bool,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Lesson> for LessonResponse {
    fn from(l: Lesson) -> Self {
        Self {
            id: l.id,
            subject_id: l.subject_id,
            grade_id: l.grade_id,
            title: l.title,
            summary: l.summary,
            content: l.content,
            video_url: l.video_url,
            duration_minutes: l.duration_minutes,
            tag_ids: l.tag_ids,
            published_version: l.published_version,
            has_changes: l.has_changes,
            archived: l.archived_at.is_some(),
            created_at: l.created_at,
            updated_at: l.updated_at,
        }
    }
}

/// Dòng trong danh sách (không kèm nội dung).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonSummary {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: String,
    pub summary: String,
    pub has_video: bool,
    pub duration_minutes: Option<i32>,
    pub tag_ids: Vec<Uuid>,
    pub published_version: Option<i32>,
    pub has_changes: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<Lesson> for LessonSummary {
    fn from(l: Lesson) -> Self {
        Self {
            id: l.id,
            subject_id: l.subject_id,
            grade_id: l.grade_id,
            title: l.title,
            summary: l.summary,
            has_video: l.video_url.is_some(),
            duration_minutes: l.duration_minutes,
            tag_ids: l.tag_ids,
            published_version: l.published_version,
            has_changes: l.has_changes,
            updated_at: l.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionSummary {
    pub version: i32,
    pub title: String,
    pub published_at: DateTime<Utc>,
}

/// Bài đã xuất bản — nội dung học sinh đọc.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedLesson {
    pub lesson_id: Uuid,
    pub version: i32,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub video_url: Option<String>,
    pub duration_minutes: Option<i32>,
    pub published_at: DateTime<Utc>,
}

impl From<LessonRevision> for PublishedLesson {
    fn from(r: LessonRevision) -> Self {
        Self {
            lesson_id: r.lesson_id,
            version: r.version,
            title: r.title,
            summary: r.summary,
            content: r.content,
            video_url: r.video_url,
            duration_minutes: r.duration_minutes,
            published_at: r.published_at,
        }
    }
}
