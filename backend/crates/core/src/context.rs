//! `AppContext`: mọi thứ service cần (DB, hàng đợi, giới hạn tần suất, khóa JWT, cấu hình).
//! Clone rẻ (bên trong là `Arc` / pool), được Axum dùng làm state.

use std::sync::Arc;

use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::{
    config::Config, jobs::JobQueue, modules::sso::oidc::SsoClients, rate_limit::RateLimiter, security::jwt::JwtKeys,
};

#[derive(Clone)]
pub struct AppContext {
    pub db: PgPool,
    pub jobs: JobQueue,
    pub limiter: RateLimiter,
    /// `None` trong test.
    pub valkey: Option<ConnectionManager>,
    pub jwt: Arc<JwtKeys>,
    pub config: Arc<Config>,
    /// Client OIDC của các nhà cung cấp SSO đã cấu hình.
    pub sso: Arc<SsoClients>,
}

impl AppContext {
    pub fn new(
        config: Config,
        db: PgPool,
        jobs: JobQueue,
        limiter: RateLimiter,
        valkey: Option<ConnectionManager>,
    ) -> Self {
        let jwt = Arc::new(JwtKeys::new(config.jwt_secret.as_bytes(), config.access_ttl));
        let sso = Arc::new(SsoClients::from_config(&config));
        Self { db, jobs, limiter, valkey, jwt, sso, config: Arc::new(config) }
    }

    /// Kiểm tra các phụ thuộc (dùng cho `GET /api/health`).
    pub async fn health(&self) -> HealthChecks {
        let database = sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&self.db).await.is_ok();
        let valkey = match &self.valkey {
            Some(conn) => {
                let mut conn = conn.clone();
                Some(redis::cmd("PING").query_async::<String>(&mut conn).await.is_ok())
            }
            None => None,
        };
        HealthChecks { database, valkey, queue: self.jobs.is_healthy() }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct HealthChecks {
    pub database: bool,
    /// `None` khi chạy không có Valkey (test).
    pub valkey: Option<bool>,
    pub queue: bool,
}

impl HealthChecks {
    pub fn all_ok(&self) -> bool {
        self.database && self.valkey.unwrap_or(true) && self.queue
    }
}
