//! Hỏi đáp theo bài (M18, đề xuất D3) + báo vi phạm (D7, phần hỏi đáp).
//!
//! - Ai được vào: giáo viên chủ lớp + học sinh đang học (`active`). Câu hỏi có thể gắn với một bước của lộ trình.
//! - Nội dung Markdown + công thức (chưa có ảnh tải lên — cần Garage).
//! - Giáo viên: ghim, ẩn / hiện câu hỏi và trả lời, chọn câu trả lời đúng (người hỏi cũng chọn được), xử lý báo vi phạm.
//!   Nội dung bị ẩn: học sinh không thấy câu hỏi; trả lời bị ẩn hiện "Nội dung đã bị ẩn".
//! - Không có tin nhắn riêng giữa hai học sinh: mọi trao đổi đều nằm trong lớp học, giáo viên xem được.
//! - Thông báo: câu hỏi mới → giáo viên; trả lời mới → người hỏi + những người đã trả lời; báo vi phạm → giáo viên.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        classes::{model::ClassRow, path, repository as classes},
        notifications::{self, NotificationKind},
    },
    pagination::{Page, PageRequest},
    security::Actor,
};

// ---------- DTO ----------

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ThreadInput {
    pub step_id: Option<Uuid>,
    #[validate(length(min = 1, max = 200, message = "Tiêu đề từ 1 đến 200 ký tự."))]
    pub title: String,
    #[validate(length(min = 1, max = 10000, message = "Nội dung từ 1 đến 10.000 ký tự."))]
    pub body: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct PostInput {
    #[validate(length(min = 1, max = 10000, message = "Nội dung từ 1 đến 10.000 ký tự."))]
    pub body: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ReportInput {
    /// thread | post
    pub target_type: String,
    pub target_id: Uuid,
    #[validate(length(min = 1, max = 500, message = "Lý do từ 1 đến 500 ký tự."))]
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveInput {
    /// hide | dismiss
    pub action: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThreadQuery {
    pub step_id: Option<Uuid>,
    /// all | open (chưa có câu trả lời đúng) | answered | mine
    pub status: Option<String>,
    pub q: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReportQuery {
    /// open | all
    pub status: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ThreadRow {
    id: Uuid,
    class_id: Uuid,
    step_id: Option<Uuid>,
    title: String,
    body: String,
    author_id: Uuid,
    author_name: String,
    pinned: bool,
    accepted_post_id: Option<Uuid>,
    hidden_at: Option<DateTime<Utc>>,
    post_count: i32,
    last_post_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[sqlx(default)]
    total: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct PostRow {
    id: Uuid,
    author_id: Uuid,
    author_name: String,
    body: String,
    hidden_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
    pub id: Uuid,
    pub class_id: Uuid,
    pub step_id: Option<Uuid>,
    pub step_title: Option<String>,
    pub title: String,
    pub author_id: Uuid,
    pub author_name: String,
    /// Người hỏi là giáo viên chủ lớp.
    pub by_teacher: bool,
    pub pinned: bool,
    pub answered: bool,
    pub hidden: bool,
    pub post_count: i32,
    pub last_post_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub mine: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostView {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_name: String,
    pub by_teacher: bool,
    /// Rỗng khi bị ẩn và người xem không phải giáo viên.
    pub body: String,
    pub hidden: bool,
    pub accepted: bool,
    pub mine: bool,
    pub reported_by_me: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadDetail {
    #[serde(flatten)]
    pub summary: ThreadSummary,
    pub body: String,
    pub updated_at: DateTime<Utc>,
    pub posts: Vec<PostView>,
    /// Người xem là giáo viên chủ lớp.
    pub can_moderate: bool,
    /// Người xem chọn được câu trả lời đúng (giáo viên hoặc người hỏi).
    pub can_accept: bool,
    pub reported_by_me: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ReportView {
    pub id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub thread_id: Uuid,
    pub thread_title: String,
    pub excerpt: String,
    pub author_name: String,
    pub reporter_name: String,
    pub reason: String,
    pub status: String,
    pub target_hidden: bool,
    pub created_at: DateTime<Utc>,
}

// ---------- Quyền ----------

struct Access {
    class: ClassRow,
    teacher: bool,
}

fn class_not_found() -> AppError {
    AppError::not_found("Không tìm thấy lớp học.")
}

fn thread_not_found() -> AppError {
    AppError::not_found("Không tìm thấy câu hỏi.")
}

/// Giáo viên chủ lớp hoặc học sinh đang học.
async fn access(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<Access> {
    let class = classes::find(&ctx.db, class_id).await?.ok_or_else(class_not_found)?;
    if class.owner_id == actor.id {
        return Ok(Access { class, teacher: true });
    }
    match classes::membership(&ctx.db, class_id, actor.id).await? {
        Some(m) if m.status == "active" => Ok(Access { class, teacher: false }),
        Some(_) => Err(AppError::forbidden("NOT_ENROLLED", "Bạn chưa ghi danh vào lớp học này.")),
        None if class.visibility == "private" => Err(class_not_found()),
        None => Err(AppError::forbidden("NOT_ENROLLED", "Bạn chưa ghi danh vào lớp học này.")),
    }
}

const THREAD_COLS: &str = "t.id, t.class_id, t.step_id, t.title, t.body, t.author_id, u.full_name AS author_name, t.pinned, \
     t.accepted_post_id, t.hidden_at, t.post_count, t.last_post_at, t.created_at, t.updated_at";

async fn load_thread(ctx: &AppContext, thread_id: Uuid) -> AppResult<ThreadRow> {
    sqlx::query_as::<_, ThreadRow>(crate::sql!(
        "SELECT {THREAD_COLS} FROM class_threads t JOIN users u ON u.id = t.author_id WHERE t.id = $1"
    ))
    .bind(thread_id)
    .fetch_optional(&ctx.db)
    .await?
    .ok_or_else(thread_not_found)
}

/// Câu hỏi + quyền của người xem; câu hỏi bị ẩn chỉ giáo viên thấy.
async fn thread_access(ctx: &AppContext, actor: &Actor, thread_id: Uuid) -> AppResult<(ThreadRow, Access)> {
    let t = load_thread(ctx, thread_id).await?;
    let a = access(ctx, actor, t.class_id).await?;
    if t.hidden_at.is_some() && !a.teacher {
        return Err(thread_not_found());
    }
    Ok((t, a))
}

fn summary(t: &ThreadRow, owner_id: Uuid, actor: &Actor, step_titles: &HashMap<Uuid, String>) -> ThreadSummary {
    ThreadSummary {
        id: t.id,
        class_id: t.class_id,
        step_id: t.step_id,
        step_title: t.step_id.and_then(|s| step_titles.get(&s).cloned()),
        title: t.title.clone(),
        author_id: t.author_id,
        author_name: t.author_name.clone(),
        by_teacher: t.author_id == owner_id,
        pinned: t.pinned,
        answered: t.accepted_post_id.is_some(),
        hidden: t.hidden_at.is_some(),
        post_count: t.post_count,
        last_post_at: t.last_post_at,
        created_at: t.created_at,
        mine: t.author_id == actor.id,
    }
}

// ---------- Câu hỏi ----------

pub async fn list(ctx: &AppContext, actor: &Actor, class_id: Uuid, q: ThreadQuery) -> AppResult<Page<ThreadSummary>> {
    let a = access(ctx, actor, class_id).await?;
    let page = PageRequest::new(q.page, q.page_size);
    let pattern = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| format!("%{s}%"));
    let status = q.status.as_deref().unwrap_or("all");
    let rows = sqlx::query_as::<_, ThreadRow>(crate::sql!(
        "SELECT {THREAD_COLS}, count(*) OVER () AS total FROM class_threads t JOIN users u ON u.id = t.author_id \
         WHERE t.class_id = $1 AND ($2 OR t.hidden_at IS NULL) AND ($3::uuid IS NULL OR t.step_id = $3) \
           AND ($4::text IS NULL OR t.title ILIKE $4 OR t.body ILIKE $4) \
           AND ($5 <> 'open' OR t.accepted_post_id IS NULL) AND ($5 <> 'answered' OR t.accepted_post_id IS NOT NULL) \
           AND ($5 <> 'mine' OR t.author_id = $6 OR EXISTS (SELECT 1 FROM thread_posts p WHERE p.thread_id = t.id AND p.author_id = $6)) \
         ORDER BY t.pinned DESC, t.last_post_at DESC LIMIT $7 OFFSET $8"
    ))
    .bind(class_id)
    .bind(a.teacher)
    .bind(q.step_id)
    .bind(pattern)
    .bind(status)
    .bind(actor.id)
    .bind(page.limit())
    .bind(page.offset())
    .fetch_all(&ctx.db)
    .await?;
    let total = rows.first().map_or(0, |r| r.total);
    let titles = path::published_step_titles(ctx, class_id).await?;
    let items = rows.iter().map(|t| summary(t, a.class.owner_id, actor, &titles)).collect();
    Ok(Page::new(items, total, page))
}

pub async fn get(ctx: &AppContext, actor: &Actor, thread_id: Uuid) -> AppResult<ThreadDetail> {
    let (t, a) = thread_access(ctx, actor, thread_id).await?;
    let posts = sqlx::query_as::<_, PostRow>(
        "SELECT p.id, p.author_id, u.full_name AS author_name, p.body, p.hidden_at, p.created_at, p.updated_at \
         FROM thread_posts p JOIN users u ON u.id = p.author_id WHERE p.thread_id = $1 ORDER BY p.created_at, p.id",
    )
    .bind(thread_id)
    .fetch_all(&ctx.db)
    .await?;
    let mut ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();
    ids.push(t.id);
    let reported: HashSet<Uuid> = sqlx::query_scalar::<_, Uuid>(
        "SELECT target_id FROM content_reports WHERE reporter_id = $1 AND target_id = ANY($2)",
    )
    .bind(actor.id)
    .bind(&ids)
    .fetch_all(&ctx.db)
    .await?
    .into_iter()
    .collect();
    let owner = a.class.owner_id;
    let titles = path::published_step_titles(ctx, t.class_id).await?;
    Ok(ThreadDetail {
        summary: summary(&t, owner, actor, &titles),
        body: t.body.clone(),
        updated_at: t.updated_at,
        posts: posts
            .into_iter()
            .map(|p| PostView {
                id: p.id,
                author_id: p.author_id,
                author_name: p.author_name,
                by_teacher: p.author_id == owner,
                body: if p.hidden_at.is_some() && !a.teacher { String::new() } else { p.body },
                hidden: p.hidden_at.is_some(),
                accepted: t.accepted_post_id == Some(p.id),
                mine: p.author_id == actor.id,
                reported_by_me: reported.contains(&p.id),
                created_at: p.created_at,
                updated_at: p.updated_at,
            })
            .collect(),
        can_moderate: a.teacher,
        can_accept: a.teacher || t.author_id == actor.id,
        reported_by_me: reported.contains(&t.id),
    })
}

async fn check_step(ctx: &AppContext, class_id: Uuid, step_id: Option<Uuid>) -> AppResult<()> {
    if let Some(s) = step_id
        && !path::published_step_titles(ctx, class_id).await?.contains_key(&s)
    {
        return Err(AppError::field("stepId", "Bước này không có trong lộ trình của lớp học."));
    }
    Ok(())
}

pub async fn create(ctx: &AppContext, actor: &Actor, class_id: Uuid, input: ThreadInput) -> AppResult<ThreadDetail> {
    input.validate()?;
    let a = access(ctx, actor, class_id).await?;
    if a.class.archived_at.is_some() {
        return Err(AppError::conflict("CLASS_ARCHIVED", "Lớp học đã lưu trữ."));
    }
    check_step(ctx, class_id, input.step_id).await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO class_threads (class_id, step_id, author_id, title, body) VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(class_id)
    .bind(input.step_id)
    .bind(actor.id)
    .bind(input.title.trim())
    .bind(input.body.trim())
    .fetch_one(&ctx.db)
    .await?;
    if !a.teacher {
        notifications::notify(
            &ctx.db,
            &[a.class.owner_id],
            NotificationKind::QuestionNew,
            &format!("Câu hỏi mới trong lớp học \"{}\"", a.class.title),
            input.title.trim(),
            Some(&format!("/lop-hoc/{class_id}/hoi-dap/{id}")),
        )
        .await?;
    }
    get(ctx, actor, id).await
}

pub async fn update(ctx: &AppContext, actor: &Actor, thread_id: Uuid, input: ThreadInput) -> AppResult<ThreadDetail> {
    input.validate()?;
    let (t, _) = thread_access(ctx, actor, thread_id).await?;
    if t.author_id != actor.id {
        return Err(AppError::forbidden("NOT_AUTHOR", "Chỉ người hỏi mới sửa được câu hỏi."));
    }
    check_step(ctx, t.class_id, input.step_id).await?;
    sqlx::query("UPDATE class_threads SET step_id = $2, title = $3, body = $4, updated_at = now() WHERE id = $1")
        .bind(thread_id)
        .bind(input.step_id)
        .bind(input.title.trim())
        .bind(input.body.trim())
        .execute(&ctx.db)
        .await?;
    get(ctx, actor, thread_id).await
}

/// Người hỏi xóa câu hỏi chưa có trả lời; giáo viên xóa được mọi câu hỏi.
pub async fn delete(ctx: &AppContext, actor: &Actor, thread_id: Uuid) -> AppResult<()> {
    let (t, a) = thread_access(ctx, actor, thread_id).await?;
    if !a.teacher {
        if t.author_id != actor.id {
            return Err(AppError::forbidden("NOT_AUTHOR", "Chỉ người hỏi mới xóa được câu hỏi."));
        }
        if t.post_count > 0 {
            return Err(AppError::conflict("HAS_REPLIES", "Câu hỏi đã có trả lời, không xóa được."));
        }
    }
    sqlx::query("DELETE FROM content_reports WHERE target_type = 'post' AND target_id IN (SELECT id FROM thread_posts WHERE thread_id = $1)")
        .bind(thread_id)
        .execute(&ctx.db)
        .await?;
    sqlx::query("DELETE FROM content_reports WHERE target_type = 'thread' AND target_id = $1")
        .bind(thread_id)
        .execute(&ctx.db)
        .await?;
    sqlx::query("DELETE FROM class_threads WHERE id = $1").bind(thread_id).execute(&ctx.db).await?;
    Ok(())
}

/// Giáo viên: ghim / bỏ ghim, ẩn / hiện câu hỏi.
pub async fn moderate_thread(
    ctx: &AppContext,
    actor: &Actor,
    thread_id: Uuid,
    action: &str,
) -> AppResult<ThreadDetail> {
    let (_, a) = thread_access(ctx, actor, thread_id).await?;
    if !a.teacher {
        return Err(AppError::forbidden("NOT_TEACHER", "Chỉ giáo viên chủ lớp mới làm được việc này."));
    }
    let sql = match action {
        "pin" => "UPDATE class_threads SET pinned = true WHERE id = $1 AND $2 IS NOT NULL",
        "unpin" => "UPDATE class_threads SET pinned = false WHERE id = $1 AND $2 IS NOT NULL",
        "hide" => "UPDATE class_threads SET hidden_at = now(), hidden_by = $2 WHERE id = $1",
        "unhide" => "UPDATE class_threads SET hidden_at = NULL, hidden_by = NULL WHERE id = $1 AND $2 IS NOT NULL",
        _ => return Err(AppError::bad_request("BAD_ACTION", "Thao tác không hợp lệ.")),
    };
    sqlx::query(sql).bind(thread_id).bind(actor.id).execute(&ctx.db).await?;
    if action == "hide" {
        resolve_target(ctx, actor, "thread", thread_id, "hidden").await?;
    }
    get(ctx, actor, thread_id).await
}

// ---------- Trả lời ----------

pub async fn reply(ctx: &AppContext, actor: &Actor, thread_id: Uuid, input: PostInput) -> AppResult<ThreadDetail> {
    input.validate()?;
    let (t, a) = thread_access(ctx, actor, thread_id).await?;
    if a.class.archived_at.is_some() {
        return Err(AppError::conflict("CLASS_ARCHIVED", "Lớp học đã lưu trữ."));
    }
    let mut tx = ctx.db.begin().await?;
    sqlx::query("INSERT INTO thread_posts (thread_id, author_id, body) VALUES ($1, $2, $3)")
        .bind(thread_id)
        .bind(actor.id)
        .bind(input.body.trim())
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE class_threads SET post_count = post_count + 1, last_post_at = now() WHERE id = $1")
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;
    // Người hỏi + những người đã trả lời (trừ chính người trả lời).
    let mut to: Vec<Uuid> = sqlx::query_scalar("SELECT DISTINCT author_id FROM thread_posts WHERE thread_id = $1")
        .bind(thread_id)
        .fetch_all(&mut *tx)
        .await?;
    to.push(t.author_id);
    to.retain(|u| *u != actor.id);
    let who = if a.teacher { "Giáo viên" } else { "Bạn cùng lớp" };
    notifications::notify(
        &mut *tx,
        &to,
        NotificationKind::QuestionReply,
        &format!("Có trả lời mới: {}", t.title),
        &format!("{who} vừa trả lời trong lớp học \"{}\".", a.class.title),
        Some(&format!("/lop-hoc/{}/hoi-dap/{thread_id}", t.class_id)),
    )
    .await?;
    tx.commit().await?;
    get(ctx, actor, thread_id).await
}

async fn load_post(ctx: &AppContext, post_id: Uuid) -> AppResult<(Uuid, Uuid)> {
    sqlx::query_as::<_, (Uuid, Uuid)>("SELECT thread_id, author_id FROM thread_posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or_else(|| AppError::not_found("Không tìm thấy câu trả lời."))
}

pub async fn update_post(ctx: &AppContext, actor: &Actor, post_id: Uuid, input: PostInput) -> AppResult<ThreadDetail> {
    input.validate()?;
    let (thread_id, author) = load_post(ctx, post_id).await?;
    thread_access(ctx, actor, thread_id).await?;
    if author != actor.id {
        return Err(AppError::forbidden("NOT_AUTHOR", "Chỉ người viết mới sửa được câu trả lời."));
    }
    sqlx::query("UPDATE thread_posts SET body = $2, updated_at = now() WHERE id = $1")
        .bind(post_id)
        .bind(input.body.trim())
        .execute(&ctx.db)
        .await?;
    get(ctx, actor, thread_id).await
}

/// Người viết hoặc giáo viên xóa câu trả lời.
pub async fn delete_post(ctx: &AppContext, actor: &Actor, post_id: Uuid) -> AppResult<ThreadDetail> {
    let (thread_id, author) = load_post(ctx, post_id).await?;
    let (_, a) = thread_access(ctx, actor, thread_id).await?;
    if author != actor.id && !a.teacher {
        return Err(AppError::forbidden("NOT_AUTHOR", "Chỉ người viết mới xóa được câu trả lời."));
    }
    let mut tx = ctx.db.begin().await?;
    sqlx::query("DELETE FROM content_reports WHERE target_type = 'post' AND target_id = $1")
        .bind(post_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM thread_posts WHERE id = $1").bind(post_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE class_threads SET post_count = greatest(post_count - 1, 0) WHERE id = $1")
        .bind(thread_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    get(ctx, actor, thread_id).await
}

/// Giáo viên: ẩn / hiện câu trả lời. Giáo viên hoặc người hỏi: chọn / bỏ chọn câu trả lời đúng.
pub async fn moderate_post(ctx: &AppContext, actor: &Actor, post_id: Uuid, action: &str) -> AppResult<ThreadDetail> {
    let (thread_id, _) = load_post(ctx, post_id).await?;
    let (t, a) = thread_access(ctx, actor, thread_id).await?;
    match action {
        "accept" | "unaccept" => {
            if !a.teacher && t.author_id != actor.id {
                return Err(AppError::forbidden(
                    "NOT_AUTHOR",
                    "Chỉ giáo viên hoặc người hỏi mới chọn được câu trả lời.",
                ));
            }
            let value = (action == "accept").then_some(post_id);
            sqlx::query("UPDATE class_threads SET accepted_post_id = $2 WHERE id = $1")
                .bind(thread_id)
                .bind(value)
                .execute(&ctx.db)
                .await?;
        }
        "hide" | "unhide" => {
            if !a.teacher {
                return Err(AppError::forbidden("NOT_TEACHER", "Chỉ giáo viên chủ lớp mới làm được việc này."));
            }
            let sql = if action == "hide" {
                "UPDATE thread_posts SET hidden_at = now(), hidden_by = $2 WHERE id = $1"
            } else {
                "UPDATE thread_posts SET hidden_at = NULL, hidden_by = NULL WHERE id = $1 AND $2 IS NOT NULL"
            };
            sqlx::query(sql).bind(post_id).bind(actor.id).execute(&ctx.db).await?;
            if action == "hide" {
                resolve_target(ctx, actor, "post", post_id, "hidden").await?;
            }
        }
        _ => return Err(AppError::bad_request("BAD_ACTION", "Thao tác không hợp lệ.")),
    }
    get(ctx, actor, thread_id).await
}

// ---------- Báo vi phạm ----------

/// Báo một câu hỏi / câu trả lời (không báo nội dung của chính mình; báo lại lần nữa không tính).
pub async fn report(ctx: &AppContext, actor: &Actor, input: ReportInput) -> AppResult<()> {
    input.validate()?;
    let (thread_id, author) = match input.target_type.as_str() {
        "thread" => {
            let t = load_thread(ctx, input.target_id).await?;
            (t.id, t.author_id)
        }
        "post" => load_post(ctx, input.target_id).await?,
        _ => return Err(AppError::field("targetType", "Loại nội dung không hợp lệ.")),
    };
    let (t, a) = thread_access(ctx, actor, thread_id).await?;
    if author == actor.id {
        return Err(AppError::bad_request("OWN_CONTENT", "Không báo vi phạm nội dung của chính mình."));
    }
    let inserted = sqlx::query(
        "INSERT INTO content_reports (class_id, target_type, target_id, reporter_id, reason) VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (target_type, target_id, reporter_id) DO NOTHING",
    )
    .bind(t.class_id)
    .bind(&input.target_type)
    .bind(input.target_id)
    .bind(actor.id)
    .bind(input.reason.trim())
    .execute(&ctx.db)
    .await?
    .rows_affected();
    if inserted > 0 && !a.teacher {
        notifications::notify(
            &ctx.db,
            &[a.class.owner_id],
            NotificationKind::ContentReported,
            &format!("Có nội dung bị báo vi phạm trong lớp học \"{}\"", a.class.title),
            input.reason.trim(),
            Some(&format!("/day-hoc/lop-hoc/{}?tab=hoi-dap", t.class_id)),
        )
        .await?;
    }
    Ok(())
}

/// Đóng mọi báo cáo đang mở của một nội dung.
async fn resolve_target(ctx: &AppContext, actor: &Actor, kind: &str, target: Uuid, status: &str) -> AppResult<()> {
    sqlx::query(
        "UPDATE content_reports SET status = $3, resolved_by = $4, resolved_at = now() \
         WHERE target_type = $1 AND target_id = $2 AND status = 'open'",
    )
    .bind(kind)
    .bind(target)
    .bind(status)
    .bind(actor.id)
    .execute(&ctx.db)
    .await?;
    Ok(())
}

pub async fn class_reports(
    ctx: &AppContext,
    actor: &Actor,
    class_id: Uuid,
    q: ReportQuery,
) -> AppResult<Vec<ReportView>> {
    let a = access(ctx, actor, class_id).await?;
    if !a.teacher {
        return Err(class_not_found());
    }
    let open_only = q.status.as_deref() != Some("all");
    Ok(sqlx::query_as::<_, ReportView>(
        "SELECT r.id, r.target_type, r.target_id, \
           COALESCE(p.thread_id, t.id) AS thread_id, COALESCE(tp.title, t.title) AS thread_title, \
           left(COALESCE(p.body, t.body), 300) AS excerpt, au.full_name AS author_name, ru.full_name AS reporter_name, \
           r.reason, r.status, COALESCE(p.hidden_at, t.hidden_at) IS NOT NULL AS target_hidden, r.created_at \
         FROM content_reports r \
         LEFT JOIN class_threads t ON r.target_type = 'thread' AND t.id = r.target_id \
         LEFT JOIN thread_posts p ON r.target_type = 'post' AND p.id = r.target_id \
         LEFT JOIN class_threads tp ON tp.id = p.thread_id \
         JOIN users au ON au.id = COALESCE(p.author_id, t.author_id) \
         JOIN users ru ON ru.id = r.reporter_id \
         WHERE r.class_id = $1 AND (NOT $2 OR r.status = 'open') ORDER BY r.created_at DESC LIMIT 200",
    )
    .bind(class_id)
    .bind(open_only)
    .fetch_all(&ctx.db)
    .await?)
}

/// Giáo viên xử lý một báo cáo: ẩn nội dung (đóng mọi báo cáo của nội dung đó) hoặc bỏ qua.
pub async fn resolve(ctx: &AppContext, actor: &Actor, report_id: Uuid, input: ResolveInput) -> AppResult<()> {
    let (class_id, kind, target): (Uuid, String, Uuid) =
        sqlx::query_as("SELECT class_id, target_type, target_id FROM content_reports WHERE id = $1")
            .bind(report_id)
            .fetch_optional(&ctx.db)
            .await?
            .ok_or_else(|| AppError::not_found("Không tìm thấy báo cáo."))?;
    let a = access(ctx, actor, class_id).await?;
    if !a.teacher {
        return Err(AppError::not_found("Không tìm thấy báo cáo."));
    }
    match input.action.as_str() {
        "hide" => {
            let sql = if kind == "thread" {
                "UPDATE class_threads SET hidden_at = COALESCE(hidden_at, now()), hidden_by = $2 WHERE id = $1"
            } else {
                "UPDATE thread_posts SET hidden_at = COALESCE(hidden_at, now()), hidden_by = $2 WHERE id = $1"
            };
            sqlx::query(sql).bind(target).bind(actor.id).execute(&ctx.db).await?;
            resolve_target(ctx, actor, &kind, target, "hidden").await
        }
        "dismiss" => resolve_target(ctx, actor, &kind, target, "dismissed").await,
        _ => Err(AppError::field("action", "Thao tác không hợp lệ.")),
    }
}

/// Số câu hỏi chưa có câu trả lời đúng + số báo cáo đang mở (giáo viên).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QaCounts {
    pub open_threads: i64,
    pub open_reports: i64,
}

pub async fn counts(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<QaCounts> {
    let a = access(ctx, actor, class_id).await?;
    let (open_threads, open_reports): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM class_threads WHERE class_id = $1 AND hidden_at IS NULL AND accepted_post_id IS NULL), \
                (SELECT count(*) FROM content_reports WHERE class_id = $1 AND status = 'open' AND $2)",
    )
    .bind(class_id)
    .bind(a.teacher)
    .fetch_one(&ctx.db)
    .await?;
    Ok(QaCounts { open_threads, open_reports })
}
