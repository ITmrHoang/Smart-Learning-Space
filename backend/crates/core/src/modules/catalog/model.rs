use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Subject {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Grade {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub sort_order: i32,
    pub is_system: bool,
    /// `None` = dùng chung; `Some` = lớp riêng của giáo viên.
    pub owner_id: Option<Uuid>,
    pub merged_into: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Tag {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub name: String,
    pub slug: String,
    pub sort_order: i32,
    pub owner_id: Option<Uuid>,
    pub merged_into: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Read model: mục do giáo viên tự thêm, chờ admin xử lý.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingItem {
    /// `grade` hoặc `tag`.
    pub kind: String,
    pub id: Uuid,
    pub name: String,
    pub subject_id: Option<Uuid>,
    pub subject_name: Option<String>,
    pub owner_id: Uuid,
    pub owner_username: String,
    pub created_at: DateTime<Utc>,
}

/// Mức độ câu hỏi (cố định theo chương trình GDPT 2018).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionLevel {
    NhanBiet,
    ThongHieu,
    VanDung,
    VanDungCao,
}

impl QuestionLevel {
    pub const ALL: [QuestionLevel; 4] =
        [QuestionLevel::NhanBiet, QuestionLevel::ThongHieu, QuestionLevel::VanDung, QuestionLevel::VanDungCao];

    pub fn label(self) -> &'static str {
        match self {
            QuestionLevel::NhanBiet => "Nhận biết",
            QuestionLevel::ThongHieu => "Thông hiểu",
            QuestionLevel::VanDung => "Vận dụng",
            QuestionLevel::VanDungCao => "Vận dụng cao",
        }
    }
}
