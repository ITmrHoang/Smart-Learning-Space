use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    Invited,
    Members,
    LoggedIn,
}

impl Access {
    pub fn as_str(self) -> &'static str {
        match self {
            Access::Invited => "invited",
            Access::Members => "members",
            Access::LoggedIn => "logged_in",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub schedule_id: Option<Uuid>,
    pub class_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
    pub access: String,
    pub meeting_url: Option<String>,
    pub remind_minutes: i32,
    pub ics_sequence: i32,
    pub class_title: Option<String>,
    pub owner_name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InviteeRow {
    pub target_type: String,
    pub target_id: Uuid,
    /// Tên lớp học / nhóm / học sinh.
    pub name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Recipient {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub email: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttendeeRow {
    pub user_id: Uuid,
    pub rsvp: Option<String>,
    pub joined_at: Option<DateTime<Utc>>,
    pub attended: bool,
}

/// Thông tin rút gọn của buổi (bước lộ trình).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRef {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub class_id: Option<Uuid>,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
}
