use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "application_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ApplicationStatus {
    Pending,
    Approved,
    Rejected,
}

/// Read model: hồ sơ kèm thông tin người gửi (JOIN `users`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApplicationRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub email: String,
    pub subject_ids: Vec<Uuid>,
    pub grade_ids: Vec<Uuid>,
    pub bio: String,
    pub experience: String,
    pub status: ApplicationStatus,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reject_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}
