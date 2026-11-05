use sqlx::PgExecutor;
use uuid::Uuid;

use super::model::{ApplicationRow, ApplicationStatus};

macro_rules! app_select {
    () => {
        "SELECT a.id, a.user_id, u.username, u.full_name, u.email, a.subject_ids, a.grade_ids, a.bio, \
         a.experience, a.status, a.reviewed_by, a.reviewed_at, a.reject_reason, a.created_at \
         FROM teacher_applications a JOIN users u ON u.id = a.user_id "
    };
}

pub struct NewApplication<'a> {
    pub user_id: Uuid,
    pub subject_ids: &'a [Uuid],
    pub grade_ids: &'a [Uuid],
    pub bio: &'a str,
    pub experience: &'a str,
}

pub async fn insert<'e, E: PgExecutor<'e>>(db: E, a: &NewApplication<'_>) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO teacher_applications (user_id, subject_ids, grade_ids, bio, experience) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(a.user_id)
    .bind(a.subject_ids)
    .bind(a.grade_ids)
    .bind(a.bio)
    .bind(a.experience)
    .fetch_one(db)
    .await
}

pub async fn find<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<ApplicationRow>> {
    sqlx::query_as::<_, ApplicationRow>(concat!(app_select!(), "WHERE a.id = $1")).bind(id).fetch_optional(db).await
}

/// Khóa dòng hồ sơ trong transaction để hai admin không cùng duyệt một hồ sơ.
pub async fn lock_status<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<ApplicationStatus>> {
    sqlx::query_scalar::<_, ApplicationStatus>("SELECT status FROM teacher_applications WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn list_for_user<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<Vec<ApplicationRow>> {
    sqlx::query_as::<_, ApplicationRow>(concat!(app_select!(), "WHERE a.user_id = $1 ORDER BY a.created_at DESC"))
        .bind(user_id)
        .fetch_all(db)
        .await
}

pub async fn list<'e, E: PgExecutor<'e>>(
    db: E,
    status: Option<ApplicationStatus>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<ApplicationRow>> {
    sqlx::query_as::<_, ApplicationRow>(concat!(
        app_select!(),
        "WHERE ($1::application_status IS NULL OR a.status = $1) ORDER BY a.created_at, a.id LIMIT $2 OFFSET $3"
    ))
    .bind(status)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
}

pub async fn count<'e, E: PgExecutor<'e>>(db: E, status: Option<ApplicationStatus>) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM teacher_applications WHERE ($1::application_status IS NULL OR status = $1)",
    )
    .bind(status)
    .fetch_one(db)
    .await
}

pub async fn set_reviewed<'e, E: PgExecutor<'e>>(
    db: E,
    id: Uuid,
    status: ApplicationStatus,
    reviewer: Uuid,
    reason: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE teacher_applications SET status = $2, reviewed_by = $3, reviewed_at = now(), reject_reason = $4 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(reviewer)
    .bind(reason)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn add_teacher_subjects<'e, E: PgExecutor<'e>>(
    db: E,
    teacher_id: Uuid,
    subject_ids: &[Uuid],
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO teacher_subjects (teacher_id, subject_id) SELECT $1, unnest($2::uuid[]) ON CONFLICT DO NOTHING",
    )
    .bind(teacher_id)
    .bind(subject_ids)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn teacher_subject_ids<'e, E: PgExecutor<'e>>(db: E, teacher_id: Uuid) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar::<_, Uuid>("SELECT subject_id FROM teacher_subjects WHERE teacher_id = $1")
        .bind(teacher_id)
        .fetch_all(db)
        .await
}
