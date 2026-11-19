use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    /// Nhiều lựa chọn, một đáp án đúng.
    Single,
    /// Nhiều lựa chọn, nhiều đáp án đúng (chọn đủ và đúng mới được điểm).
    Multiple,
    /// Đúng/sai từng ý (thường 4 ý a, b, c, d).
    TrueFalse,
    /// Trả lời ngắn (số hoặc chữ).
    Short,
    /// Tự luận: học sinh gõ / chụp ảnh bài làm, giáo viên chấm theo hướng dẫn chấm.
    Essay,
}

impl QuestionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            QuestionKind::Single => "single",
            QuestionKind::Multiple => "multiple",
            QuestionKind::TrueFalse => "true_false",
            QuestionKind::Short => "short",
            QuestionKind::Essay => "essay",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            QuestionKind::Single => "Một đáp án",
            QuestionKind::Multiple => "Nhiều đáp án",
            QuestionKind::TrueFalse => "Đúng/sai",
            QuestionKind::Short => "Trả lời ngắn",
            QuestionKind::Essay => "Tự luận",
        }
    }

    /// Giáo viên chấm tay (không chấm tự động được).
    pub fn is_manual(self) -> bool {
        self == QuestionKind::Essay
    }
}

impl fmt::Display for QuestionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for QuestionKind {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "single" => Ok(Self::Single),
            "multiple" => Ok(Self::Multiple),
            "true_false" => Ok(Self::TrueFalse),
            "short" => Ok(Self::Short),
            "essay" => Ok(Self::Essay),
            _ => Err(()),
        }
    }
}

/// Một phương án (hoặc một ý của câu đúng/sai). `id` cố định qua các phiên bản.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QOption {
    pub id: Uuid,
    pub content: String,
}

fn yes() -> bool {
    true
}

/// Một bước trong hướng dẫn chấm câu tự luận ("Tính Δ = 1 — 0,25 điểm"). `id` cố định qua các phiên bản.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RubricStep {
    pub id: Uuid,
    pub content: String,
    pub points: f64,
}

/// Cách so câu trả lời ngắn với đáp án.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    /// Cả hai là số thì so số (có sai số), không thì so chữ (bỏ khoảng trắng thừa, không phân biệt hoa thường).
    #[default]
    Auto,
    /// So biểu thức tương đương: `2(x+1)` = `2x + 2`.
    Expression,
}

impl MatchMode {
    pub fn is_auto(&self) -> bool {
        *self == MatchMode::Auto
    }
}

/// Nội dung một phiên bản câu hỏi (cột `question_versions.body`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionBody {
    pub content: String,
    #[serde(default)]
    pub options: Vec<QOption>,
    /// single / multiple: ID phương án đúng. true_false: ID các ý **đúng** (ý còn lại là sai).
    #[serde(default)]
    pub correct: Vec<Uuid>,
    /// short: các câu trả lời được chấp nhận.
    #[serde(default)]
    pub accepted: Vec<String>,
    #[serde(default)]
    pub explanation: String,
    /// `false` khi có phương án kiểu "Cả A và B" — không trộn thứ tự phương án.
    #[serde(default = "yes")]
    pub shuffle_options: bool,
    /// essay: hướng dẫn chấm theo bước (có thể trống — giáo viên cho điểm cả câu).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rubric: Vec<RubricStep>,
    /// short: sai số cho phép khi đáp án là số (`|trả lời − đáp án| ≤ tolerance`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    /// short: cách so đáp án.
    #[serde(default, skip_serializing_if = "MatchMode::is_auto")]
    pub match_mode: MatchMode,
}

impl QuestionBody {
    /// Tổng điểm các bước chấm (0 khi không có hướng dẫn chấm).
    pub fn rubric_total(&self) -> f64 {
        self.rubric.iter().map(|s| s.points).sum()
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuestionRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub kind: String,
    pub level: String,
    pub current_version_id: Option<Uuid>,
    pub version: i32,
    pub body: Json<QuestionBody>,
    pub tag_ids: Vec<Uuid>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Số lượt làm bài đã dùng câu này.
    pub used_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QuestionVersion {
    pub id: Uuid,
    pub question_id: Uuid,
    pub version: i32,
    pub kind: String,
    pub body: Json<QuestionBody>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Folder {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub sort_order: i32,
    pub question_count: i64,
}
