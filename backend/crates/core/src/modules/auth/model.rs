use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailVerification {
    pub user_id: Uuid,
    pub code_hash: String,
    pub attempts: i32,
    pub expires_at: DateTime<Utc>,
    pub sent_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub replaced_by: Option<Uuid>,
    /// `session_limit`: bị đăng xuất vì tài khoản đăng nhập trên quá nhiều thiết bị.
    pub revoked_reason: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PasswordReset {
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// Một refresh token mới cần lưu.
pub struct NewRefreshToken<'a> {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub token_hash: &'a str,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<&'a str>,
    pub ip: Option<&'a str>,
}
