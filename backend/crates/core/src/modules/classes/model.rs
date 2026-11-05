use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassMode {
    SelfPaced,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Unlisted,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enrollment {
    Open,
    Approval,
    Invite,
}

impl ClassMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ClassMode::SelfPaced => "self_paced",
            ClassMode::Live => "live",
        }
    }
}

impl Visibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Unlisted => "unlisted",
            Visibility::Private => "private",
        }
    }
}

impl Enrollment {
    pub fn as_str(self) -> &'static str {
        match self {
            Enrollment::Open => "open",
            Enrollment::Approval => "approval",
            Enrollment::Invite => "invite",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClassRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub subject_id: Uuid,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub visibility: String,
    pub enrollment: String,
    pub invite_code: String,
    pub copied_from: Option<Uuid>,
    pub path_updated_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub grade_ids: Vec<Uuid>,
    pub tag_ids: Vec<Uuid>,
    pub owner_name: String,
    pub student_count: i64,
    pub pending_count: i64,
    pub path_version: Option<i32>,
    pub path_published_at: Option<DateTime<Utc>>,
    /// Số bước bài / đề trong lộ trình đã xuất bản.
    pub step_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemberRow {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub email: String,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Membership {
    pub role: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    Milestone,
    Lesson,
    Exam,
}

impl StepKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StepKind::Milestone => "milestone",
            StepKind::Lesson => "lesson",
            StepKind::Exam => "exam",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unlock {
    /// Học lúc nào cũng được.
    Free,
    /// Phải xong bước bài / đề ngay trước (bỏ qua mốc).
    Previous,
}

impl Unlock {
    pub fn as_str(self) -> &'static str {
        match self {
            Unlock::Free => "free",
            Unlock::Previous => "previous",
        }
    }
}

/// Một bước của lộ trình (bản nháp hoặc trong ảnh chụp đã xuất bản).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: Uuid,
    pub kind: StepKind,
    pub title: String,
    pub lesson_id: Option<Uuid>,
    pub exam_id: Option<Uuid>,
    pub unlock: Unlock,
    /// Bước đề: điểm cao nhất phải ≥ mức này mới tính là xong.
    pub pass_score: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StepRow {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub lesson_id: Option<Uuid>,
    pub exam_id: Option<Uuid>,
    pub unlock: String,
    pub pass_score: Option<f64>,
}

impl From<StepRow> for Step {
    fn from(r: StepRow) -> Self {
        Self {
            id: r.id,
            kind: match r.kind.as_str() {
                "lesson" => StepKind::Lesson,
                "exam" => StepKind::Exam,
                _ => StepKind::Milestone,
            },
            title: r.title,
            lesson_id: r.lesson_id,
            exam_id: r.exam_id,
            unlock: if r.unlock == "previous" { Unlock::Previous } else { Unlock::Free },
            pass_score: r.pass_score,
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PathVersionRow {
    pub version: i32,
    pub steps: Json<Vec<Step>>,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProgressRow {
    pub user_id: Uuid,
    pub step_id: Uuid,
    pub completed_at: DateTime<Utc>,
}
