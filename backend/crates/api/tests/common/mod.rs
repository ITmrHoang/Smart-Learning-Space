//! Hỗ trợ integration test: dựng router với database riêng của từng test (`#[sqlx::test]`),
//! hàng đợi bộ nhớ (đọc lại email), không có Valkey.
#![allow(dead_code)]

use std::sync::LazyLock;

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use serde_json::{Value, json};
use sls_api::app::build_router;
use sls_core::{
    AppContext,
    config::Config,
    jobs::{Email, Job, JobQueue},
    modules::users::{
        model::{UserRole, UserStatus},
        repository::NewUser,
        service as users,
    },
    rate_limit::RateLimiter,
    security::password::hash_password_sync,
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// Mật khẩu dùng chung cho user tạo trong test.
pub const PASSWORD: &str = "MatKhau123";
static PASSWORD_HASH: LazyLock<String> = LazyLock::new(|| hash_password_sync(PASSWORD).unwrap());

pub struct Res {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Value,
}

impl Res {
    /// Mã lỗi (`error.code`), chuỗi rỗng nếu không phải lỗi.
    pub fn code(&self) -> &str {
        self.body["error"]["code"].as_str().unwrap_or("")
    }

    /// Giá trị cookie `sls_refresh` trong `Set-Cookie` (chuỗi rỗng = cookie bị xóa).
    pub fn refresh_cookie(&self) -> Option<String> {
        self.set_cookie_header()
            .map(|s| s.trim_start_matches("sls_refresh=").split(';').next().unwrap_or("").to_string())
    }

    pub fn set_cookie_header(&self) -> Option<String> {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|s| s.starts_with("sls_refresh="))
            .map(str::to_string)
    }

    pub fn token(&self) -> String {
        self.body["accessToken"].as_str().expect("có accessToken").to_string()
    }
}

pub struct TestApp {
    pub router: Router,
    pub ctx: AppContext,
}

impl TestApp {
    pub fn new(pool: PgPool) -> Self {
        Self::with_limiter(pool, RateLimiter::Disabled)
    }

    pub fn with_limiter(pool: PgPool, limiter: RateLimiter) -> Self {
        let ctx = AppContext::new(Config::for_tests("postgres://test"), pool, JobQueue::memory(), limiter, None);
        Self { router: build_router(ctx.clone()), ctx }
    }

    pub async fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        token: Option<&str>,
        refresh_cookie: Option<&str>,
    ) -> Res {
        let mut builder = Request::builder().method(method).uri(path).header(header::USER_AGENT, "sls-test");
        if let Some(t) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }
        if let Some(c) = refresh_cookie {
            builder = builder.header(header::COOKIE, format!("sls_refresh={c}"));
        }
        let req = match body {
            Some(v) => builder.header(header::CONTENT_TYPE, "application/json").body(Body::from(v.to_string())),
            None => builder.body(Body::empty()),
        }
        .unwrap();
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()))
        };
        Res { status, headers, body }
    }

    pub async fn get(&self, path: &str, token: Option<&str>) -> Res {
        self.send("GET", path, None, token, None).await
    }

    pub async fn post(&self, path: &str, body: Value, token: Option<&str>) -> Res {
        self.send("POST", path, Some(body), token, None).await
    }

    pub async fn patch(&self, path: &str, body: Value, token: Option<&str>) -> Res {
        self.send("PATCH", path, Some(body), token, None).await
    }

    pub async fn refresh(&self, cookie: &str) -> Res {
        self.send("POST", "/api/auth/refresh", None, None, Some(cookie)).await
    }

    /// Lấy ra các email đã đẩy vào hàng đợi (và xóa khỏi hàng đợi).
    pub fn emails(&self) -> Vec<Email> {
        self.ctx
            .jobs
            .drain_memory()
            .into_iter()
            .map(|job| match job {
                Job::SendEmail(e) => e,
            })
            .collect()
    }

    pub async fn register(&self, username: &str, email: &str) -> Res {
        self.post(
            "/api/auth/register",
            json!({ "username": username, "email": email, "fullName": "Nguyễn Văn Test", "password": PASSWORD }),
            None,
        )
        .await
    }

    /// Đăng ký + nhập OTP → đã đăng nhập. Trả về response của `verify-email`.
    pub async fn register_verified(&self, username: &str) -> Res {
        let email = format!("{username}@example.com");
        assert_eq!(self.register(username, &email).await.status, StatusCode::CREATED);
        let code = otp_from(self.emails().last().expect("có email OTP"));
        let res = self.post("/api/auth/verify-email", json!({ "email": email, "code": code }), None).await;
        assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
        res
    }

    pub async fn login(&self, login: &str, password: &str) -> Res {
        self.post("/api/auth/login", json!({ "login": login, "password": password }), None).await
    }

    /// Tạo user đã kích hoạt (không qua API) với mật khẩu `PASSWORD`.
    pub async fn create_user(&self, username: &str, role: UserRole) -> Uuid {
        users::create(
            &self.ctx.db,
            &NewUser {
                username,
                email: &format!("{username}@example.com"),
                full_name: &format!("User {username}"),
                password_hash: Some(&PASSWORD_HASH),
                role,
                status: UserStatus::Active,
                email_verified: true,
            },
        )
        .await
        .unwrap()
        .id
    }

    /// Tạo user rồi đăng nhập. Trả về `(id, access token)`.
    pub async fn login_as(&self, username: &str, role: UserRole) -> (Uuid, String) {
        let id = self.create_user(username, role).await;
        let res = self.login(username, PASSWORD).await;
        assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
        (id, res.token())
    }

    pub async fn subject_id(&self, slug: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM subjects WHERE slug = $1")
            .bind(slug.to_string())
            .fetch_one(&self.ctx.db)
            .await
            .unwrap()
    }
}

/// Đọc mã OTP 6 số trong email xác minh.
pub fn otp_from(email: &Email) -> String {
    let start = email.text.find("là: ").expect("email có mã") + "là: ".len();
    email.text[start..start + 6].to_string()
}

/// Đọc token trong link đặt lại mật khẩu.
pub fn reset_token_from(email: &Email) -> String {
    let start = email.text.find("token=").expect("email có link") + "token=".len();
    email.text[start..].split_whitespace().next().unwrap().to_string()
}
