use sqlx::PgExecutor;
use uuid::Uuid;

use super::model::{AuditEntry, AuditLogRow};

pub async fn insert<'e, E: PgExecutor<'e>>(db: E, e: &AuditEntry<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO audit_logs (actor_id, action, target_type, target_id, data, ip) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(e.actor_id)
    .bind(e.action)
    .bind(e.target_type)
    .bind(e.target_id)
    .bind(&e.data)
    .bind(e.ip)
    .execute(db)
    .await?;
    Ok(())
}

pub struct AuditFilter<'a> {
    pub action: Option<&'a str>,
    pub target_type: Option<&'a str>,
    pub target_id: Option<Uuid>,
}

pub async fn list<'e, E: PgExecutor<'e>>(
    db: E,
    f: &AuditFilter<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<AuditLogRow>> {
    sqlx::query_as::<_, AuditLogRow>(
        "SELECT a.id, a.actor_id, u.username AS actor_username, a.action, a.target_type, a.target_id, a.data, \
         a.ip, a.created_at FROM audit_logs a LEFT JOIN users u ON u.id = a.actor_id \
         WHERE ($1::text IS NULL OR a.action = $1) AND ($2::text IS NULL OR a.target_type = $2) \
         AND ($3::uuid IS NULL OR a.target_id = $3) ORDER BY a.created_at DESC, a.id DESC LIMIT $4 OFFSET $5",
    )
    .bind(f.action)
    .bind(f.target_type)
    .bind(f.target_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
}

pub async fn count<'e, E: PgExecutor<'e>>(db: E, f: &AuditFilter<'_>) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM audit_logs WHERE ($1::text IS NULL OR action = $1) \
         AND ($2::text IS NULL OR target_type = $2) AND ($3::uuid IS NULL OR target_id = $3)",
    )
    .bind(f.action)
    .bind(f.target_type)
    .bind(f.target_id)
    .fetch_one(db)
    .await
}
