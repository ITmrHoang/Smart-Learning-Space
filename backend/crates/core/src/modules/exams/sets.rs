//! Bộ đề (M41): giáo viên gom các đề của mình thành bộ ("20 đề ôn thi vào 10 — 2026"), sắp thứ tự, rồi giao nhanh
//! từng đề cho lớp. Đề vẫn là đề riêng (sửa / làm thử / trộn mã như thường); bộ đề chỉ giữ danh sách + thứ tự.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::service as exams;
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::teachers::service as teachers,
    security::{Actor, Permission},
};

/// Mỗi bộ đề tối đa chừng này đề.
pub const MAX_EXAMS: usize = 100;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ExamSetInput {
    pub subject_id: Uuid,
    #[validate(length(min = 1, max = 200, message = "Tên bộ đề từ 1 đến 200 ký tự."))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 2000, message = "Mô tả tối đa 2.000 ký tự."))]
    pub description: String,
    /// Các đề theo thứ tự.
    #[serde(default)]
    pub exam_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ExamSetSummary {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub title: String,
    pub description: String,
    pub exam_count: i64,
    pub updated_at: DateTime<Utc>,
}

/// Một đề trong bộ.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetExam {
    pub id: Uuid,
    pub title: String,
    pub kind: String,
    pub time_limit_minutes: Option<i32>,
    pub question_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamSetDetail {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub title: String,
    pub description: String,
    pub exams: Vec<SetExam>,
    pub updated_at: DateTime<Utc>,
}

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy bộ đề.")
}

const SUMMARY: &str = "SELECT s.id, s.subject_id, s.title, s.description, s.updated_at, \
     (SELECT count(*) FROM exam_set_items i JOIN exams e ON e.id = i.exam_id \
      WHERE i.set_id = s.id AND e.archived_at IS NULL) AS exam_count FROM exam_sets s";

pub async fn list(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<ExamSetSummary>> {
    actor.require(Permission::TeachingManage)?;
    Ok(sqlx::query_as::<_, ExamSetSummary>(crate::sql!(
        "{SUMMARY} WHERE s.owner_id = $1 AND s.archived_at IS NULL ORDER BY s.updated_at DESC LIMIT 300"
    ))
    .bind(actor.id)
    .fetch_all(&ctx.db)
    .await?)
}

async fn load_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<ExamSetSummary> {
    actor.require(Permission::TeachingManage)?;
    sqlx::query_as::<_, ExamSetSummary>(crate::sql!(
        "{SUMMARY} WHERE s.id = $1 AND s.owner_id = $2 AND s.archived_at IS NULL"
    ))
    .bind(id)
    .bind(actor.id)
    .fetch_optional(&ctx.db)
    .await?
    .ok_or_else(not_found)
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<ExamSetDetail> {
    let s = load_own(ctx, actor, id).await?;
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT exam_id FROM exam_set_items WHERE set_id = $1 ORDER BY position")
        .bind(id)
        .fetch_all(&ctx.db)
        .await?;
    let refs = exams::refs(ctx, &ids).await?;
    // Giữ thứ tự của bộ; bỏ đề đã ẩn
    let list = ids
        .iter()
        .filter_map(|id| refs.iter().find(|r| r.id == *id && !r.archived))
        .map(|r| SetExam {
            id: r.id,
            title: r.title.clone(),
            kind: r.kind.clone(),
            time_limit_minutes: r.time_limit_minutes,
            question_count: r.item_count,
        })
        .collect();
    Ok(ExamSetDetail {
        id: s.id,
        subject_id: s.subject_id,
        title: s.title,
        description: s.description,
        exams: list,
        updated_at: s.updated_at,
    })
}

/// Đề phải của chính giáo viên, chưa ẩn, cùng môn với bộ; bỏ trùng giữ thứ tự.
async fn check_exams(ctx: &AppContext, actor: &Actor, input: &ExamSetInput) -> AppResult<Vec<Uuid>> {
    input.validate()?;
    teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    let mut ids = Vec::with_capacity(input.exam_ids.len());
    for id in &input.exam_ids {
        if !ids.contains(id) {
            ids.push(*id);
        }
    }
    if ids.len() > MAX_EXAMS {
        return Err(AppError::field("examIds", format!("Mỗi bộ tối đa {MAX_EXAMS} đề.")));
    }
    let refs = exams::refs(ctx, &ids).await?;
    let subjects: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id, subject_id FROM exams WHERE id = ANY($1)").bind(&ids).fetch_all(&ctx.db).await?;
    let ok = refs.len() == ids.len()
        && refs.iter().all(|r| r.owner_id == actor.id && !r.archived)
        && subjects.iter().all(|(_, s)| *s == input.subject_id);
    if !ok {
        return Err(AppError::field("examIds", "Có đề không tồn tại, đã bị ẩn hoặc khác môn của bộ đề."));
    }
    Ok(ids)
}

async fn write_items(conn: &mut sqlx::PgConnection, set_id: Uuid, ids: &[Uuid]) -> AppResult<()> {
    sqlx::query("DELETE FROM exam_set_items WHERE set_id = $1").bind(set_id).execute(&mut *conn).await?;
    sqlx::query(
        "INSERT INTO exam_set_items (set_id, exam_id, position) \
         SELECT $1, eid, (ord - 1)::int FROM unnest($2::uuid[]) WITH ORDINALITY AS t(eid, ord)",
    )
    .bind(set_id)
    .bind(ids)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: ExamSetInput) -> AppResult<ExamSetDetail> {
    actor.require(Permission::TeachingManage)?;
    let ids = check_exams(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO exam_sets (owner_id, subject_id, title, description) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(actor.id)
    .bind(input.subject_id)
    .bind(input.title.trim())
    .bind(input.description.trim())
    .fetch_one(&mut *tx)
    .await?;
    write_items(&mut tx, id, &ids).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: ExamSetInput) -> AppResult<ExamSetDetail> {
    load_own(ctx, actor, id).await?;
    let ids = check_exams(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    sqlx::query("UPDATE exam_sets SET subject_id = $2, title = $3, description = $4, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(input.subject_id)
        .bind(input.title.trim())
        .bind(input.description.trim())
        .execute(&mut *tx)
        .await?;
    write_items(&mut tx, id, &ids).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

pub async fn archive(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    load_own(ctx, actor, id).await?;
    sqlx::query("UPDATE exam_sets SET archived_at = now(), updated_at = now() WHERE id = $1")
        .bind(id)
        .execute(&ctx.db)
        .await?;
    Ok(())
}
