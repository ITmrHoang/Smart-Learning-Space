use sqlx::{PgExecutor, Postgres, QueryBuilder};
use uuid::Uuid;

use super::model::{Lesson, LessonRef, LessonRevision};

const LESSON_COLS: &str = "l.id, l.owner_id, l.subject_id, l.grade_id, l.title, l.summary, l.content, l.video_url, \
     l.duration_minutes, l.published_revision_id, l.has_changes, l.archived_at, l.created_at, l.updated_at, \
     array(SELECT tag_id FROM lesson_tags t WHERE t.lesson_id = l.id) AS tag_ids, \
     (SELECT version FROM lesson_revisions r WHERE r.id = l.published_revision_id) AS published_version";

pub struct LessonFields<'a> {
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: &'a str,
    pub summary: &'a str,
    pub content: &'a str,
    pub video_url: Option<&'a str>,
    pub duration_minutes: Option<i32>,
}

pub struct LessonFilter<'a> {
    pub owner_id: Uuid,
    pub q: Option<&'a str>,
    pub subject_id: Option<Uuid>,
    pub status: Option<&'a str>,
}

fn push_filter(qb: &mut QueryBuilder<Postgres>, f: &LessonFilter<'_>) {
    qb.push(" WHERE l.archived_at IS NULL AND l.owner_id = ").push_bind(f.owner_id);
    if let Some(q) = f.q {
        qb.push(" AND unaccent(lower(l.title)) LIKE unaccent(lower(")
            .push_bind(format!("%{}%", crate::db::escape_like(q)))
            .push("))");
    }
    if let Some(s) = f.subject_id {
        qb.push(" AND l.subject_id = ").push_bind(s);
    }
    match f.status {
        Some("draft") => {
            qb.push(" AND l.published_revision_id IS NULL");
        }
        Some("published") => {
            qb.push(" AND l.published_revision_id IS NOT NULL");
        }
        Some("changed") => {
            qb.push(" AND l.published_revision_id IS NOT NULL AND l.has_changes");
        }
        _ => {}
    }
}

pub async fn list<'e, E: PgExecutor<'e>>(
    db: E,
    f: &LessonFilter<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<Lesson>> {
    let mut qb = QueryBuilder::new(format!("SELECT {LESSON_COLS} FROM lessons l"));
    push_filter(&mut qb, f);
    qb.push(" ORDER BY l.updated_at DESC LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);
    qb.build_query_as::<Lesson>().fetch_all(db).await
}

pub async fn count<'e, E: PgExecutor<'e>>(db: E, f: &LessonFilter<'_>) -> sqlx::Result<i64> {
    let mut qb = QueryBuilder::new("SELECT count(*) FROM lessons l");
    push_filter(&mut qb, f);
    qb.build_query_scalar::<i64>().fetch_one(db).await
}

pub async fn find<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<Lesson>> {
    sqlx::query_as::<_, Lesson>(crate::sql!("SELECT {LESSON_COLS} FROM lessons l WHERE l.id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn insert<'e, E: PgExecutor<'e>>(db: E, owner_id: Uuid, f: &LessonFields<'_>) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO lessons (owner_id, subject_id, grade_id, title, summary, content, video_url, duration_minutes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind(owner_id)
    .bind(f.subject_id)
    .bind(f.grade_id)
    .bind(f.title)
    .bind(f.summary)
    .bind(f.content)
    .bind(f.video_url)
    .bind(f.duration_minutes)
    .fetch_one(db)
    .await
}

pub async fn update<'e, E: PgExecutor<'e>>(db: E, id: Uuid, f: &LessonFields<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE lessons SET subject_id = $2, grade_id = $3, title = $4, summary = $5, content = $6, video_url = $7, \
         duration_minutes = $8, has_changes = true, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(f.subject_id)
    .bind(f.grade_id)
    .bind(f.title)
    .bind(f.summary)
    .bind(f.content)
    .bind(f.video_url)
    .bind(f.duration_minutes)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_tags(conn: &mut sqlx::PgConnection, id: Uuid, tag_ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM lesson_tags WHERE lesson_id = $1").bind(id).execute(&mut *conn).await?;
    sqlx::query("INSERT INTO lesson_tags (lesson_id, tag_id) SELECT $1, unnest($2::uuid[]) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(tag_ids)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn archive<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE lessons SET archived_at = now(), updated_at = now() WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Chép bản nháp thành phiên bản mới, trỏ bài tới phiên bản đó. Trả số phiên bản.
pub async fn publish(conn: &mut sqlx::PgConnection, id: Uuid, by: Uuid) -> sqlx::Result<i32> {
    let (rev_id, version) = sqlx::query_as::<_, (Uuid, i32)>(
        "INSERT INTO lesson_revisions (lesson_id, version, title, summary, content, video_url, duration_minutes, published_by) \
         SELECT l.id, COALESCE((SELECT max(version) FROM lesson_revisions r WHERE r.lesson_id = l.id), 0) + 1, \
                l.title, l.summary, l.content, l.video_url, l.duration_minutes, $2 \
         FROM lessons l WHERE l.id = $1 RETURNING id, version",
    )
    .bind(id)
    .bind(by)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query("UPDATE lessons SET published_revision_id = $2, has_changes = false, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(rev_id)
        .execute(&mut *conn)
        .await?;
    Ok(version)
}

pub async fn revisions<'e, E: PgExecutor<'e>>(db: E, lesson_id: Uuid) -> sqlx::Result<Vec<LessonRevision>> {
    sqlx::query_as::<_, LessonRevision>(
        "SELECT id, lesson_id, version, title, summary, content, video_url, duration_minutes, published_at \
         FROM lesson_revisions WHERE lesson_id = $1 ORDER BY version DESC",
    )
    .bind(lesson_id)
    .fetch_all(db)
    .await
}

pub async fn find_revision<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<LessonRevision>> {
    sqlx::query_as::<_, LessonRevision>(
        "SELECT id, lesson_id, version, title, summary, content, video_url, duration_minutes, published_at \
         FROM lesson_revisions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

/// Thông tin rút gọn của nhiều bài (dựng lộ trình).
pub async fn refs<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid]) -> sqlx::Result<Vec<LessonRef>> {
    sqlx::query_as::<_, LessonRef>(
        "SELECT l.id, l.owner_id, COALESCE(r.title, l.title) AS title, COALESCE(r.duration_minutes, l.duration_minutes) AS duration_minutes, \
                l.published_revision_id, l.archived_at IS NOT NULL AS archived \
         FROM lessons l LEFT JOIN lesson_revisions r ON r.id = l.published_revision_id WHERE l.id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(db)
    .await
}
