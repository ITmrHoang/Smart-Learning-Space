use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::model::{EmailVerification, NewRefreshToken, PasswordReset, RefreshToken};

// ---------- Mã OTP xác minh email ----------

/// Tạo hoặc thay mã OTP của user (đặt lại số lần nhập sai).
pub async fn upsert_verification<'e, E: PgExecutor<'e>>(
    db: E,
    user_id: Uuid,
    code_hash: &str,
    expires_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO email_verifications (user_id, code_hash, expires_at) VALUES ($1, $2, $3) \
         ON CONFLICT (user_id) DO UPDATE SET code_hash = EXCLUDED.code_hash, attempts = 0, \
         expires_at = EXCLUDED.expires_at, sent_at = now()",
    )
    .bind(user_id)
    .bind(code_hash)
    .bind(expires_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn find_verification<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<Option<EmailVerification>> {
    sqlx::query_as::<_, EmailVerification>(
        "SELECT user_id, code_hash, attempts, expires_at, sent_at FROM email_verifications WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
}

/// Tăng số lần nhập sai, trả về số lần sau khi tăng.
pub async fn increment_verification_attempts<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<i32> {
    sqlx::query_scalar::<_, i32>(
        "UPDATE email_verifications SET attempts = attempts + 1 WHERE user_id = $1 RETURNING attempts",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
}

pub async fn delete_verification<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM email_verifications WHERE user_id = $1").bind(user_id).execute(db).await?;
    Ok(())
}

// ---------- Refresh token ----------

pub async fn insert_refresh<'e, E: PgExecutor<'e>>(db: E, t: &NewRefreshToken<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO refresh_tokens (id, user_id, family_id, token_hash, expires_at, user_agent, ip) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(t.id)
    .bind(t.user_id)
    .bind(t.family_id)
    .bind(t.token_hash)
    .bind(t.expires_at)
    .bind(t.user_agent)
    .bind(t.ip)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn find_refresh_by_hash<'e, E: PgExecutor<'e>>(
    db: E,
    token_hash: &str,
) -> sqlx::Result<Option<RefreshToken>> {
    sqlx::query_as::<_, RefreshToken>(
        "SELECT id, user_id, family_id, expires_at, revoked_at, replaced_by, revoked_reason \
         FROM refresh_tokens WHERE token_hash = $1",
    )
    .bind(token_hash)
    .fetch_optional(db)
    .await
}

/// Thu hồi token `id` và ghi token thay thế. Trả `false` nếu token đã bị thu hồi trước đó
/// (hai request refresh chạy song song — chỉ một request thắng).
pub async fn claim_refresh<'e, E: PgExecutor<'e>>(db: E, id: Uuid, replaced_by: Uuid) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now(), replaced_by = $2 WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(id)
    .bind(replaced_by)
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn revoke_family<'e, E: PgExecutor<'e>>(db: E, family_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE family_id = $1 AND revoked_at IS NULL")
        .bind(family_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn revoke_all_for_user<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

/// Thu hồi mọi phiên của user trừ phiên `keep_family` (đổi mật khẩu: giữ thiết bị đang dùng).
pub async fn revoke_others_for_user<'e, E: PgExecutor<'e>>(
    db: E,
    user_id: Uuid,
    keep_family: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND family_id <> $2 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .bind(keep_family)
    .execute(db)
    .await?;
    Ok(())
}

/// Giữ lại `keep` phiên dùng gần nhất của user, thu hồi các phiên còn lại (lý do `session_limit`).
/// Mỗi phiên còn mở có đúng một refresh token chưa thu hồi, nên sắp theo token là sắp theo phiên.
pub async fn revoke_beyond_limit<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid, keep: i64) -> sqlx::Result<u64> {
    let res = sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = now(), revoked_reason = 'session_limit' \
         WHERE user_id = $1 AND revoked_at IS NULL AND id IN ( \
             SELECT id FROM refresh_tokens \
             WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now() \
             ORDER BY created_at DESC, id DESC OFFSET $2)",
    )
    .bind(user_id)
    .bind(keep)
    .execute(db)
    .await?;
    Ok(res.rows_affected())
}

pub async fn count_active_sessions<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(DISTINCT family_id) FROM refresh_tokens WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
}

// ---------- Đặt lại mật khẩu ----------

pub async fn insert_password_reset<'e, E: PgExecutor<'e>>(
    db: E,
    token_hash: &str,
    user_id: Uuid,
    expires_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO password_resets (token_hash, user_id, expires_at) VALUES ($1, $2, $3)")
        .bind(token_hash)
        .bind(user_id)
        .bind(expires_at)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn find_password_reset<'e, E: PgExecutor<'e>>(
    db: E,
    token_hash: &str,
) -> sqlx::Result<Option<PasswordReset>> {
    sqlx::query_as::<_, PasswordReset>("SELECT user_id, expires_at, used_at FROM password_resets WHERE token_hash = $1")
        .bind(token_hash)
        .fetch_optional(db)
        .await
}

/// Đánh dấu link đã dùng. `false` nếu link đã được dùng trước đó.
pub async fn mark_password_reset_used<'e, E: PgExecutor<'e>>(db: E, token_hash: &str) -> sqlx::Result<bool> {
    let res = sqlx::query("UPDATE password_resets SET used_at = now() WHERE token_hash = $1 AND used_at IS NULL")
        .bind(token_hash)
        .execute(db)
        .await?;
    Ok(res.rows_affected() == 1)
}

/// Vô hiệu hóa mọi link đặt lại chưa dùng của user.
pub async fn invalidate_password_resets<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE password_resets SET used_at = now() WHERE user_id = $1 AND used_at IS NULL")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}
