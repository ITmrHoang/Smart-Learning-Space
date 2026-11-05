use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    modules::users::{
        dto::{UserResponse, validate_password, validate_username},
        model::User,
    },
    security::permissions::{Permission, permissions_for},
};

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(custom(function = "validate_username"))]
    pub username: String,
    #[validate(email(message = "Email không hợp lệ."))]
    pub email: String,
    #[validate(length(min = 2, max = 100, message = "Họ tên 2–100 ký tự."))]
    pub full_name: String,
    #[validate(custom(function = "validate_password"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailRequest {
    #[validate(email(message = "Email không hợp lệ."))]
    pub email: String,
    #[validate(length(equal = 6, message = "Mã xác minh gồm 6 chữ số."))]
    pub code: String,
}

/// Dùng cho "gửi lại mã" và "quên mật khẩu".
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EmailRequest {
    #[validate(email(message = "Email không hợp lệ."))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    /// Username hoặc email.
    #[validate(length(min = 1, max = 255, message = "Nhập username hoặc email."))]
    pub login: String,
    #[validate(length(min = 1, max = 128, message = "Nhập mật khẩu."))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    #[validate(length(min = 20, max = 200, message = "Link đặt lại mật khẩu không hợp lệ."))]
    pub token: String,
    #[validate(custom(function = "validate_password"))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, message = "Nhập mật khẩu hiện tại."))]
    pub current_password: String,
    #[validate(custom(function = "validate_password"))]
    pub new_password: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SetPasswordRequest {
    #[validate(custom(function = "validate_password"))]
    pub new_password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub email: String,
    pub verification_required: bool,
}

/// Người dùng hiện tại + danh sách quyền (frontend dùng để ẩn / hiện menu).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    #[serde(flatten)]
    pub user: UserResponse,
    /// `false` với tài khoản tạo từ SSO chưa đặt mật khẩu.
    pub has_password: bool,
    pub permissions: Vec<Permission>,
}

impl From<User> for MeResponse {
    fn from(u: User) -> Self {
        let permissions = permissions_for(u.role).to_vec();
        let has_password = u.password_hash.is_some();
        Self { user: u.into(), has_password, permissions }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: &'static str,
    /// Số giây access token còn hiệu lực.
    pub expires_in: i64,
    pub user: MeResponse,
}
