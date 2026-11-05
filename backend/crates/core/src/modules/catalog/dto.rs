use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::model::{Grade, PendingItem, QuestionLevel, Subject, Tag};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub sort_order: i32,
    pub is_active: bool,
}

impl From<Subject> for SubjectResponse {
    fn from(s: Subject) -> Self {
        Self { id: s.id, name: s.name, slug: s.slug, sort_order: s.sort_order, is_active: s.is_active }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub sort_order: i32,
    pub is_system: bool,
    /// `true` = dùng chung; `false` = lớp riêng của giáo viên `ownerId`.
    pub is_shared: bool,
    pub owner_id: Option<Uuid>,
    pub is_active: bool,
}

impl From<Grade> for GradeResponse {
    fn from(g: Grade) -> Self {
        Self {
            id: g.id,
            name: g.name,
            slug: g.slug,
            sort_order: g.sort_order,
            is_system: g.is_system,
            is_shared: g.owner_id.is_none(),
            owner_id: g.owner_id,
            is_active: g.is_active,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagResponse {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub name: String,
    pub slug: String,
    pub sort_order: i32,
    pub is_shared: bool,
    pub owner_id: Option<Uuid>,
    pub is_active: bool,
}

impl From<Tag> for TagResponse {
    fn from(t: Tag) -> Self {
        Self {
            id: t.id,
            subject_id: t.subject_id,
            name: t.name,
            slug: t.slug,
            sort_order: t.sort_order,
            is_shared: t.owner_id.is_none(),
            owner_id: t.owner_id,
            is_active: t.is_active,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelResponse {
    pub code: QuestionLevel,
    pub name: &'static str,
}

/// Toàn bộ master data cho form ở frontend.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterDataResponse {
    pub subjects: Vec<SubjectResponse>,
    pub grades: Vec<GradeResponse>,
    pub levels: Vec<LevelResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingItemResponse {
    pub kind: String,
    pub id: Uuid,
    pub name: String,
    pub subject_id: Option<Uuid>,
    pub subject_name: Option<String>,
    pub owner_id: Uuid,
    pub owner_username: String,
    pub created_at: DateTime<Utc>,
}

impl From<PendingItem> for PendingItemResponse {
    fn from(p: PendingItem) -> Self {
        Self {
            kind: p.kind,
            id: p.id,
            name: p.name,
            subject_id: p.subject_id,
            subject_name: p.subject_name,
            owner_id: p.owner_id,
            owner_username: p.owner_username,
            created_at: p.created_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubjectRequest {
    #[validate(length(min = 1, max = 100, message = "Tên môn học 1–100 ký tự."))]
    pub name: String,
    pub sort_order: Option<i32>,
}

/// Thêm lớp riêng / danh mục.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateItemRequest {
    #[validate(length(min = 1, max = 60, message = "Tên 1–60 ký tự."))]
    pub name: String,
    pub sort_order: Option<i32>,
}

/// Admin sửa môn học / lớp / danh mục: chỉ gửi trường cần đổi.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateItemRequest {
    #[validate(length(min = 1, max = 100, message = "Tên 1–100 ký tự."))]
    pub name: Option<String>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MergeRequest {
    /// Mục dùng chung sẽ nhận các tham chiếu của mục bị gộp.
    pub into_id: Uuid,
}
