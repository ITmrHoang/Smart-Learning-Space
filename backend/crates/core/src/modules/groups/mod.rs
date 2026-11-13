//! Nhóm học sinh (M13): giáo viên gom học sinh từ các lớp học của mình (một hoặc nhiều lớp)
//! để mời buổi học. Thành viên nhóm phải đang học ít nhất một lớp học của giáo viên đó.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    security::{Actor, Permission},
};

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GroupInput {
    #[validate(length(min = 1, max = 100, message = "Tên nhóm từ 1 đến 100 ký tự."))]
    pub name: String,
    pub class_id: Option<Uuid>,
    #[serde(default)]
    #[validate(length(max = 500, message = "Nhóm tối đa 500 học sinh."))]
    pub member_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupResponse {
    pub id: Uuid,
    pub name: String,
    pub class_id: Option<Uuid>,
    pub class_title: Option<String>,
    pub member_count: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentRef {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub email: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDetail {
    #[serde(flatten)]
    pub group: GroupResponse,
    pub members: Vec<StudentRef>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StudentQuery {
    pub class_id: Option<Uuid>,
    pub q: Option<String>,
}

const GROUP_COLS: &str = "g.id, g.name, g.class_id, c.title AS class_title, \
     (SELECT count(*) FROM student_group_members m WHERE m.group_id = g.id) AS member_count, g.updated_at";

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy nhóm.")
}

/// Học sinh đang học các lớp học của giáo viên (để chọn vào nhóm / mời buổi học).
pub async fn students(ctx: &AppContext, actor: &Actor, q: StudentQuery) -> AppResult<Vec<StudentRef>> {
    actor.require(Permission::TeachingManage)?;
    let search =
        q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| format!("%{}%", crate::db::escape_like(s)));
    Ok(sqlx::query_as::<_, StudentRef>(
        "SELECT DISTINCT u.id, u.username, u.full_name, u.email FROM class_members m \
         JOIN classes c ON c.id = m.class_id JOIN users u ON u.id = m.user_id \
         WHERE c.owner_id = $1 AND m.role = 'student' AND m.status = 'active' \
           AND ($2::uuid IS NULL OR c.id = $2) \
           AND ($3::text IS NULL OR unaccent(lower(u.full_name || ' ' || u.username || ' ' || u.email)) LIKE unaccent(lower($3))) \
         ORDER BY u.full_name LIMIT 300",
    )
    .bind(actor.id)
    .bind(q.class_id)
    .bind(search)
    .fetch_all(&ctx.db)
    .await?)
}

/// Kiểm tra mọi ID là học sinh đang học một lớp học của `owner`.
pub async fn ensure_own_students(ctx: &AppContext, owner: Uuid, ids: &[Uuid]) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut unique = ids.to_vec();
    unique.sort();
    unique.dedup();
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(DISTINCT m.user_id) FROM class_members m JOIN classes c ON c.id = m.class_id \
         WHERE c.owner_id = $1 AND m.role = 'student' AND m.status = 'active' AND m.user_id = ANY($2)",
    )
    .bind(owner)
    .bind(&unique)
    .fetch_one(&ctx.db)
    .await?;
    if count != unique.len() as i64 {
        return Err(AppError::field("memberIds", "Chỉ chọn được học sinh đang học các lớp học của bạn."));
    }
    Ok(())
}

async fn ensure_own_class(ctx: &AppContext, actor: &Actor, class_id: Option<Uuid>) -> AppResult<()> {
    if let Some(id) = class_id {
        let owner = sqlx::query_scalar::<_, Uuid>("SELECT owner_id FROM classes WHERE id = $1")
            .bind(id)
            .fetch_optional(&ctx.db)
            .await?;
        if owner != Some(actor.id) {
            return Err(AppError::field("classId", "Lớp học không tồn tại."));
        }
    }
    Ok(())
}

async fn load_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<GroupResponse> {
    actor.require(Permission::TeachingManage)?;
    sqlx::query_as::<_, GroupResponse>(crate::sql!(
        "SELECT {GROUP_COLS} FROM student_groups g LEFT JOIN classes c ON c.id = g.class_id WHERE g.id = $1 AND g.owner_id = $2"
    ))
    .bind(id)
    .bind(actor.id)
    .fetch_optional(&ctx.db)
    .await?
    .ok_or_else(not_found)
}

pub async fn list(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<GroupResponse>> {
    actor.require(Permission::TeachingManage)?;
    Ok(sqlx::query_as::<_, GroupResponse>(crate::sql!(
        "SELECT {GROUP_COLS} FROM student_groups g LEFT JOIN classes c ON c.id = g.class_id \
         WHERE g.owner_id = $1 ORDER BY g.name"
    ))
    .bind(actor.id)
    .fetch_all(&ctx.db)
    .await?)
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<GroupDetail> {
    let group = load_own(ctx, actor, id).await?;
    let members = sqlx::query_as::<_, StudentRef>(
        "SELECT u.id, u.username, u.full_name, u.email FROM student_group_members m JOIN users u ON u.id = m.user_id \
         WHERE m.group_id = $1 ORDER BY u.full_name",
    )
    .bind(id)
    .fetch_all(&ctx.db)
    .await?;
    Ok(GroupDetail { group, members })
}

async fn set_members(conn: &mut sqlx::PgConnection, id: Uuid, member_ids: &[Uuid]) -> AppResult<()> {
    sqlx::query("DELETE FROM student_group_members WHERE group_id = $1").bind(id).execute(&mut *conn).await?;
    sqlx::query(
        "INSERT INTO student_group_members (group_id, user_id) SELECT $1, unnest($2::uuid[]) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(member_ids)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: GroupInput) -> AppResult<GroupDetail> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    ensure_own_class(ctx, actor, input.class_id).await?;
    ensure_own_students(ctx, actor.id, &input.member_ids).await?;
    let mut tx = ctx.db.begin().await?;
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO student_groups (owner_id, class_id, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(actor.id)
    .bind(input.class_id)
    .bind(input.name.trim())
    .fetch_one(&mut *tx)
    .await?;
    set_members(&mut tx, id, &input.member_ids).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: GroupInput) -> AppResult<GroupDetail> {
    load_own(ctx, actor, id).await?;
    input.validate()?;
    ensure_own_class(ctx, actor, input.class_id).await?;
    ensure_own_students(ctx, actor.id, &input.member_ids).await?;
    let mut tx = ctx.db.begin().await?;
    sqlx::query("UPDATE student_groups SET name = $2, class_id = $3, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(input.name.trim())
        .bind(input.class_id)
        .execute(&mut *tx)
        .await?;
    set_members(&mut tx, id, &input.member_ids).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

pub async fn delete(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    load_own(ctx, actor, id).await?;
    sqlx::query("DELETE FROM student_groups WHERE id = $1").bind(id).execute(&ctx.db).await?;
    Ok(())
}

/// Nhóm thuộc `owner` (kiểm tra khi mời buổi học).
pub async fn ensure_own_groups(ctx: &AppContext, owner: Uuid, ids: &[Uuid]) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let count =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM student_groups WHERE owner_id = $1 AND id = ANY($2)")
            .bind(owner)
            .bind(ids)
            .fetch_one(&ctx.db)
            .await?;
    if count != ids.len() as i64 {
        return Err(AppError::field("invitees", "Có nhóm không tồn tại."));
    }
    Ok(())
}

/// Cặp (nhóm, học sinh) trong các nhóm của `owner`; `user_id` = chỉ nhóm có học sinh này
/// (lộ trình riêng: bước dành cho nhóm nào thì học sinh trong nhóm đó thấy).
pub async fn memberships(ctx: &AppContext, owner: Uuid, user_id: Option<Uuid>) -> AppResult<Vec<(Uuid, Uuid)>> {
    Ok(sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT m.group_id, m.user_id FROM student_group_members m JOIN student_groups g ON g.id = m.group_id \
         WHERE g.owner_id = $1 AND ($2::uuid IS NULL OR m.user_id = $2)",
    )
    .bind(owner)
    .bind(user_id)
    .fetch_all(&ctx.db)
    .await?)
}
