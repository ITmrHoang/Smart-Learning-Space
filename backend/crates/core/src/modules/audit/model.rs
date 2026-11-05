use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Read model: dòng `audit_logs` kèm username của người thực hiện (JOIN `users`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub data: serde_json::Value,
    pub ip: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Một thao tác cần ghi nhật ký.
pub struct AuditEntry<'a> {
    pub actor_id: Option<Uuid>,
    pub action: &'a str,
    pub target_type: &'a str,
    pub target_id: Option<Uuid>,
    pub data: serde_json::Value,
    pub ip: Option<&'a str>,
}
