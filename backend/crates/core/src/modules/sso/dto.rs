use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::model::{SsoProvider, UserIdentity};

/// Email SSO trùng một tài khoản đã có → cần chứng minh sở hữu trước khi liên kết.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkRequired {
    /// Token của yêu cầu liên kết (dùng một lần, hết hạn sau 15 phút).
    pub link_token: String,
    pub provider: SsoProvider,
    /// Email đã che bớt, ví dụ `n***@gmail.com`.
    pub email_hint: String,
    /// `false` khi tài khoản chưa có mật khẩu → chỉ xác minh được bằng mã OTP.
    pub can_use_password: bool,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LinkTokenRequest {
    #[validate(length(min = 20, max = 200, message = "Yêu cầu liên kết không hợp lệ."))]
    pub token: String,
}

/// Xác nhận liên kết bằng mật khẩu **hoặc** mã OTP.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmLinkRequest {
    #[validate(length(min = 20, max = 200, message = "Yêu cầu liên kết không hợp lệ."))]
    pub token: String,
    #[validate(length(min = 1, max = 128, message = "Nhập mật khẩu."))]
    pub password: Option<String>,
    #[validate(length(equal = 6, message = "Mã xác minh gồm 6 chữ số."))]
    pub code: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityResponse {
    pub provider: String,
    pub email: Option<String>,
    pub linked_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl From<UserIdentity> for IdentityResponse {
    fn from(i: UserIdentity) -> Self {
        Self { provider: i.provider, email: i.email, linked_at: i.linked_at, last_login_at: i.last_login_at }
    }
}
