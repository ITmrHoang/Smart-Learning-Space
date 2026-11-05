use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::model::{AttemptListRow, ExamKind, ShowAnswers};
use crate::modules::{
    catalog::model::QuestionLevel,
    questions::{
        dto::QuestionResponse,
        grading::{Answer, Scoring},
        model::QuestionKind,
    },
};

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RuleInput {
    pub tag_id: Option<Uuid>,
    pub level: Option<QuestionLevel>,
    pub kind: Option<QuestionKind>,
    pub folder_id: Option<Uuid>,
    #[validate(range(min = 1, max = 100, message = "Số câu mỗi ô ma trận từ 1 đến 100."))]
    pub count: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SectionInput {
    #[validate(length(min = 1, max = 100, message = "Tên phần từ 1 đến 100 ký tự."))]
    pub title: String,
    pub scoring: Scoring,
    #[validate(range(min = 0.0, max = 100.0, message = "Điểm mỗi câu từ 0 đến 100."))]
    pub points_per_question: f64,
    #[serde(default)]
    #[validate(length(max = 200, message = "Mỗi phần tối đa 200 câu cố định."))]
    pub question_ids: Vec<Uuid>,
    #[serde(default)]
    #[validate(length(max = 30, message = "Mỗi phần tối đa 30 ô ma trận."), nested)]
    pub rules: Vec<RuleInput>,
}

/// Tạo / sửa đề (gửi cả cấu trúc phần).
#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ExamInput {
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    #[validate(length(min = 1, max = 200, message = "Tên đề từ 1 đến 200 ký tự."))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 5000, message = "Mô tả tối đa 5.000 ký tự."))]
    pub description: String,
    pub kind: ExamKind,
    #[validate(range(min = 1, max = 600, message = "Thời gian làm bài từ 1 đến 600 phút."))]
    pub time_limit_minutes: Option<i32>,
    #[validate(range(min = 1, max = 100, message = "Số lần làm từ 1 đến 100."))]
    pub max_attempts: Option<i32>,
    pub opens_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    #[serde(default = "default_true")]
    pub shuffle_questions: bool,
    #[serde(default = "default_true")]
    pub shuffle_options: bool,
    pub show_answers: ShowAnswers,
    #[validate(length(min = 1, max = 10, message = "Đề có từ 1 đến 10 phần."), nested)]
    pub sections: Vec<SectionInput>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExamQuery {
    pub q: Option<String>,
    pub subject_id: Option<Uuid>,
    pub kind: Option<ExamKind>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleResponse {
    pub tag_id: Option<Uuid>,
    pub level: Option<String>,
    pub kind: Option<String>,
    pub folder_id: Option<Uuid>,
    pub count: i32,
    /// Số câu trong ngân hàng khớp ô này (không tính câu cố định của đề).
    pub available: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionResponse {
    pub id: Uuid,
    pub title: String,
    pub scoring: String,
    pub points_per_question: f64,
    pub questions: Vec<QuestionResponse>,
    pub rules: Vec<RuleResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamResponse {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub time_limit_minutes: Option<i32>,
    pub max_attempts: Option<i32>,
    pub opens_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    pub shuffle_questions: bool,
    pub shuffle_options: bool,
    pub show_answers: String,
    pub sections: Vec<SectionResponse>,
    /// Số câu mỗi lượt làm.
    pub question_count: i64,
    pub max_score: f64,
    /// Ô ma trận không đủ câu, câu cố định đã bị ẩn…
    pub warnings: Vec<String>,
    pub attempt_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamSummary {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: String,
    pub kind: String,
    pub time_limit_minutes: Option<i32>,
    pub question_count: i64,
    pub attempt_count: i64,
    pub updated_at: DateTime<Utc>,
}

// ---------- Làm bài ----------

#[derive(Debug, Serialize)]
pub struct ViewOption {
    pub id: Uuid,
    pub content: String,
}

/// Đáp án + lời giải của một câu (chỉ gửi khi được phép xem).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemResult {
    pub is_correct: bool,
    pub score: f64,
    pub correct: Vec<Uuid>,
    pub accepted: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub position: i32,
    pub section_position: i32,
    pub section_title: String,
    pub points: f64,
    pub kind: String,
    pub content: String,
    /// Theo thứ tự đã trộn của lượt làm này.
    pub options: Vec<ViewOption>,
    pub answer: Option<Answer>,
    /// Ôn tập: đã bấm "Kiểm tra" (không sửa được nữa).
    pub checked: bool,
    pub result: Option<ItemResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentInfo {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptView {
    pub id: Uuid,
    pub exam_id: Uuid,
    pub class_id: Option<Uuid>,
    pub step_id: Option<Uuid>,
    pub title: String,
    pub exam_kind: String,
    pub is_preview: bool,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub deadline_at: Option<DateTime<Utc>>,
    /// Giờ server, để trình duyệt tính đồng hồ đếm ngược đúng dù đồng hồ máy lệch.
    pub now: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    /// Điểm (sau khi nộp).
    pub score: Option<f64>,
    pub max_score: f64,
    /// Được xem đáp án + lời giải.
    pub reveal: bool,
    /// Ôn tập đang làm: được bấm "Kiểm tra" từng câu.
    pub can_check: bool,
    pub student: Option<StudentInfo>,
    pub items: Vec<ItemView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptSummary {
    pub id: Uuid,
    pub exam_id: Uuid,
    pub class_id: Option<Uuid>,
    pub is_preview: bool,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub score: Option<f64>,
    pub max_score: f64,
    pub student: StudentInfo,
}

impl From<AttemptListRow> for AttemptSummary {
    fn from(r: AttemptListRow) -> Self {
        let submitted = r.status == "submitted";
        Self {
            id: r.id,
            exam_id: r.exam_id,
            class_id: r.class_id,
            is_preview: r.is_preview,
            status: r.status,
            started_at: r.started_at,
            submitted_at: r.submitted_at,
            score: submitted.then_some(r.score),
            max_score: r.max_score,
            student: StudentInfo { id: r.user_id, username: r.username, full_name: r.full_name },
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AttemptQuery {
    pub class_id: Option<Uuid>,
}
