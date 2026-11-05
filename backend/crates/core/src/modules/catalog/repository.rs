use sqlx::PgExecutor;
use uuid::Uuid;

use super::model::{Grade, PendingItem, Subject, Tag};

macro_rules! subject_cols {
    () => {
        "id, name, slug, sort_order, is_active, created_at, updated_at"
    };
}
macro_rules! grade_cols {
    () => {
        "id, name, slug, sort_order, is_system, owner_id, merged_into, is_active, created_at, updated_at"
    };
}
macro_rules! tag_cols {
    () => {
        "id, subject_id, name, slug, sort_order, owner_id, merged_into, is_active, created_at, updated_at"
    };
}

// ---------- Môn học ----------

pub async fn list_subjects<'e, E: PgExecutor<'e>>(db: E, include_inactive: bool) -> sqlx::Result<Vec<Subject>> {
    sqlx::query_as::<_, Subject>(concat!(
        "SELECT ",
        subject_cols!(),
        " FROM subjects WHERE ($1 OR is_active) ORDER BY sort_order, name"
    ))
    .bind(include_inactive)
    .fetch_all(db)
    .await
}

pub async fn find_subject<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<Subject>> {
    sqlx::query_as::<_, Subject>(concat!("SELECT ", subject_cols!(), " FROM subjects WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn insert_subject<'e, E: PgExecutor<'e>>(
    db: E,
    name: &str,
    slug: &str,
    sort_order: i32,
) -> sqlx::Result<Subject> {
    sqlx::query_as::<_, Subject>(concat!(
        "INSERT INTO subjects (name, slug, sort_order) VALUES ($1, $2, $3) RETURNING ",
        subject_cols!()
    ))
    .bind(name)
    .bind(slug)
    .bind(sort_order)
    .fetch_one(db)
    .await
}

pub async fn update_subject<'e, E: PgExecutor<'e>>(
    db: E,
    id: Uuid,
    name_slug: Option<(&str, &str)>,
    sort_order: Option<i32>,
    is_active: Option<bool>,
) -> sqlx::Result<Subject> {
    let (name, slug) = name_slug.unzip();
    sqlx::query_as::<_, Subject>(concat!(
        "UPDATE subjects SET name = COALESCE($2, name), slug = COALESCE($3, slug), \
         sort_order = COALESCE($4, sort_order), is_active = COALESCE($5, is_active), updated_at = now() \
         WHERE id = $1 RETURNING ",
        subject_cols!()
    ))
    .bind(id)
    .bind(name)
    .bind(slug)
    .bind(sort_order)
    .bind(is_active)
    .fetch_one(db)
    .await
}

pub async fn subjects_exist<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid]) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM subjects WHERE id = ANY($1) AND is_active")
        .bind(ids)
        .fetch_one(db)
        .await
}

// ---------- Lớp ----------

/// Lớp người xem thấy được: lớp dùng chung + lớp riêng của chính họ (`viewer`).
pub async fn list_visible_grades<'e, E: PgExecutor<'e>>(db: E, viewer: Option<Uuid>) -> sqlx::Result<Vec<Grade>> {
    sqlx::query_as::<_, Grade>(concat!(
        "SELECT ",
        grade_cols!(),
        " FROM grades WHERE merged_into IS NULL AND is_active AND (owner_id IS NULL OR owner_id = $1) \
         ORDER BY (owner_id IS NOT NULL), sort_order, name"
    ))
    .bind(viewer)
    .fetch_all(db)
    .await
}

/// Lớp dùng chung, kể cả đang ẩn (màn hình admin).
pub async fn list_shared_grades<'e, E: PgExecutor<'e>>(db: E) -> sqlx::Result<Vec<Grade>> {
    sqlx::query_as::<_, Grade>(concat!(
        "SELECT ",
        grade_cols!(),
        " FROM grades WHERE merged_into IS NULL AND owner_id IS NULL ORDER BY sort_order, name"
    ))
    .fetch_all(db)
    .await
}

pub async fn find_grade<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<Grade>> {
    sqlx::query_as::<_, Grade>(concat!("SELECT ", grade_cols!(), " FROM grades WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Lớp trùng slug mà người xem thấy được (chung hoặc của chính họ).
pub async fn find_visible_grade_by_slug<'e, E: PgExecutor<'e>>(
    db: E,
    slug: &str,
    viewer: Uuid,
) -> sqlx::Result<Option<Grade>> {
    sqlx::query_as::<_, Grade>(concat!(
        "SELECT ",
        grade_cols!(),
        " FROM grades WHERE slug = $1 AND merged_into IS NULL AND (owner_id IS NULL OR owner_id = $2) \
         ORDER BY (owner_id IS NOT NULL) LIMIT 1"
    ))
    .bind(slug)
    .bind(viewer)
    .fetch_optional(db)
    .await
}

pub async fn insert_grade<'e, E: PgExecutor<'e>>(
    db: E,
    name: &str,
    slug: &str,
    sort_order: i32,
    owner_id: Option<Uuid>,
) -> sqlx::Result<Grade> {
    sqlx::query_as::<_, Grade>(concat!(
        "INSERT INTO grades (name, slug, sort_order, owner_id) VALUES ($1, $2, $3, $4) RETURNING ",
        grade_cols!()
    ))
    .bind(name)
    .bind(slug)
    .bind(sort_order)
    .bind(owner_id)
    .fetch_one(db)
    .await
}

pub async fn update_grade<'e, E: PgExecutor<'e>>(
    db: E,
    id: Uuid,
    name_slug: Option<(&str, &str)>,
    sort_order: Option<i32>,
    is_active: Option<bool>,
) -> sqlx::Result<Grade> {
    let (name, slug) = name_slug.unzip();
    sqlx::query_as::<_, Grade>(concat!(
        "UPDATE grades SET name = COALESCE($2, name), slug = COALESCE($3, slug), \
         sort_order = COALESCE($4, sort_order), is_active = COALESCE($5, is_active), updated_at = now() \
         WHERE id = $1 RETURNING ",
        grade_cols!()
    ))
    .bind(id)
    .bind(name)
    .bind(slug)
    .bind(sort_order)
    .bind(is_active)
    .fetch_one(db)
    .await
}

pub async fn promote_grade<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Grade> {
    sqlx::query_as::<_, Grade>(concat!(
        "UPDATE grades SET owner_id = NULL, updated_at = now() WHERE id = $1 RETURNING ",
        grade_cols!()
    ))
    .bind(id)
    .fetch_one(db)
    .await
}

pub async fn merge_grade<'e, E: PgExecutor<'e>>(db: E, id: Uuid, into: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE grades SET merged_into = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(into)
        .execute(db)
        .await?;
    Ok(())
}

/// Đếm số lớp hợp lệ (đang dùng, chưa gộp) trong danh sách mà `viewer` thấy được.
pub async fn count_visible_grades<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid], viewer: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM grades WHERE id = ANY($1) AND merged_into IS NULL AND is_active \
         AND (owner_id IS NULL OR owner_id = $2)",
    )
    .bind(ids)
    .bind(viewer)
    .fetch_one(db)
    .await
}

// ---------- Danh mục (nhãn trong môn) ----------

pub async fn list_visible_tags<'e, E: PgExecutor<'e>>(
    db: E,
    subject_id: Uuid,
    viewer: Option<Uuid>,
) -> sqlx::Result<Vec<Tag>> {
    sqlx::query_as::<_, Tag>(concat!(
        "SELECT ",
        tag_cols!(),
        " FROM tags WHERE subject_id = $1 AND merged_into IS NULL AND is_active \
         AND (owner_id IS NULL OR owner_id = $2) ORDER BY (owner_id IS NOT NULL), sort_order, name"
    ))
    .bind(subject_id)
    .bind(viewer)
    .fetch_all(db)
    .await
}

pub async fn list_shared_tags<'e, E: PgExecutor<'e>>(db: E, subject_id: Uuid) -> sqlx::Result<Vec<Tag>> {
    sqlx::query_as::<_, Tag>(concat!(
        "SELECT ",
        tag_cols!(),
        " FROM tags WHERE subject_id = $1 AND merged_into IS NULL AND owner_id IS NULL ORDER BY sort_order, name"
    ))
    .bind(subject_id)
    .fetch_all(db)
    .await
}

pub async fn find_tag<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<Tag>> {
    sqlx::query_as::<_, Tag>(concat!("SELECT ", tag_cols!(), " FROM tags WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn find_visible_tag_by_slug<'e, E: PgExecutor<'e>>(
    db: E,
    subject_id: Uuid,
    slug: &str,
    viewer: Uuid,
) -> sqlx::Result<Option<Tag>> {
    sqlx::query_as::<_, Tag>(concat!(
        "SELECT ",
        tag_cols!(),
        " FROM tags WHERE subject_id = $1 AND slug = $2 AND merged_into IS NULL \
         AND (owner_id IS NULL OR owner_id = $3) ORDER BY (owner_id IS NOT NULL) LIMIT 1"
    ))
    .bind(subject_id)
    .bind(slug)
    .bind(viewer)
    .fetch_optional(db)
    .await
}

pub async fn insert_tag<'e, E: PgExecutor<'e>>(
    db: E,
    subject_id: Uuid,
    name: &str,
    slug: &str,
    sort_order: i32,
    owner_id: Option<Uuid>,
) -> sqlx::Result<Tag> {
    sqlx::query_as::<_, Tag>(concat!(
        "INSERT INTO tags (subject_id, name, slug, sort_order, owner_id) VALUES ($1, $2, $3, $4, $5) RETURNING ",
        tag_cols!()
    ))
    .bind(subject_id)
    .bind(name)
    .bind(slug)
    .bind(sort_order)
    .bind(owner_id)
    .fetch_one(db)
    .await
}

pub async fn update_tag<'e, E: PgExecutor<'e>>(
    db: E,
    id: Uuid,
    name_slug: Option<(&str, &str)>,
    sort_order: Option<i32>,
    is_active: Option<bool>,
) -> sqlx::Result<Tag> {
    let (name, slug) = name_slug.unzip();
    sqlx::query_as::<_, Tag>(concat!(
        "UPDATE tags SET name = COALESCE($2, name), slug = COALESCE($3, slug), \
         sort_order = COALESCE($4, sort_order), is_active = COALESCE($5, is_active), updated_at = now() \
         WHERE id = $1 RETURNING ",
        tag_cols!()
    ))
    .bind(id)
    .bind(name)
    .bind(slug)
    .bind(sort_order)
    .bind(is_active)
    .fetch_one(db)
    .await
}

pub async fn promote_tag<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Tag> {
    sqlx::query_as::<_, Tag>(concat!(
        "UPDATE tags SET owner_id = NULL, updated_at = now() WHERE id = $1 RETURNING ",
        tag_cols!()
    ))
    .bind(id)
    .fetch_one(db)
    .await
}

pub async fn merge_tag<'e, E: PgExecutor<'e>>(db: E, id: Uuid, into: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE tags SET merged_into = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(into)
        .execute(db)
        .await?;
    Ok(())
}

// ---------- Mục chờ admin xử lý ----------

pub async fn list_pending<'e, E: PgExecutor<'e>>(db: E) -> sqlx::Result<Vec<PendingItem>> {
    sqlx::query_as::<_, PendingItem>(
        "SELECT 'grade' AS kind, g.id, g.name, NULL::uuid AS subject_id, NULL::text AS subject_name, \
                g.owner_id, u.username AS owner_username, g.created_at \
         FROM grades g JOIN users u ON u.id = g.owner_id \
         WHERE g.owner_id IS NOT NULL AND g.merged_into IS NULL AND g.is_active \
         UNION ALL \
         SELECT 'tag', t.id, t.name, t.subject_id, s.name, t.owner_id, u.username, t.created_at \
         FROM tags t JOIN users u ON u.id = t.owner_id JOIN subjects s ON s.id = t.subject_id \
         WHERE t.owner_id IS NOT NULL AND t.merged_into IS NULL AND t.is_active \
         ORDER BY created_at",
    )
    .fetch_all(db)
    .await
}
