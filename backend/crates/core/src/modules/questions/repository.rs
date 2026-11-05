use sqlx::{PgExecutor, Postgres, QueryBuilder, types::Json};
use uuid::Uuid;

use super::model::{Folder, QuestionBody, QuestionRow, QuestionVersion};

const QUESTION_COLS: &str = "q.id, q.owner_id, q.subject_id, q.grade_id, q.folder_id, q.kind, q.level, q.current_version_id, \
     v.version, v.body, array(SELECT tag_id FROM question_tags t WHERE t.question_id = q.id) AS tag_ids, \
     q.archived_at, q.created_at, q.updated_at, \
     (SELECT count(DISTINCT attempt_id) FROM attempt_items ai WHERE ai.question_id = q.id) AS used_count";

pub struct QuestionFields<'a> {
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub kind: &'a str,
    pub level: &'a str,
    pub search_text: &'a str,
}

pub struct QuestionFilter<'a> {
    pub owner_id: Uuid,
    pub q: Option<&'a str>,
    pub subject_id: Option<Uuid>,
    /// `Some(None)` = câu chưa xếp thư mục.
    pub folder_id: Option<Option<Uuid>>,
    pub tag_id: Option<Uuid>,
    pub level: Option<&'a str>,
    pub kind: Option<&'a str>,
    /// Bỏ qua các câu này (rút câu cho ma trận đề).
    pub exclude: &'a [Uuid],
}

fn push_filter(qb: &mut QueryBuilder<Postgres>, f: &QuestionFilter<'_>) {
    qb.push(" WHERE q.archived_at IS NULL AND q.owner_id = ").push_bind(f.owner_id);
    if let Some(q) = f.q {
        qb.push(" AND q.search_text LIKE ").push_bind(format!("%{}%", crate::db::escape_like(&crate::slug::fold(q))));
    }
    if let Some(s) = f.subject_id {
        qb.push(" AND q.subject_id = ").push_bind(s);
    }
    match f.folder_id {
        Some(Some(id)) => {
            qb.push(" AND q.folder_id = ").push_bind(id);
        }
        Some(None) => {
            qb.push(" AND q.folder_id IS NULL");
        }
        None => {}
    }
    if let Some(t) = f.tag_id {
        qb.push(" AND EXISTS (SELECT 1 FROM question_tags qt WHERE qt.question_id = q.id AND qt.tag_id = ")
            .push_bind(t)
            .push(")");
    }
    if let Some(l) = f.level {
        qb.push(" AND q.level = ").push_bind(l.to_string());
    }
    if let Some(k) = f.kind {
        qb.push(" AND q.kind = ").push_bind(k.to_string());
    }
    if !f.exclude.is_empty() {
        qb.push(" AND q.id <> ALL(").push_bind(f.exclude.to_vec()).push(")");
    }
}

pub async fn list<'e, E: PgExecutor<'e>>(
    db: E,
    f: &QuestionFilter<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<QuestionRow>> {
    let mut qb = QueryBuilder::new(format!(
        "SELECT {QUESTION_COLS} FROM questions q JOIN question_versions v ON v.id = q.current_version_id"
    ));
    push_filter(&mut qb, f);
    qb.push(" ORDER BY q.updated_at DESC, q.id DESC LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);
    qb.build_query_as::<QuestionRow>().fetch_all(db).await
}

pub async fn count<'e, E: PgExecutor<'e>>(db: E, f: &QuestionFilter<'_>) -> sqlx::Result<i64> {
    let mut qb = QueryBuilder::new("SELECT count(*) FROM questions q");
    push_filter(&mut qb, f);
    qb.build_query_scalar::<i64>().fetch_one(db).await
}

/// Rút ngẫu nhiên tối đa `limit` ID câu theo bộ lọc (ma trận đề).
pub async fn random_ids<'e, E: PgExecutor<'e>>(db: E, f: &QuestionFilter<'_>, limit: i64) -> sqlx::Result<Vec<Uuid>> {
    let mut qb = QueryBuilder::new("SELECT q.id FROM questions q");
    push_filter(&mut qb, f);
    qb.push(" ORDER BY random() LIMIT ").push_bind(limit);
    qb.build_query_scalar::<Uuid>().fetch_all(db).await
}

pub async fn find<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<QuestionRow>> {
    sqlx::query_as::<_, QuestionRow>(crate::sql!(
        "SELECT {QUESTION_COLS} FROM questions q JOIN question_versions v ON v.id = q.current_version_id WHERE q.id = $1"
    ))
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn find_many<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid]) -> sqlx::Result<Vec<QuestionRow>> {
    sqlx::query_as::<_, QuestionRow>(crate::sql!(
        "SELECT {QUESTION_COLS} FROM questions q JOIN question_versions v ON v.id = q.current_version_id WHERE q.id = ANY($1)"
    ))
    .bind(ids)
    .fetch_all(db)
    .await
}

pub async fn insert(conn: &mut sqlx::PgConnection, owner_id: Uuid, f: &QuestionFields<'_>) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO questions (owner_id, subject_id, grade_id, folder_id, kind, level, search_text) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(owner_id)
    .bind(f.subject_id)
    .bind(f.grade_id)
    .bind(f.folder_id)
    .bind(f.kind)
    .bind(f.level)
    .bind(f.search_text)
    .fetch_one(&mut *conn)
    .await
}

pub async fn update(conn: &mut sqlx::PgConnection, id: Uuid, f: &QuestionFields<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE questions SET subject_id = $2, grade_id = $3, folder_id = $4, kind = $5, level = $6, search_text = $7, \
         updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(f.subject_id)
    .bind(f.grade_id)
    .bind(f.folder_id)
    .bind(f.kind)
    .bind(f.level)
    .bind(f.search_text)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Thêm phiên bản mới và đặt làm phiên bản hiện tại.
pub async fn add_version(
    conn: &mut sqlx::PgConnection,
    question_id: Uuid,
    body: &QuestionBody,
    by: Uuid,
) -> sqlx::Result<i32> {
    let (id, version) = sqlx::query_as::<_, (Uuid, i32)>(
        "INSERT INTO question_versions (question_id, version, body, created_by) \
         SELECT $1, COALESCE(max(version), 0) + 1, $2, $3 FROM question_versions WHERE question_id = $1 \
         RETURNING id, version",
    )
    .bind(question_id)
    .bind(Json(body))
    .bind(by)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query("UPDATE questions SET current_version_id = $2, updated_at = now() WHERE id = $1")
        .bind(question_id)
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(version)
}

pub async fn set_tags(conn: &mut sqlx::PgConnection, id: Uuid, tag_ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM question_tags WHERE question_id = $1").bind(id).execute(&mut *conn).await?;
    sqlx::query("INSERT INTO question_tags (question_id, tag_id) SELECT $1, unnest($2::uuid[]) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(tag_ids)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn archive<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE questions SET archived_at = now(), updated_at = now() WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn find_versions<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid]) -> sqlx::Result<Vec<QuestionVersion>> {
    sqlx::query_as::<_, QuestionVersion>(
        "SELECT v.id, v.question_id, v.version, q.kind, v.body FROM question_versions v \
         JOIN questions q ON q.id = v.question_id WHERE v.id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(db)
    .await
}

/// Số bài đã nộp có câu này.
pub async fn count_submitted_attempts<'e, E: PgExecutor<'e>>(db: E, question_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(DISTINCT a.id) FROM attempt_items ai JOIN attempts a ON a.id = ai.attempt_id \
         WHERE ai.question_id = $1 AND a.status = 'submitted'",
    )
    .bind(question_id)
    .fetch_one(db)
    .await
}

// ---------- Thư mục ----------

pub async fn list_folders<'e, E: PgExecutor<'e>>(db: E, owner_id: Uuid) -> sqlx::Result<Vec<Folder>> {
    sqlx::query_as::<_, Folder>(
        "SELECT f.id, f.parent_id, f.name, f.sort_order, \
                (SELECT count(*) FROM questions q WHERE q.folder_id = f.id AND q.archived_at IS NULL) AS question_count \
         FROM question_folders f WHERE f.owner_id = $1 ORDER BY f.sort_order, f.name",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await
}

pub async fn folder_owner<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM question_folders WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn insert_folder<'e, E: PgExecutor<'e>>(
    db: E,
    owner_id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO question_folders (owner_id, parent_id, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(owner_id)
    .bind(parent_id)
    .bind(name)
    .fetch_one(db)
    .await
}

pub async fn update_folder<'e, E: PgExecutor<'e>>(
    db: E,
    id: Uuid,
    parent_id: Option<Uuid>,
    name: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE question_folders SET parent_id = $2, name = $3 WHERE id = $1")
        .bind(id)
        .bind(parent_id)
        .bind(name)
        .execute(db)
        .await?;
    Ok(())
}

/// Xóa thư mục (cả thư mục con); câu hỏi bên trong chuyển về "chưa xếp thư mục".
pub async fn delete_folder<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM question_folders WHERE id = $1").bind(id).execute(db).await?;
    Ok(())
}

/// Các thư mục con cháu của `id` (kể cả chính nó) — chống đặt thư mục vào trong con của nó.
pub async fn descendant_ids<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar::<_, Uuid>(
        "WITH RECURSIVE d AS (SELECT id FROM question_folders WHERE id = $1 \
           UNION ALL SELECT f.id FROM question_folders f JOIN d ON f.parent_id = d.id) SELECT id FROM d",
    )
    .bind(id)
    .fetch_all(db)
    .await
}
