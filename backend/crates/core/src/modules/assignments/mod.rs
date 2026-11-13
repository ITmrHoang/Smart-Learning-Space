//! Thư viện bài tập về nhà của giáo viên (M22). Bài tập là một bước của lộ trình (`classes/homework.rs`
//! lo phần nộp bài / chấm bài trong lớp học); một bài tập dùng lại được ở nhiều lớp học như bộ thẻ.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::teachers::service as teachers,
    security::{Actor, Permission},
};

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentInput {
    pub subject_id: Uuid,
    #[validate(length(min = 1, max = 200, message = "Tên bài tập từ 1 đến 200 ký tự."))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 20000, message = "Đề bài tối đa 20.000 ký tự."))]
    pub instructions: String,
    /// Bỏ trống = chỉ nhận xét, không chấm điểm.
    #[validate(range(exclusive_min = 0.0, max = 1000.0, message = "Thang điểm từ trên 0 đến 1.000."))]
    pub max_score: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentQuery {
    pub q: Option<String>,
    pub subject_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentSummary {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub title: String,
    pub max_score: Option<f64>,
    pub updated_at: DateTime<Utc>,
    /// Số bài nộp ở mọi lớp học.
    pub submission_count: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentDetail {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub title: String,
    pub instructions: String,
    pub max_score: Option<f64>,
    pub updated_at: DateTime<Utc>,
}

/// Bài tập được trỏ tới từ lộ trình.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AssignmentRef {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub instructions: String,
    pub max_score: Option<f64>,
    pub archived: bool,
}

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy bài tập.")
}

const DETAIL_COLS: &str = "id, subject_id, title, instructions, max_score, updated_at";

async fn find_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<AssignmentDetail> {
    actor.require(Permission::TeachingManage)?;
    sqlx::query_as::<_, AssignmentDetail>(crate::sql!(
        "SELECT {DETAIL_COLS} FROM assignments WHERE id = $1 AND owner_id = $2 AND archived_at IS NULL"
    ))
    .bind(id)
    .bind(actor.id)
    .fetch_optional(&ctx.db)
    .await?
    .ok_or_else(not_found)
}

pub async fn list(ctx: &AppContext, actor: &Actor, q: AssignmentQuery) -> AppResult<Vec<AssignmentSummary>> {
    actor.require(Permission::TeachingManage)?;
    let pattern = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| format!("%{s}%"));
    Ok(sqlx::query_as::<_, AssignmentSummary>(
        "SELECT a.id, a.subject_id, a.title, a.max_score, a.updated_at, \
         (SELECT count(*) FROM homework_submissions s WHERE s.assignment_id = a.id) AS submission_count \
         FROM assignments a WHERE a.owner_id = $1 AND a.archived_at IS NULL \
         AND ($2::text IS NULL OR a.title ILIKE $2) AND ($3::uuid IS NULL OR a.subject_id = $3) \
         ORDER BY a.updated_at DESC LIMIT 200",
    )
    .bind(actor.id)
    .bind(pattern)
    .bind(q.subject_id)
    .fetch_all(&ctx.db)
    .await?)
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<AssignmentDetail> {
    find_own(ctx, actor, id).await
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: AssignmentInput) -> AppResult<AssignmentDetail> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    Ok(sqlx::query_as::<_, AssignmentDetail>(crate::sql!(
        "INSERT INTO assignments (owner_id, subject_id, title, instructions, max_score) \
         VALUES ($1, $2, $3, $4, $5) RETURNING {DETAIL_COLS}"
    ))
    .bind(actor.id)
    .bind(input.subject_id)
    .bind(input.title.trim())
    .bind(input.instructions.trim())
    .bind(input.max_score)
    .fetch_one(&ctx.db)
    .await?)
}

/// Sửa bài tập: mọi lớp học dùng bài tập này thấy nội dung mới ngay (không cần xuất bản lại lộ trình).
pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: AssignmentInput) -> AppResult<AssignmentDetail> {
    input.validate()?;
    let current = find_own(ctx, actor, id).await?;
    if current.subject_id != input.subject_id {
        teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    }
    if let Some(max) = input.max_score {
        let over: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM homework_submissions WHERE assignment_id = $1 AND score > $2)",
        )
        .bind(id)
        .bind(max)
        .fetch_one(&ctx.db)
        .await?;
        if over {
            return Err(AppError::field("maxScore", "Đã có bài được chấm cao hơn thang điểm mới."));
        }
    }
    Ok(sqlx::query_as::<_, AssignmentDetail>(crate::sql!(
        "UPDATE assignments SET subject_id = $2, title = $3, instructions = $4, max_score = $5, updated_at = now() \
         WHERE id = $1 RETURNING {DETAIL_COLS}"
    ))
    .bind(id)
    .bind(input.subject_id)
    .bind(input.title.trim())
    .bind(input.instructions.trim())
    .bind(input.max_score)
    .fetch_one(&ctx.db)
    .await?)
}

/// Ẩn bài tập (lớp học đang dùng vẫn nộp / chấm được, nhưng không xuất bản lại lộ trình được).
pub async fn archive(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    find_own(ctx, actor, id).await?;
    sqlx::query("UPDATE assignments SET archived_at = now() WHERE id = $1").bind(id).execute(&ctx.db).await?;
    Ok(())
}

pub async fn refs(ctx: &AppContext, ids: &[Uuid]) -> AppResult<Vec<AssignmentRef>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as::<_, AssignmentRef>(
        "SELECT id, owner_id, title, instructions, max_score, archived_at IS NOT NULL AS archived \
         FROM assignments WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(&ctx.db)
    .await?)
}
