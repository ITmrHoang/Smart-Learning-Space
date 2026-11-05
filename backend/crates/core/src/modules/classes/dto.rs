use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::model::{ClassMode, ClassRow, Enrollment, MemberRow, StepKind, Unlock, Visibility};
use crate::modules::lessons::dto::PublishedLesson;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ClassInput {
    pub subject_id: Uuid,
    #[serde(default)]
    #[validate(length(max = 12, message = "Tối đa 12 lớp."))]
    pub grade_ids: Vec<Uuid>,
    #[serde(default)]
    #[validate(length(max = 20, message = "Tối đa 20 danh mục."))]
    pub tag_ids: Vec<Uuid>,
    #[validate(length(min = 1, max = 150, message = "Tên lớp học từ 1 đến 150 ký tự."))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 5000, message = "Giới thiệu tối đa 5.000 ký tự."))]
    pub description: String,
    pub mode: ClassMode,
    pub visibility: Visibility,
    pub enrollment: Enrollment,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClassQuery {
    pub q: Option<String>,
    pub subject_id: Option<Uuid>,
    pub grade_id: Option<Uuid>,
    /// Giáo viên: xem cả lớp học đã lưu trữ.
    #[serde(default)]
    pub archived: bool,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// Lớp học (giáo viên chủ lớp xem).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassResponse {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_ids: Vec<Uuid>,
    pub tag_ids: Vec<Uuid>,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub visibility: String,
    pub enrollment: String,
    pub invite_code: String,
    pub student_count: i64,
    pub pending_count: i64,
    pub step_count: i64,
    pub path_version: Option<i32>,
    pub path_published_at: Option<DateTime<Utc>>,
    /// Bản nháp lộ trình có thay đổi chưa xuất bản.
    pub path_has_changes: bool,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ClassRow> for ClassResponse {
    fn from(c: ClassRow) -> Self {
        let path_has_changes = match (c.path_updated_at, c.path_published_at) {
            (Some(edited), Some(published)) => edited > published,
            (Some(_), None) => true,
            _ => false,
        };
        Self {
            id: c.id,
            subject_id: c.subject_id,
            grade_ids: c.grade_ids,
            tag_ids: c.tag_ids,
            title: c.title,
            description: c.description,
            mode: c.mode,
            visibility: c.visibility,
            enrollment: c.enrollment,
            invite_code: c.invite_code,
            student_count: c.student_count,
            pending_count: c.pending_count,
            step_count: c.step_count,
            path_version: c.path_version,
            path_published_at: c.path_published_at,
            path_has_changes,
            archived: c.archived_at.is_some(),
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

/// Lớp học như học sinh thấy (danh sách công khai, lớp của tôi).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassCard {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_ids: Vec<Uuid>,
    pub tag_ids: Vec<Uuid>,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub visibility: String,
    pub enrollment: String,
    pub teacher_name: String,
    pub student_count: i64,
    pub step_count: i64,
    /// Trạng thái ghi danh của người xem: pending | active | rejected | left | removed (null = chưa ghi danh).
    pub my_status: Option<String>,
    pub is_owner: bool,
}

impl ClassCard {
    pub fn new(c: ClassRow, my_status: Option<String>, viewer: Option<Uuid>) -> Self {
        Self {
            is_owner: viewer == Some(c.owner_id),
            id: c.id,
            subject_id: c.subject_id,
            grade_ids: c.grade_ids,
            tag_ids: c.tag_ids,
            title: c.title,
            description: c.description,
            mode: c.mode,
            visibility: c.visibility,
            enrollment: c.enrollment,
            teacher_name: c.owner_name,
            student_count: c.student_count,
            step_count: c.step_count,
            my_status,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineStep {
    pub kind: StepKind,
    pub title: String,
    pub duration_minutes: Option<i32>,
}

/// Trang giới thiệu lớp học: thông tin + mục lục lộ trình (chỉ tên bước).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassDetail {
    #[serde(flatten)]
    pub card: ClassCard,
    pub outline: Vec<OutlineStep>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberResponse {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub email: String,
    pub status: String,
    pub joined_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<MemberRow> for MemberResponse {
    fn from(m: MemberRow) -> Self {
        Self {
            user_id: m.user_id,
            username: m.username,
            full_name: m.full_name,
            email: m.email,
            status: m.status,
            joined_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemberQuery {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddMemberRequest {
    /// Username hoặc email của học sinh.
    #[validate(length(min = 1, max = 254, message = "Nhập username hoặc email."))]
    pub login: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct JoinRequest {
    #[validate(length(min = 4, max = 20, message = "Mã mời không hợp lệ."))]
    pub code: String,
}

#[derive(Debug, Deserialize, Validate, Default)]
#[serde(rename_all = "camelCase")]
pub struct CopyRequest {
    #[validate(length(min = 1, max = 150, message = "Tên lớp học từ 1 đến 150 ký tự."))]
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollResponse {
    pub class_id: Uuid,
    pub status: String,
}

// ---------- Lộ trình ----------

#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct StepInput {
    /// ID bước cũ (giữ để không mất tiến độ); bỏ trống = bước mới.
    pub id: Option<Uuid>,
    pub kind: StepKind,
    #[serde(default)]
    #[validate(length(max = 200, message = "Tên bước tối đa 200 ký tự."))]
    pub title: String,
    pub lesson_id: Option<Uuid>,
    pub exam_id: Option<Uuid>,
    #[serde(default = "default_unlock")]
    pub unlock: Unlock,
    #[validate(range(min = 0.0, max = 1000.0, message = "Điểm đạt từ 0 đến 1000."))]
    pub pass_score: Option<f64>,
}

fn default_unlock() -> Unlock {
    Unlock::Free
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PathInput {
    #[validate(length(max = 300, message = "Lộ trình tối đa 300 bước."), nested)]
    pub steps: Vec<StepInput>,
}

/// Bước trong bản nháp (giáo viên xem), kèm thông tin bài / đề được trỏ tới.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftStep {
    pub id: Uuid,
    pub kind: StepKind,
    pub title: String,
    pub lesson_id: Option<Uuid>,
    pub exam_id: Option<Uuid>,
    pub unlock: Unlock,
    pub pass_score: Option<f64>,
    /// Tên bài / đề.
    pub ref_title: Option<String>,
    /// Bài: published | draft | archived. Đề: ready | empty | archived.
    pub ref_status: Option<String>,
    pub duration_minutes: Option<i32>,
    pub exam_kind: Option<String>,
    pub item_count: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathResponse {
    pub steps: Vec<DraftStep>,
    pub published_version: Option<i32>,
    pub published_at: Option<DateTime<Utc>>,
    pub has_changes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Done,
    Available,
    Locked,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnStep {
    pub id: Uuid,
    pub kind: StepKind,
    pub title: String,
    pub status: StepStatus,
    pub lesson_id: Option<Uuid>,
    pub exam_id: Option<Uuid>,
    pub exam_kind: Option<String>,
    pub duration_minutes: Option<i32>,
    pub time_limit_minutes: Option<i32>,
    pub item_count: Option<i64>,
    pub pass_score: Option<f64>,
    pub best_score: Option<f64>,
    pub max_score: Option<f64>,
    pub submitted_count: i64,
    pub in_progress_attempt_id: Option<Uuid>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressSummary {
    pub done: i64,
    pub total: i64,
    pub percent: i64,
}

impl ProgressSummary {
    pub fn new(done: i64, total: i64) -> Self {
        let percent = if total == 0 { 0 } else { done * 100 / total };
        Self { done, total, percent }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnView {
    pub class: ClassCard,
    pub steps: Vec<LearnStep>,
    pub progress: ProgressSummary,
    pub path_version: Option<i32>,
    /// Giáo viên chủ lớp xem như học sinh (không ghi tiến độ, mọi bước đều mở).
    pub is_preview: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepLesson {
    pub step: LearnStep,
    pub lesson: PublishedLesson,
    pub prev_step_id: Option<Uuid>,
    pub next_step_id: Option<Uuid>,
    pub is_preview: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartedAttempt {
    pub attempt_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyClass {
    pub class: ClassCard,
    pub progress: ProgressSummary,
}

// ---------- Tiến độ (giáo viên) ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressStep {
    pub id: Uuid,
    pub kind: StepKind,
    pub title: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressCell {
    pub step_id: Uuid,
    pub done: bool,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentProgress {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub progress: ProgressSummary,
    pub cells: Vec<ProgressCell>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassProgress {
    pub steps: Vec<ProgressStep>,
    pub students: Vec<StudentProgress>,
}
