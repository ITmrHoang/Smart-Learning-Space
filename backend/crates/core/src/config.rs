//! Cấu hình đọc từ biến môi trường. Xem `backend/.env.example`.

use std::{fmt, time::Duration};

#[derive(Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub valkey_url: String,
    pub amqp_url: String,
    pub jwt_secret: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
    /// Địa chỉ frontend, dùng để tạo link trong email (đặt lại mật khẩu…).
    pub app_base_url: String,
    pub cors_origins: Vec<String>,
    /// `true` khi chạy HTTPS: cookie refresh có cờ `Secure`.
    pub cookie_secure: bool,
    /// `true` khi đứng sau Caddy: lấy IP người dùng từ `X-Forwarded-For`.
    pub trust_proxy: bool,
    pub bootstrap_admin: Option<BootstrapAdmin>,
}

/// Tài khoản admin đầu tiên, chỉ được tạo khi hệ thống chưa có admin nào.
#[derive(Clone)]
pub struct BootstrapAdmin {
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub password: String,
}

impl fmt::Debug for Config {
    // Viết tay để không bao giờ in ra secret / mật khẩu trong log.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("bind_addr", &self.bind_addr)
            .field("access_ttl", &self.access_ttl)
            .field("refresh_ttl", &self.refresh_ttl)
            .field("app_base_url", &self.app_base_url)
            .field("cors_origins", &self.cors_origins)
            .field("cookie_secure", &self.cookie_secure)
            .field("trust_proxy", &self.trust_proxy)
            .field("bootstrap_admin", &self.bootstrap_admin.as_ref().map(|a| &a.username))
            .finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConfigError {
    #[error("thiếu biến môi trường bắt buộc: {0}")]
    Missing(&'static str),
    #[error("giá trị không hợp lệ cho {name}: {reason}")]
    Invalid { name: &'static str, reason: String },
}

/// Nạp file `.env` (nếu có). File sai cú pháp thì cảnh báo rõ ràng thay vì im lặng:
/// dotenvy dừng ở dòng lỗi nên các biến phía sau sẽ không được nạp.
pub fn load_dotenv() {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(e) if e.not_found() => {}
        Err(e) => eprintln!(
            "CẢNH BÁO: file .env sai cú pháp ({e}). Giá trị có dấu cách phải đặt trong ngoặc kép, \
             ví dụ KEY=\"giá trị có dấu cách\"."
        ),
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok().filter(|v| !v.trim().is_empty()))
    }

    /// Tách riêng hàm đọc biến để test được mà không phải sửa biến môi trường thật.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let required = |name: &'static str| get(name).ok_or(ConfigError::Missing(name));
        let secs = |name: &'static str, default: u64| -> Result<Duration, ConfigError> {
            match get(name) {
                None => Ok(Duration::from_secs(default)),
                Some(v) => v
                    .parse::<u64>()
                    .map(Duration::from_secs)
                    .map_err(|e| ConfigError::Invalid { name, reason: e.to_string() }),
            }
        };
        let flag = |name: &'static str, default: bool| {
            get(name).map(|v| matches!(v.as_str(), "1" | "true" | "yes")).unwrap_or(default)
        };

        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err(ConfigError::Invalid { name: "JWT_SECRET", reason: "cần ít nhất 32 ký tự".into() });
        }

        let bootstrap_admin = match (get("BOOTSTRAP_ADMIN_EMAIL"), get("BOOTSTRAP_ADMIN_PASSWORD")) {
            (Some(email), Some(password)) => Some(BootstrapAdmin {
                username: get("BOOTSTRAP_ADMIN_USERNAME").unwrap_or_else(|| "admin".into()),
                full_name: get("BOOTSTRAP_ADMIN_FULL_NAME").unwrap_or_else(|| "Quản trị viên".into()),
                email,
                password,
            }),
            _ => None,
        };

        Ok(Self {
            bind_addr: get("BIND_ADDR").unwrap_or_else(|| "127.0.0.1:8080".into()),
            database_url: required("DATABASE_URL")?,
            valkey_url: get("VALKEY_URL").unwrap_or_else(|| "redis://127.0.0.1:6379".into()),
            amqp_url: get("AMQP_URL").unwrap_or_else(|| "amqp://127.0.0.1:5672/%2f".into()),
            jwt_secret,
            access_ttl: secs("ACCESS_TOKEN_TTL_SECS", 15 * 60)?,
            refresh_ttl: secs("REFRESH_TOKEN_TTL_SECS", 30 * 24 * 3600)?,
            app_base_url: get("APP_BASE_URL")
                .unwrap_or_else(|| "http://127.0.0.1:5173".into())
                .trim_end_matches('/')
                .to_string(),
            cors_origins: get("CORS_ORIGINS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default(),
            cookie_secure: flag("COOKIE_SECURE", false),
            trust_proxy: flag("TRUST_PROXY", false),
            bootstrap_admin,
        })
    }

    /// Cấu hình cố định dùng trong test.
    pub fn for_tests(database_url: &str) -> Self {
        Self::from_lookup(|key| match key {
            "DATABASE_URL" => Some(database_url.to_string()),
            "JWT_SECRET" => Some("test-secret-test-secret-test-secret-0123".into()),
            _ => None,
        })
        .expect("cấu hình test hợp lệ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string())
    }

    #[test]
    fn thieu_bien_bat_buoc_thi_bao_ten_bien() {
        let err = Config::from_lookup(lookup(&[("DATABASE_URL", "postgres://x")])).unwrap_err();
        assert_eq!(err, ConfigError::Missing("JWT_SECRET"));
    }

    #[test]
    fn jwt_secret_qua_ngan_bi_tu_choi() {
        let err = Config::from_lookup(lookup(&[("DATABASE_URL", "postgres://x"), ("JWT_SECRET", "ngan")])).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { name: "JWT_SECRET", .. }));
    }

    #[test]
    fn gia_tri_mac_dinh_va_danh_sach_cors() {
        let cfg = Config::from_lookup(lookup(&[
            ("DATABASE_URL", "postgres://x"),
            ("JWT_SECRET", "0123456789abcdef0123456789abcdef"),
            ("CORS_ORIGINS", "http://a.test, http://b.test"),
            ("APP_BASE_URL", "http://app.test/"),
        ]))
        .unwrap();
        assert_eq!(cfg.access_ttl, Duration::from_secs(900));
        assert_eq!(cfg.cors_origins, vec!["http://a.test", "http://b.test"]);
        assert_eq!(cfg.app_base_url, "http://app.test");
        assert!(cfg.bootstrap_admin.is_none());
        assert!(!cfg.cookie_secure);
    }

    #[test]
    fn debug_khong_in_secret() {
        let cfg = Config::for_tests("postgres://x");
        let text = format!("{cfg:?}");
        assert!(!text.contains("test-secret"));
    }
}
