//! Nhận xét mẫu của giáo viên (M39): câu nhận xét hay dùng ("Trình bày rõ ràng", "Thiếu điều kiện xác định"…),
//! bấm để chèn khi chấm câu tự luận và chấm bài tập về nhà.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    security::{Actor, Permission},
};

/// Mỗi giáo viên giữ tối đa chừng này nhận xét mẫu.
pub const MAX_SNIPPETS: i64 = 100;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SnippetInput {
    #[validate(length(min = 1, max = 1000, message = "Nhận xét mẫu từ 1 đến 1.000 ký tự."))]
    pub body: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

pub async fn list(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<Snippet>> {
    actor.require(Permission::TeachingManage)?;
    Ok(sqlx::query_as::<_, Snippet>(
        "SELECT id, body, created_at FROM comment_snippets WHERE owner_id = $1 ORDER BY created_at LIMIT 200",
    )
    .bind(actor.id)
    .fetch_all(&ctx.db)
    .await?)
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: SnippetInput) -> AppResult<Snippet> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    let body = input.body.trim();
    if body.is_empty() {
        return Err(AppError::field("body", "Nhận xét mẫu không được để trống."));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM comment_snippets WHERE owner_id = $1")
        .bind(actor.id)
        .fetch_one(&ctx.db)
        .await?;
    if count >= MAX_SNIPPETS {
        return Err(AppError::conflict("SNIPPET_LIMIT", format!("Tối đa {MAX_SNIPPETS} nhận xét mẫu. Hãy xóa bớt.")));
    }
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM comment_snippets WHERE owner_id = $1 AND body = $2)")
            .bind(actor.id)
            .bind(body)
            .fetch_one(&ctx.db)
            .await?;
    if exists {
        return Err(AppError::field("body", "Đã có nhận xét mẫu này."));
    }
    Ok(sqlx::query_as::<_, Snippet>(
        "INSERT INTO comment_snippets (owner_id, body) VALUES ($1, $2) RETURNING id, body, created_at",
    )
    .bind(actor.id)
    .bind(body)
    .fetch_one(&ctx.db)
    .await?)
}

pub async fn delete(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    actor.require(Permission::TeachingManage)?;
    let res = sqlx::query("DELETE FROM comment_snippets WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(actor.id)
        .execute(&ctx.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("Không tìm thấy nhận xét mẫu."));
    }
    Ok(())
}
