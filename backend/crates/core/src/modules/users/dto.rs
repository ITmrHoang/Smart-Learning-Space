//! DTO của module users: request (validate) và response (chỉ trường được phép lộ ra).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::model::{User, UserRole, UserStatus};

/// Thông tin user trả về client (không có `password_hash`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub role: UserRole,
    pub status: UserStatus,
    pub email_verified: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            email: u.email,
            full_name: u.full_name,
            role: u.role,
            status: u.status,
            email_verified: u.email_verified_at.is_some(),
            last_login_at: u.last_login_at,
            created_at: u.created_at,
        }
    }
}

/// Username: 3–32 ký tự, chữ thường không dấu, số, dấu chấm, gạch dưới; bắt đầu bằng chữ.
pub fn validate_username(value: &str) -> Result<(), ValidationError> {
    let ok = value.len() >= 3
        && value.len() <= 32
        && value.starts_with(|c: char| c.is_ascii_lowercase())
        && value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(ValidationError::new("username").with_message(
            "Username 3–32 ký tự, bắt đầu bằng chữ cái; chỉ gồm chữ thường không dấu, số, dấu chấm, gạch dưới.".into(),
        ))
    }
}

/// Mật khẩu: 8–128 ký tự, có cả chữ và số.
pub fn validate_password(value: &str) -> Result<(), ValidationError> {
    let len = value.chars().count();
    let ok = (8..=128).contains(&len)
        && value.chars().any(|c| c.is_alphabetic())
        && value.chars().any(|c| c.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err(ValidationError::new("password")
            .with_message("Mật khẩu 8–128 ký tự, có ít nhất một chữ cái và một chữ số.".into()))
    }
}

/// Admin tạo tài khoản (đã kích hoạt sẵn).
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    #[validate(custom(function = "validate_username"))]
    pub username: String,
    #[validate(email(message = "Email không hợp lệ."))]
    pub email: String,
    #[validate(length(min = 2, max = 100, message = "Họ tên 2–100 ký tự."))]
    pub full_name: String,
    #[validate(custom(function = "validate_password"))]
    pub password: String,
    pub role: UserRole,
}

/// Admin sửa tài khoản: chỉ gửi trường cần đổi.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    #[validate(email(message = "Email không hợp lệ."))]
    pub email: Option<String>,
    #[validate(length(min = 2, max = 100, message = "Họ tên 2–100 ký tự."))]
    pub full_name: Option<String>,
    pub role: Option<UserRole>,
}

/// Bộ lọc danh sách user (query string): `?q=&role=&status=&page=&pageSize=`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserListQuery {
    pub q: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kiem_tra_username() {
        for ok in ["abc", "nguyen.van_a", "hs2026"] {
            assert!(validate_username(ok).is_ok(), "{ok}");
        }
        for bad in ["ab", "1abc", "Nguyen", "nguyễn", "a b c", &"a".repeat(33)] {
            assert!(validate_username(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn kiem_tra_mat_khau() {
        assert!(validate_password("matkhau123").is_ok());
        assert!(validate_password("ngắn1").is_err());
        assert!(validate_password("chicochu").is_err());
        assert!(validate_password("12345678").is_err());
    }
}
