//! Kết nối PostgreSQL và migration.

use std::time::Duration;

use sqlx::{PgPool, migrate::Migrator, postgres::PgPoolOptions};

/// Migration nhúng sẵn trong binary, tự chạy khi `api` khởi động.
pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().max_connections(10).acquire_timeout(Duration::from_secs(5)).connect(database_url).await
}

/// `true` nếu lỗi là vi phạm ràng buộc UNIQUE có tên `constraint`.
pub fn is_unique_violation(err: &sqlx::Error, constraint: &str) -> bool {
    match err {
        sqlx::Error::Database(db) => db.code().as_deref() == Some("23505") && db.constraint() == Some(constraint),
        _ => false,
    }
}

/// Thoát các ký tự đặc biệt của `LIKE` (`%`, `_`, `\`) trong chuỗi người dùng nhập.
pub fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// SQL ghép từ các hằng số trong code (danh sách cột…), **không** chứa dữ liệu người dùng — dữ liệu luôn đi qua `bind`.
/// sqlx 0.9 bắt đánh dấu rõ những chuỗi SQL dựng động như vậy.
#[macro_export]
macro_rules! sql {
    ($($arg:tt)*) => {
        ::sqlx::AssertSqlSafe(format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn escape_like_thoat_ky_tu_dac_biet() {
        assert_eq!(super::escape_like("50%_a\\b"), "50\\%\\_a\\\\b");
    }
}
