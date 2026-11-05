use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::model::{LinkRequest, LoginState, UserIdentity};

macro_rules! identity_cols {
    () => {
        "id, user_id, provider, subject, email, linked_at, last_login_at"
    };
}

pub async fn find_identity<'e, E: PgExecutor<'e>>(
    db: E,
    provider: &str,
    subject: &str,
) -> sqlx::Result<Option<UserIdentity>> {
    sqlx::query_as::<_, UserIdentity>(concat!(
        "SELECT ",
        identity_cols!(),
        " FROM user_identities WHERE provider = $1 AND subject = $2"
    ))
    .bind(provider)
    .bind(subject)
    .fetch_optional(db)
    .await
}

pub async fn list_for_user<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<Vec<UserIdentity>> {
    sqlx::query_as::<_, UserIdentity>(concat!(
        "SELECT ",
        identity_cols!(),
        " FROM user_identities WHERE user_id = $1 ORDER BY linked_at"
    ))
    .bind(user_id)
    .fetch_all(db)
    .await
}

pub async fn insert_identity<'e, E: PgExecutor<'e>>(
    db: E,
    user_id: Uuid,
    provider: &str,
    subject: &str,
    email: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO user_identities (user_id, provider, subject, email, last_login_at) VALUES ($1, $2, $3, $4, now())",
    )
    .bind(user_id)
    .bind(provider)
    .bind(subject)
    .bind(email)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn touch_identity<'e, E: PgExecutor<'e>>(db: E, id: Uuid, email: Option<&str>) -> sqlx::Result<()> {
    sqlx::query("UPDATE user_identities SET last_login_at = now(), email = COALESCE($2, email) WHERE id = $1")
        .bind(id)
        .bind(email)
        .execute(db)
        .await?;
    Ok(())
}

/// Gỡ liên kết. Trả `false` nếu không có liên kết nào để gỡ.
pub async fn delete_identity<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid, provider: &str) -> sqlx::Result<bool> {
    let res = sqlx::query("DELETE FROM user_identities WHERE user_id = $1 AND provider = $2")
        .bind(user_id)
        .bind(provider)
        .execute(db)
        .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn count_for_user<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM user_identities WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(db)
        .await
}

// ---------- Yêu cầu liên kết ----------

pub struct NewLinkRequest<'a> {
    pub token_hash: &'a str,
    pub user_id: Uuid,
    pub provider: &'a str,
    pub subject: &'a str,
    pub email: &'a str,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert_link_request<'e, E: PgExecutor<'e>>(db: E, r: &NewLinkRequest<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO sso_link_requests (token_hash, user_id, provider, subject, email, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(r.token_hash)
    .bind(r.user_id)
    .bind(r.provider)
    .bind(r.subject)
    .bind(r.email)
    .bind(r.expires_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn find_link_request<'e, E: PgExecutor<'e>>(db: E, token_hash: &str) -> sqlx::Result<Option<LinkRequest>> {
    sqlx::query_as::<_, LinkRequest>(
        "SELECT user_id, provider, subject, email, attempts, code_hash, code_sent_at, expires_at \
         FROM sso_link_requests WHERE token_hash = $1",
    )
    .bind(token_hash)
    .fetch_optional(db)
    .await
}

pub async fn set_link_code<'e, E: PgExecutor<'e>>(db: E, token_hash: &str, code_hash: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE sso_link_requests SET code_hash = $2, code_sent_at = now() WHERE token_hash = $1")
        .bind(token_hash)
        .bind(code_hash)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn increment_link_attempts<'e, E: PgExecutor<'e>>(db: E, token_hash: &str) -> sqlx::Result<i32> {
    sqlx::query_scalar::<_, i32>(
        "UPDATE sso_link_requests SET attempts = attempts + 1 WHERE token_hash = $1 RETURNING attempts",
    )
    .bind(token_hash)
    .fetch_one(db)
    .await
}

pub async fn delete_link_request<'e, E: PgExecutor<'e>>(db: E, token_hash: &str) -> sqlx::Result<bool> {
    let res = sqlx::query("DELETE FROM sso_link_requests WHERE token_hash = $1").bind(token_hash).execute(db).await?;
    Ok(res.rows_affected() == 1)
}

/// Dọn các yêu cầu đã hết hạn (gọi mỗi lần tạo yêu cầu mới, bảng luôn nhỏ).
pub async fn delete_expired_link_requests<'e, E: PgExecutor<'e>>(db: E) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sso_link_requests WHERE expires_at < now()").execute(db).await?;
    Ok(())
}

// ---------- Lượt đăng nhập SSO đang dở ----------

pub struct NewLoginState<'a> {
    pub state_hash: &'a str,
    pub provider: &'a str,
    pub nonce: &'a str,
    pub code_verifier: &'a str,
    pub link_user_id: Option<Uuid>,
    pub next_path: Option<&'a str>,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert_login_state<'e, E: PgExecutor<'e>>(db: E, s: &NewLoginState<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO sso_login_states (state_hash, provider, nonce, code_verifier, link_user_id, next_path, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(s.state_hash)
    .bind(s.provider)
    .bind(s.nonce)
    .bind(s.code_verifier)
    .bind(s.link_user_id)
    .bind(s.next_path)
    .bind(s.expires_at)
    .execute(db)
    .await?;
    Ok(())
}

/// Lấy và xóa luôn (dùng một lần): hai callback cùng `state` thì chỉ một callback nhận được.
pub async fn take_login_state<'e, E: PgExecutor<'e>>(db: E, state_hash: &str) -> sqlx::Result<Option<LoginState>> {
    sqlx::query_as::<_, LoginState>(
        "DELETE FROM sso_login_states WHERE state_hash = $1 \
         RETURNING provider, nonce, code_verifier, link_user_id, next_path, expires_at",
    )
    .bind(state_hash)
    .fetch_optional(db)
    .await
}

pub async fn delete_expired_login_states<'e, E: PgExecutor<'e>>(db: E) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sso_login_states WHERE expires_at < now()").execute(db).await?;
    Ok(())
}
