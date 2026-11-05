use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

use crate::modules::questions::grading::Answer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExamKind {
    /// Ôn tập: không giới hạn lần làm, kiểm tra từng câu, luôn xem được lời giải sau khi nộp.
    Practice,
    /// Kiểm tra: thời gian, khung giờ, số lần làm, chế độ hiện đáp án.
    Test,
}

impl ExamKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ExamKind::Practice => "practice",
            ExamKind::Test => "test",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShowAnswers {
    AfterSubmit,
    /// Sau khi đề đóng (`closes_at`); đề không có giờ đóng thì như `after_submit`.
    AfterClose,
    Never,
}

impl ShowAnswers {
    pub fn as_str(self) -> &'static str {
        match self {
            ShowAnswers::AfterSubmit => "after_submit",
            ShowAnswers::AfterClose => "after_close",
            ShowAnswers::Never => "never",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExamRow {
    pub id: Uuid,
    pub owner_id: Uuid,
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
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attempt_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SectionRow {
    pub id: Uuid,
    pub exam_id: Uuid,
    pub position: i32,
    pub title: String,
    pub scoring: String,
    pub points_per_question: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SectionQuestionRow {
    pub section_id: Uuid,
    pub position: i32,
    pub question_id: Uuid,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RuleRow {
    pub section_id: Uuid,
    pub position: i32,
    pub tag_id: Option<Uuid>,
    pub level: Option<String>,
    pub kind: Option<String>,
    pub folder_id: Option<Uuid>,
    pub count: i32,
}

/// Thông tin rút gọn của đề (dựng lộ trình).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExamRef {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub kind: String,
    pub time_limit_minutes: Option<i32>,
    pub archived: bool,
    /// Số câu mỗi lượt làm (câu cố định + số câu rút theo ma trận).
    pub item_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttemptRow {
    pub id: Uuid,
    pub exam_id: Uuid,
    pub user_id: Uuid,
    pub class_id: Option<Uuid>,
    pub step_id: Option<Uuid>,
    pub is_preview: bool,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub deadline_at: Option<DateTime<Utc>>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub score: f64,
    pub max_score: f64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttemptItemRow {
    pub position: i32,
    pub section_position: i32,
    pub section_title: String,
    pub scoring: String,
    pub points: f64,
    pub question_id: Uuid,
    pub question_version_id: Uuid,
    pub option_order: Vec<Uuid>,
    pub answer: Option<Json<Answer>>,
    pub checked: bool,
    pub is_correct: Option<bool>,
    pub score: f64,
}

/// Một dòng trong danh sách lượt làm (giáo viên xem kết quả).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttemptListRow {
    pub id: Uuid,
    pub exam_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub class_id: Option<Uuid>,
    pub is_preview: bool,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub score: f64,
    pub max_score: f64,
}

/// Điểm cao nhất của một học sinh với một đề trong một lớp học.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BestScoreRow {
    pub user_id: Uuid,
    pub exam_id: Uuid,
    pub best_score: Option<f64>,
    pub max_score: Option<f64>,
    pub submitted: i64,
    pub in_progress: Option<Uuid>,
}

/// Một câu đã nộp cần chấm lại.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RegradeItemRow {
    pub attempt_id: Uuid,
    pub position: i32,
    pub scoring: String,
    pub points: f64,
    pub answer: Option<Json<Answer>>,
    pub score: f64,
}
