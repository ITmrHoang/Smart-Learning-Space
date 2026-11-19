//! Nghiệp vụ thư viện bài. Xem mô tả ở `mod.rs`.

use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{LessonInput, LessonQuery, LessonResponse, LessonSummary, PublishedLesson, RevisionSummary},
    model::{Lesson, LessonRef},
    repository::{self as repo, LessonFields, LessonFilter},
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        audit::{model::AuditEntry, service as audit},
        catalog::service as catalog,
        files::{self, AttachmentRef},
        teachers::service as teachers,
    },
    pagination::{Page, PageRequest},
    security::{Actor, Permission},
};

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy bài học.")
}

/// Bài của chính người gọi (người khác → 404 để không lộ bài tồn tại).
async fn load_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<Lesson> {
    actor.require(Permission::TeachingManage)?;
    let lesson = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if lesson.owner_id != actor.id || lesson.archived_at.is_some() {
        return Err(not_found());
    }
    Ok(lesson)
}

/// Kiểm tra môn (được dạy), lớp, danh mục.
async fn validate_input(ctx: &AppContext, actor: &Actor, input: &LessonInput) -> AppResult<()> {
    input.validate()?;
    teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    if let Some(grade) = input.grade_id {
        catalog::ensure_grades_visible(ctx, &[grade], actor.id)
            .await
            .map_err(|_| AppError::field("gradeId", "Lớp không tồn tại."))?;
    }
    catalog::ensure_tags_visible(ctx, input.subject_id, &input.tag_ids, actor.id).await
}

/// Tài liệu đính kèm lưu trong jsonb.
fn refs_of(value: &serde_json::Value) -> Vec<AttachmentRef> {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

/// Kiểm tra tài liệu đính kèm (file của chính giáo viên, đã upload xong) → jsonb để lưu.
async fn attachments_value(ctx: &AppContext, actor: &Actor, input: &LessonInput) -> AppResult<serde_json::Value> {
    let refs = files::check_attachments(ctx, actor.id, &input.attachment_ids).await?;
    serde_json::to_value(refs).map_err(AppError::internal)
}

fn fields<'a>(input: &'a LessonInput, attachments: &'a serde_json::Value) -> LessonFields<'a> {
    LessonFields {
        attachments,
        subject_id: input.subject_id,
        grade_id: input.grade_id,
        title: input.title.trim(),
        summary: input.summary.trim(),
        content: &input.content,
        video_url: input.video_url.as_deref().map(str::trim).filter(|v| !v.is_empty()),
        duration_minutes: input.duration_minutes,
    }
}

pub async fn list(ctx: &AppContext, actor: &Actor, query: LessonQuery) -> AppResult<Page<LessonSummary>> {
    actor.require(Permission::TeachingManage)?;
    let page = PageRequest::new(query.page, query.page_size);
    let q = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    let filter = LessonFilter { owner_id: actor.id, q, subject_id: query.subject_id, status: query.status.as_deref() };
    let items = repo::list(&ctx.db, &filter, page.limit(), page.offset()).await?;
    let total = repo::count(&ctx.db, &filter).await?;
    Ok(Page::new(items.into_iter().map(Into::into).collect(), total, page))
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: LessonInput) -> AppResult<LessonResponse> {
    validate_input(ctx, actor, &input).await?;
    let attachments = attachments_value(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    let id = repo::insert(&mut *tx, actor.id, &fields(&input, &attachments)).await?;
    repo::set_tags(&mut tx, id, &input.tag_ids).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<LessonResponse> {
    let lesson = load_own(ctx, actor, id).await?;
    let refs = refs_of(&lesson.attachments);
    let mut res: LessonResponse = lesson.into();
    res.attachments = files::attachment_views(ctx, &refs).await?;
    Ok(res)
}

pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: LessonInput) -> AppResult<LessonResponse> {
    load_own(ctx, actor, id).await?;
    validate_input(ctx, actor, &input).await?;
    let attachments = attachments_value(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    repo::update(&mut *tx, id, &fields(&input, &attachments)).await?;
    repo::set_tags(&mut tx, id, &input.tag_ids).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

/// Ẩn bài khỏi thư viện. Lớp học đang dùng bài vẫn giữ bản đã xuất bản.
pub async fn archive(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    load_own(ctx, actor, id).await?;
    repo::archive(&ctx.db, id).await?;
    Ok(())
}

/// Xuất bản bản nháp hiện tại thành phiên bản mới.
pub async fn publish(ctx: &AppContext, actor: &Actor, id: Uuid, ip: Option<&str>) -> AppResult<LessonResponse> {
    let lesson = load_own(ctx, actor, id).await?;
    if lesson.published_revision_id.is_some() && !lesson.has_changes {
        return Err(AppError::conflict("NOTHING_TO_PUBLISH", "Bài không có thay đổi nào cần xuất bản."));
    }
    let mut tx = ctx.db.begin().await?;
    let version = repo::publish(&mut tx, id, actor.id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "lesson.publish",
            target_type: "lesson",
            target_id: Some(id),
            data: json!({ "version": version, "title": lesson.title }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

pub async fn revisions(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<Vec<RevisionSummary>> {
    load_own(ctx, actor, id).await?;
    Ok(repo::revisions(&ctx.db, id)
        .await?
        .into_iter()
        .map(|r| RevisionSummary { version: r.version, title: r.title, published_at: r.published_at })
        .collect())
}

// ---------- Dùng cho module lớp học / lộ trình ----------

pub async fn refs(ctx: &AppContext, ids: &[Uuid]) -> AppResult<Vec<LessonRef>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(repo::refs(&ctx.db, ids).await?)
}

/// Bản xuất bản mới nhất của bài (học sinh đọc). `None` nếu bài chưa xuất bản.
pub async fn published(ctx: &AppContext, lesson_id: Uuid) -> AppResult<Option<PublishedLesson>> {
    let Some(lesson) = repo::find(&ctx.db, lesson_id).await? else {
        return Ok(None);
    };
    let Some(rev_id) = lesson.published_revision_id else {
        return Ok(None);
    };
    let Some(rev) = repo::find_revision(&ctx.db, rev_id).await? else {
        return Ok(None);
    };
    let refs = refs_of(&rev.attachments);
    let mut out: PublishedLesson = rev.into();
    out.attachments = files::attachment_views(ctx, &refs).await?;
    Ok(Some(out))
}
