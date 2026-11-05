use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::model::{Folder, QuestionKind, QuestionRow};
use crate::modules::catalog::model::QuestionLevel;

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OptionInput {
    /// ID phương án cũ (giữ khi sửa); bỏ trống = phương án mới.
    pub id: Option<Uuid>,
    #[validate(length(min = 1, max = 5000, message = "Nội dung phương án từ 1 đến 5.000 ký tự."))]
    pub content: String,
    /// single / multiple: là đáp án đúng. true_false: ý này **Đúng**.
    #[serde(default)]
    pub correct: bool,
}

/// Tạo / sửa câu hỏi.
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct QuestionInput {
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub kind: QuestionKind,
    pub level: QuestionLevel,
    #[serde(default)]
    #[validate(length(max = 20, message = "Tối đa 20 danh mục."))]
    pub tag_ids: Vec<Uuid>,
    #[validate(length(min = 1, max = 20000, message = "Nội dung câu hỏi từ 1 đến 20.000 ký tự."))]
    pub content: String,
    #[serde(default)]
    #[validate(nested)]
    pub options: Vec<OptionInput>,
    /// Trả lời ngắn: các câu trả lời được chấp nhận.
    #[serde(default)]
    pub accepted: Vec<String>,
    #[serde(default)]
    #[validate(length(max = 20000, message = "Lời giải tối đa 20.000 ký tự."))]
    pub explanation: String,
    #[serde(default = "default_true")]
    pub shuffle_options: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuestionQuery {
    pub q: Option<String>,
    pub subject_id: Option<Uuid>,
    /// ID thư mục, hoặc `none` = câu chưa xếp thư mục.
    pub folder_id: Option<String>,
    pub tag_id: Option<Uuid>,
    pub level: Option<QuestionLevel>,
    pub kind: Option<QuestionKind>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionResponse {
    pub id: Uuid,
    pub content: String,
    pub correct: bool,
}

/// Câu hỏi đầy đủ (giáo viên — có đáp án).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionResponse {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub kind: String,
    pub level: String,
    pub tag_ids: Vec<Uuid>,
    pub version: i32,
    pub content: String,
    pub options: Vec<OptionResponse>,
    pub accepted: Vec<String>,
    pub explanation: String,
    pub shuffle_options: bool,
    pub used_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<QuestionRow> for QuestionResponse {
    fn from(q: QuestionRow) -> Self {
        let body = q.body.0;
        Self {
            id: q.id,
            subject_id: q.subject_id,
            grade_id: q.grade_id,
            folder_id: q.folder_id,
            kind: q.kind,
            level: q.level,
            tag_ids: q.tag_ids,
            version: q.version,
            options: body
                .options
                .iter()
                .map(|o| OptionResponse { id: o.id, content: o.content.clone(), correct: body.correct.contains(&o.id) })
                .collect(),
            content: body.content,
            accepted: body.accepted,
            explanation: body.explanation,
            shuffle_options: body.shuffle_options,
            used_count: q.used_count,
            created_at: q.created_at,
            updated_at: q.updated_at,
        }
    }
}

/// Kết quả lưu câu hỏi: đáp án đúng có đổi không, bao nhiêu bài đã nộp bị ảnh hưởng (để hỏi "Chấm lại?").
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveQuestionResponse {
    pub question: QuestionResponse,
    pub answer_changed: bool,
    pub affected_attempts: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegradeResponse {
    /// Số bài đã nộp được chấm lại.
    pub regraded_attempts: i64,
    /// Số bài có điểm thay đổi.
    pub changed_attempts: i64,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FolderInput {
    #[validate(length(min = 1, max = 100, message = "Tên thư mục từ 1 đến 100 ký tự."))]
    pub name: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderResponse {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub question_count: i64,
}

impl From<Folder> for FolderResponse {
    fn from(f: Folder) -> Self {
        Self { id: f.id, parent_id: f.parent_id, name: f.name, question_count: f.question_count }
    }
}

/// Lưu hàng loạt (sau bước xem trước import).
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateRequest {
    #[validate(length(min = 1, max = 500, message = "Mỗi lần lưu từ 1 đến 500 câu."))]
    pub items: Vec<QuestionInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateResponse {
    pub created: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportQuery {
    pub subject_id: Uuid,
}

/// Một dòng Excel đã đọc (dạng giống `QuestionInput` để giao diện sửa rồi gửi lại).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItem {
    /// Số dòng trong file (bắt đầu từ 2, dòng 1 là tiêu đề).
    pub row: usize,
    pub kind: Option<QuestionKind>,
    pub level: Option<QuestionLevel>,
    pub content: String,
    pub options: Vec<OptionInput>,
    pub accepted: Vec<String>,
    pub explanation: String,
    pub tag_ids: Vec<Uuid>,
    /// Lỗi khiến không lưu được dòng này.
    pub errors: Vec<String>,
    /// Cảnh báo (vẫn lưu được), ví dụ danh mục không có trong hệ thống.
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub items: Vec<ImportItem>,
    pub valid_count: usize,
}
