//! Giới hạn tần suất (chống dò mật khẩu, spam đăng ký / email).
//!
//! - `Valkey`: bộ đếm dùng chung giữa nhiều instance `api`.
//! - `Memory`: bộ đếm trong RAM, dùng trong test.
//! - `Disabled`: tắt hẳn (test không liên quan tới giới hạn).
//!
//! Valkey lỗi thì **cho qua** (fail-open) và ghi log, để sự cố cache không chặn đăng nhập.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use redis::aio::ConnectionManager;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub enum RateLimiter {
    Valkey(ConnectionManager),
    Memory(Arc<Mutex<HashMap<String, (u32, Instant)>>>),
    Disabled,
}

impl RateLimiter {
    pub fn memory() -> Self {
        Self::Memory(Arc::new(Mutex::new(HashMap::new())))
    }

    /// Báo lỗi 429 nếu `key` đã đạt `max` lần trong cửa sổ hiện tại. Không tăng bộ đếm.
    pub async fn check(&self, key: &str, max: u32) -> AppResult<()> {
        match self {
            Self::Disabled => Ok(()),
            Self::Memory(map) => {
                let map = map.lock().expect("mutex");
                match map.get(key) {
                    Some((count, until)) if *count >= max && *until > Instant::now() => {
                        Err(AppError::too_many_requests(until.duration_since(Instant::now()).as_secs().max(1)))
                    }
                    _ => Ok(()),
                }
            }
            Self::Valkey(conn) => {
                let mut conn = conn.clone();
                let key = format!("rl:{key}");
                let res: redis::RedisResult<(Option<u32>, i64)> =
                    redis::pipe().get(&key).ttl(&key).query_async(&mut conn).await;
                match res {
                    Ok((Some(count), ttl)) if count >= max => Err(AppError::too_many_requests(ttl.max(1) as u64)),
                    Ok(_) => Ok(()),
                    Err(e) => {
                        tracing::warn!(error = %e, "rate limit: Valkey lỗi, cho qua");
                        Ok(())
                    }
                }
            }
        }
    }

    /// Tăng bộ đếm của `key`; cửa sổ `window` bắt đầu từ lần đầu tiên.
    pub async fn hit(&self, key: &str, window: Duration) {
        match self {
            Self::Disabled => {}
            Self::Memory(map) => {
                let mut map = map.lock().expect("mutex");
                let now = Instant::now();
                let entry = map.entry(key.to_string()).or_insert((0, now + window));
                if entry.1 <= now {
                    *entry = (0, now + window);
                }
                entry.0 += 1;
            }
            Self::Valkey(conn) => {
                let mut conn = conn.clone();
                let key = format!("rl:{key}");
                let res: redis::RedisResult<()> = redis::pipe()
                    .cmd("SET")
                    .arg(&key)
                    .arg(0)
                    .arg("EX")
                    .arg(window.as_secs().max(1))
                    .arg("NX")
                    .ignore()
                    .incr(&key, 1)
                    .ignore()
                    .query_async(&mut conn)
                    .await;
                if let Err(e) = res {
                    tracing::warn!(error = %e, "rate limit: Valkey lỗi khi tăng bộ đếm");
                }
            }
        }
    }

    /// Tăng bộ đếm rồi kiểm tra luôn (dùng cho thao tác mà mỗi lần gọi đều tính).
    pub async fn hit_and_check(&self, key: &str, max: u32, window: Duration) -> AppResult<()> {
        self.check(key, max).await?;
        self.hit(key, window).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bo_nho_chan_sau_khi_dat_gioi_han() {
        let rl = RateLimiter::memory();
        for _ in 0..3 {
            rl.check("k", 3).await.unwrap();
            rl.hit("k", Duration::from_secs(60)).await;
        }
        let err = rl.check("k", 3).await.unwrap_err();
        assert_eq!(err.code, "RATE_LIMITED");
        // key khác không bị ảnh hưởng
        rl.check("k2", 3).await.unwrap();
    }

    #[tokio::test]
    async fn het_cua_so_thi_dem_lai() {
        let rl = RateLimiter::memory();
        rl.hit("k", Duration::from_millis(10)).await;
        rl.hit("k", Duration::from_millis(10)).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        rl.hit("k", Duration::from_secs(60)).await;
        rl.check("k", 2).await.unwrap();
    }
}
