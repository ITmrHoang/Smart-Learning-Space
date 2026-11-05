//! Băm mật khẩu bằng Argon2id (tham số mặc định của crate `argon2`).
//!
//! Argon2 cố ý tốn CPU, nên chạy trong `spawn_blocking` để không chặn luồng async.

use std::sync::LazyLock;

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

use crate::error::{AppError, AppResult};

/// Hash giả để so khi user không tồn tại → thời gian phản hồi như nhau,
/// kẻ tấn công không đoán được username nào có thật.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password_sync("mat-khau-gia-de-can-thoi-gian").expect("hash giả"));

pub fn hash_password_sync(password: &str) -> AppResult<String> {
    Argon2::default().hash_password(password.as_bytes()).map(|h| h.to_string()).map_err(AppError::internal)
}

pub fn verify_password_sync(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok(),
        Err(_) => false,
    }
}

pub async fn hash_password(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || hash_password_sync(&password)).await.map_err(AppError::internal)?
}

/// So mật khẩu với hash; `hash = None` (user không tồn tại / chưa đặt mật khẩu) thì so với hash giả
/// và luôn trả `false`.
pub async fn verify_password(password: String, hash: Option<String>) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || match hash {
        Some(h) => verify_password_sync(&password, &h),
        None => {
            let _ = verify_password_sync(&password, &DUMMY_HASH);
            false
        }
    })
    .await
    .map_err(AppError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bam_va_kiem_tra_mat_khau() {
        let hash = hash_password_sync("MatKhau@123").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password_sync("MatKhau@123", &hash));
        assert!(!verify_password_sync("matkhau@123", &hash));
        assert!(!verify_password_sync("MatKhau@123", "khong-phai-hash"));
    }

    #[tokio::test]
    async fn user_khong_ton_tai_luon_sai() {
        assert!(!verify_password("bat-ky".into(), None).await.unwrap());
    }
}
