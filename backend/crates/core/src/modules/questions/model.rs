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
}

impl QuestionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            QuestionKind::Single => "single",
            QuestionKind::Multiple => "multiple",
            QuestionKind::TrueFalse => "true_false",
            QuestionKind::Short => "short",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            QuestionKind::Single => "Một đáp án",
            QuestionKind::Multiple => "Nhiều đáp án",
            QuestionKind::TrueFalse => "Đúng/sai",
            QuestionKind::Short => "Trả lời ngắn",
        }
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
