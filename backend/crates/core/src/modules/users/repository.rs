//! Repository: chỉ chứa câu SQL, không chứa nghiệp vụ.
//! Nhận `PgExecutor` nên gọi được với pool hoặc trong transaction (`&mut *tx`).

use sqlx::PgExecutor;
use uuid::Uuid;

use super::model::{User, UserRole, UserStatus};

/// Danh sách cột của bảng `users` (sqlx 0.9 yêu cầu câu SQL là chuỗi tĩnh → dùng `concat!`).
macro_rules! user_cols {
    () => {
        "id, username, email, full_name, password_hash, role, status, email_verified_at, \
         last_login_at, created_at, updated_at"
    };
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub full_name: &'a str,
    pub password_hash: Option<&'a str>,
    pub role: UserRole,
    pub status: UserStatus,
    pub email_verified: bool,
}

pub struct UserFilter<'a> {
    /// Chuỗi tìm kiếm đã thoát ký tự `LIKE`.
    pub q: Option<&'a str>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
}

pub async fn find_by_id<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(concat!("SELECT ", user_cols!(), " FROM users WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Tìm theo username **hoặc** email, không phân biệt hoa thường.
pub async fn find_by_login<'e, E: PgExecutor<'e>>(db: E, login: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(concat!(
        "SELECT ",
        user_cols!(),
        " FROM users WHERE lower(username) = lower($1) OR lower(email) = lower($1) LIMIT 1"
    ))
    .bind(login)
    .fetch_optional(db)
    .await
}

pub async fn find_by_email<'e, E: PgExecutor<'e>>(db: E, email: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(concat!("SELECT ", user_cols!(), " FROM users WHERE lower(email) = lower($1)"))
        .bind(email)
        .fetch_optional(db)
        .await
}

pub async fn username_exists<'e, E: PgExecutor<'e>>(db: E, username: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE lower(username) = lower($1))")
        .bind(username)
        .fetch_one(db)
        .await
}

/// Email đã thuộc về user khác (`except` = user đang sửa, nếu có).
pub async fn email_taken<'e, E: PgExecutor<'e>>(db: E, email: &str, except: Option<Uuid>) -> sqlx::Result<bool> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM users WHERE lower(email) = lower($1) AND ($2::uuid IS NULL OR id <> $2))",
    )
    .bind(email)
    .bind(except)
    .fetch_one(db)
    .await
}

pub async fn insert<'e, E: PgExecutor<'e>>(db: E, new: &NewUser<'_>) -> sqlx::Result<User> {
    sqlx::query_as::<_, User>(concat!(
        "INSERT INTO users (username, email, full_name, password_hash, role, status, email_verified_at) \
         VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $7 THEN now() END) RETURNING ",
        user_cols!()
    ))
    .bind(new.username)
    .bind(new.email)
    .bind(new.full_name)
    .bind(new.password_hash)
    .bind(new.role)
    .bind(new.status)
    .bind(new.email_verified)
    .fetch_one(db)
    .await
}

/// Đánh dấu email đã xác minh; tài khoản đang chờ xác minh thì chuyển sang `active`.
pub async fn mark_email_verified<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE users SET email_verified_at = COALESCE(email_verified_at, now()), \
         status = CASE WHEN status = 'pending_verification' THEN 'active'::user_status ELSE status END, \
         updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_password<'e, E: PgExecutor<'e>>(db: E, id: Uuid, hash: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(hash)
        .execute(db)
        .await?;
    Ok(())
}

/// Xóa mật khẩu (tài khoản chỉ còn đăng nhập được bằng SSO hoặc đặt lại mật khẩu qua email).
/// Đặt mật khẩu khi tài khoản chưa có mật khẩu. Trả `false` nếu đã có.
pub async fn set_password_if_empty<'e, E: PgExecutor<'e>>(db: E, id: Uuid, hash: &str) -> sqlx::Result<bool> {
    let res =
        sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1 AND password_hash IS NULL")
            .bind(id)
            .bind(hash)
            .execute(db)
            .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn clear_password<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET password_hash = NULL, updated_at = now() WHERE id = $1").bind(id).execute(db).await?;
    Ok(())
}

pub async fn touch_login<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET last_login_at = now() WHERE id = $1").bind(id).execute(db).await?;
    Ok(())
}

pub async fn set_status<'e, E: PgExecutor<'e>>(db: E, id: Uuid, status: UserStatus) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET status = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(status)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn set_role<'e, E: PgExecutor<'e>>(db: E, id: Uuid, role: UserRole) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET role = $2, updated_at = now() WHERE id = $1").bind(id).bind(role).execute(db).await?;
    Ok(())
}

/// Cập nhật các trường được gửi (`None` = giữ nguyên).
pub async fn update<'e, E: PgExecutor<'e>>(
    db: E,
    id: Uuid,
    email: Option<&str>,
    full_name: Option<&str>,
    role: Option<UserRole>,
) -> sqlx::Result<User> {
    sqlx::query_as::<_, User>(concat!(
        "UPDATE users SET email = COALESCE($2, email), full_name = COALESCE($3, full_name), \
         role = COALESCE($4, role), updated_at = now() WHERE id = $1 RETURNING ",
        user_cols!()
    ))
    .bind(id)
    .bind(email)
    .bind(full_name)
    .bind(role)
    .fetch_one(db)
    .await
}

pub async fn list<'e, E: PgExecutor<'e>>(
    db: E,
    filter: &UserFilter<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<User>> {
    sqlx::query_as::<_, User>(concat!(
        "SELECT ",
        user_cols!(),
        " FROM users WHERE ($1::text IS NULL OR lower(username || ' ' || email || ' ' || full_name) \
         LIKE '%' || lower($1) || '%') AND ($2::user_role IS NULL OR role = $2) \
         AND ($3::user_status IS NULL OR status = $3) ORDER BY created_at DESC, id DESC LIMIT $4 OFFSET $5"
    ))
    .bind(filter.q)
    .bind(filter.role)
    .bind(filter.status)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
}

pub async fn count<'e, E: PgExecutor<'e>>(db: E, filter: &UserFilter<'_>) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM users WHERE ($1::text IS NULL OR lower(username || ' ' || email || ' ' || full_name) \
         LIKE '%' || lower($1) || '%') AND ($2::user_role IS NULL OR role = $2) \
         AND ($3::user_status IS NULL OR status = $3)",
    )
    .bind(filter.q)
    .bind(filter.role)
    .bind(filter.status)
    .fetch_one(db)
    .await
}

/// Số admin đang hoạt động (không khóa) — để không bao giờ mất admin cuối cùng.
pub async fn count_active_admins<'e, E: PgExecutor<'e>>(db: E) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users WHERE role = 'admin' AND status = 'active'")
        .fetch_one(db)
        .await
}

pub async fn admin_exists<'e, E: PgExecutor<'e>>(db: E) -> sqlx::Result<bool> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE role = 'admin')").fetch_one(db).await
}
