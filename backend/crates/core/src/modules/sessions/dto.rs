use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::model::{Access, SessionRow};
use crate::modules::{groups::StudentRef, lessons::dto::validate_http_url};

fn validate_weekdays(days: &[u32]) -> Result<(), ValidationError> {
    if days.is_empty() || days.len() > 7 || days.iter().any(|d| !(1..=7).contains(d)) {
        return Err(ValidationError::new("weekdays").with_message("Chọn ít nhất một thứ trong tuần.".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RepeatInput {
    /// 1 = thứ Hai … 7 = Chủ nhật
    #[validate(custom(function = "validate_weekdays"))]
    pub weekdays: Vec<u32>,
    /// Ngày kết thúc (giờ Việt Nam), tính cả ngày này.
    pub until: NaiveDate,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct InviteesInput {
    #[serde(default)]
    #[validate(length(max = 20, message = "Tối đa 20 lớp học."))]
    pub class_ids: Vec<Uuid>,
    #[serde(default)]
    #[validate(length(max = 50, message = "Tối đa 50 nhóm."))]
    pub group_ids: Vec<Uuid>,
    #[serde(default)]
    #[validate(length(max = 500, message = "Tối đa 500 học sinh."))]
    pub user_ids: Vec<Uuid>,
}

impl InviteesInput {
    pub fn is_empty(&self) -> bool {
        self.class_ids.is_empty() && self.group_ids.is_empty() && self.user_ids.is_empty()
    }
}

/// Nội dung buổi (dùng chung cho tạo và sửa).
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SessionFields {
    #[validate(length(min = 1, max = 200, message = "Tên buổi từ 1 đến 200 ký tự."))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 5000, message = "Mô tả tối đa 5.000 ký tự."))]
    pub description: String,
    /// Lớp học của buổi (buổi lẻ học kèm có thể để trống).
    pub class_id: Option<Uuid>,
    pub starts_at: DateTime<Utc>,
    #[validate(range(min = 15, max = 600, message = "Thời lượng từ 15 đến 600 phút."))]
    pub duration_minutes: i64,
    #[serde(default)]
    #[validate(nested)]
    pub invitees: InviteesInput,
    pub access: Access,
    #[validate(length(max = 500), custom(function = "validate_http_url"))]
    pub meeting_url: Option<String>,
    /// Nhắc trước bao nhiêu phút (0 = không nhắc).
    #[validate(range(min = 0, max = 10080, message = "Nhắc trước tối đa 7 ngày."))]
    #[serde(default = "default_remind")]
    pub remind_minutes: i32,
    #[serde(default)]
    #[validate(length(max = 20, message = "Tối đa 20 bài."))]
    pub lesson_ids: Vec<Uuid>,
}

fn default_remind() -> i32 {
    60
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionInput {
    #[serde(flatten)]
    #[validate(nested)]
    pub fields: SessionFields,
    #[validate(nested)]
    pub repeat: Option<RepeatInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EditScope {
    /// Chỉ buổi này.
    #[default]
    This,
    /// Buổi này và các buổi sau (cùng lịch lặp).
    Following,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionInput {
    #[serde(flatten)]
    #[validate(nested)]
    pub fields: SessionFields,
    #[serde(default)]
    pub scope: EditScope,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CancelInput {
    #[serde(default)]
    pub scope: EditScope,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RangeQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub class_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rsvp {
    Yes,
    No,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RsvpInput {
    pub response: Rsvp,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceInput {
    pub user_id: Uuid,
    pub attended: bool,
}

/// Một buổi trên lịch.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: Uuid,
    pub schedule_id: Option<Uuid>,
    pub class_id: Option<Uuid>,
    pub class_title: Option<String>,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
    pub access: String,
    pub teacher_name: String,
    pub is_owner: bool,
    /// Trả lời của người xem (học sinh).
    pub my_rsvp: Option<String>,
}

impl SessionSummary {
    pub fn new(s: &SessionRow, viewer: Uuid, my_rsvp: Option<String>) -> Self {
        Self {
            id: s.id,
            schedule_id: s.schedule_id,
            class_id: s.class_id,
            class_title: s.class_title.clone(),
            title: s.title.clone(),
            starts_at: s.starts_at,
            ends_at: s.ends_at,
            status: s.status.clone(),
            access: s.access.clone(),
            teacher_name: s.owner_name.clone(),
            is_owner: s.owner_id == viewer,
            my_rsvp,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedRef {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InviteesView {
    pub classes: Vec<NamedRef>,
    pub groups: Vec<NamedRef>,
    pub users: Vec<NamedRef>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttendeeView {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub rsvp: Option<String>,
    pub joined_at: Option<DateTime<Utc>>,
    pub attended: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    #[serde(flatten)]
    pub summary: SessionSummary,
    pub description: String,
    pub duration_minutes: i64,
    pub remind_minutes: i32,
    /// Chỉ người được vào mới thấy link.
    pub meeting_url: Option<String>,
    pub lessons: Vec<NamedRef>,
    /// Giáo viên: ai được mời.
    pub invitees: Option<InviteesView>,
    /// Giáo viên: danh sách học sinh được mời + trả lời + điểm danh.
    pub attendees: Option<Vec<AttendeeView>>,
    /// Lúc mở nút "Vào học" (10 phút trước giờ học).
    pub join_opens_at: DateTime<Utc>,
    pub can_join: bool,
    pub attended: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionResponse {
    pub sessions: Vec<SessionSummary>,
    /// Số học sinh được mời đang có buổi khác trùng giờ.
    pub student_conflicts: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionResponse {
    pub updated: usize,
    pub student_conflicts: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinResponse {
    pub meeting_url: Option<String>,
}

pub type Candidate = StudentRef;
