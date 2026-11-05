use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoProvider {
    Google,
    Microsoft,
}

impl SsoProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            SsoProvider::Google => "google",
            SsoProvider::Microsoft => "microsoft",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SsoProvider::Google => "Google",
            SsoProvider::Microsoft => "Microsoft",
        }
    }
}

impl fmt::Display for SsoProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SsoProvider {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "google" => Ok(SsoProvider::Google),
            "microsoft" => Ok(SsoProvider::Microsoft),
            _ => Err(()),
        }
    }
}

/// Danh tính mà nhà cung cấp đã xác thực (đọc từ id_token sau khi kiểm tra chữ ký, iss, aud, nonce).
#[derive(Debug, Clone)]
pub struct ProviderIdentity {
    pub provider: SsoProvider,
    /// Claim `sub`: mã cố định của người dùng ở nhà cung cấp.
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserIdentity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub subject: String,
    pub email: Option<String>,
    pub linked_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LinkRequest {
    pub user_id: Uuid,
    pub provider: String,
    pub subject: String,
    pub email: String,
    pub attempts: i32,
    pub code_hash: Option<String>,
    pub code_sent_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

/// Lượt đăng nhập SSO đang dở (bảng `sso_login_states`).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LoginState {
    pub provider: String,
    pub nonce: String,
    pub code_verifier: String,
    /// Có giá trị khi người dùng đã đăng nhập và đang liên kết thêm tài khoản Google / Microsoft.
    pub link_user_id: Option<Uuid>,
    pub next_path: Option<String>,
    pub expires_at: DateTime<Utc>,
}
