use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::model::AuditLogRow;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogResponse {
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

impl From<AuditLogRow> for AuditLogResponse {
    fn from(r: AuditLogRow) -> Self {
        Self {
            id: r.id,
            actor_id: r.actor_id,
            actor_username: r.actor_username,
            action: r.action,
            target_type: r.target_type,
            target_id: r.target_id,
            data: r.data,
            ip: r.ip,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditListQuery {
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
