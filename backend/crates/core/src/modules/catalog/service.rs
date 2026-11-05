//! Nghiệp vụ master data.

use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{
        CreateItemRequest, CreateSubjectRequest, GradeResponse, LevelResponse, MasterDataResponse, PendingItemResponse,
        SubjectResponse, TagResponse, UpdateItemRequest,
    },
    model::{Grade, QuestionLevel, Subject, Tag},
    repository as repo,
};
use crate::{
    context::AppContext,
    db::is_unique_violation,
    error::{AppError, AppResult},
    modules::audit::{model::AuditEntry, service as audit},
    security::{Actor, Permission},
    slug::slugify,
};

fn duplicate_name() -> AppError {
    AppError::conflict("DUPLICATE_NAME", "Đã có mục trùng tên.")
}

/// Chuẩn hóa tên và tạo slug; tên chỉ toàn ký tự đặc biệt thì báo lỗi.
fn name_and_slug(name: &str) -> AppResult<(String, String)> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let slug = slugify(&name);
    if slug.is_empty() {
        return Err(AppError::field("name", "Tên phải có chữ hoặc số."));
    }
    Ok((name, slug))
}

fn map_dup(e: sqlx::Error, constraints: &[&str]) -> AppError {
    if constraints.iter().any(|c| is_unique_violation(&e, c)) { duplicate_name() } else { e.into() }
}

async fn subject_or_404(ctx: &AppContext, id: Uuid, active_only: bool) -> AppResult<Subject> {
    repo::find_subject(&ctx.db, id)
        .await?
        .filter(|s| !active_only || s.is_active)
        .ok_or_else(|| AppError::not_found("Không tìm thấy môn học."))
}

async fn grade_or_404(ctx: &AppContext, id: Uuid) -> AppResult<Grade> {
    repo::find_grade(&ctx.db, id)
        .await?
        .filter(|g| g.merged_into.is_none())
        .ok_or_else(|| AppError::not_found("Không tìm thấy lớp."))
}

async fn tag_or_404(ctx: &AppContext, id: Uuid) -> AppResult<Tag> {
    repo::find_tag(&ctx.db, id)
        .await?
        .filter(|t| t.merged_into.is_none())
        .ok_or_else(|| AppError::not_found("Không tìm thấy danh mục."))
}

async fn log(
    ctx: &AppContext,
    actor: &Actor,
    action: &str,
    target_type: &str,
    id: Uuid,
    data: serde_json::Value,
    ip: Option<&str>,
) -> AppResult<()> {
    audit::record(&ctx.db, AuditEntry { actor_id: Some(actor.id), action, target_type, target_id: Some(id), data, ip })
        .await
}

// ---------- Đọc (mọi người) ----------

/// Môn học đang dùng, lớp người xem thấy được, mức độ câu hỏi.
pub async fn master_data(ctx: &AppContext, viewer: Option<&Actor>) -> AppResult<MasterDataResponse> {
    let subjects = repo::list_subjects(&ctx.db, false).await?;
    let grades = repo::list_visible_grades(&ctx.db, viewer.map(|a| a.id)).await?;
    Ok(MasterDataResponse {
        subjects: subjects.into_iter().map(Into::into).collect(),
        grades: grades.into_iter().map(Into::into).collect(),
        levels: QuestionLevel::ALL.iter().map(|l| LevelResponse { code: *l, name: l.label() }).collect(),
    })
}

pub async fn list_tags(ctx: &AppContext, viewer: Option<&Actor>, subject_id: Uuid) -> AppResult<Vec<TagResponse>> {
    subject_or_404(ctx, subject_id, true).await?;
    let tags = repo::list_visible_tags(&ctx.db, subject_id, viewer.map(|a| a.id)).await?;
    Ok(tags.into_iter().map(Into::into).collect())
}

// ---------- Thêm lớp / danh mục (admin: dùng chung; giáo viên: lớp riêng) ----------

/// Trả về `(mục, true nếu vừa tạo)`. Giáo viên thêm tên đã có (chung hoặc của mình) thì nhận lại mục đó.
pub async fn create_grade(
    ctx: &AppContext,
    actor: &Actor,
    req: CreateItemRequest,
    ip: Option<&str>,
) -> AppResult<(GradeResponse, bool)> {
    req.validate()?;
    let (name, slug) = name_and_slug(&req.name)?;
    if actor.can(Permission::MasterDataManage) {
        let sort = req.sort_order.unwrap_or(100);
        let grade = repo::insert_grade(&ctx.db, &name, &slug, sort, None)
            .await
            .map_err(|e| map_dup(e, &["grades_shared_slug_key"]))?;
        log(ctx, actor, "grade.create", "grade", grade.id, json!({ "name": grade.name }), ip).await?;
        return Ok((grade.into(), true));
    }
    actor.require(Permission::MasterDataPropose)?;
    if let Some(existing) = repo::find_visible_grade_by_slug(&ctx.db, &slug, actor.id).await? {
        return Ok((existing.into(), false));
    }
    match repo::insert_grade(&ctx.db, &name, &slug, 1000, Some(actor.id)).await {
        Ok(g) => Ok((g.into(), true)),
        // Hai request song song cùng tên: lấy lại mục vừa được tạo.
        Err(e) if is_unique_violation(&e, "grades_owner_slug_key") => {
            let g = repo::find_visible_grade_by_slug(&ctx.db, &slug, actor.id).await?.ok_or_else(duplicate_name)?;
            Ok((g.into(), false))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn create_tag(
    ctx: &AppContext,
    actor: &Actor,
    subject_id: Uuid,
    req: CreateItemRequest,
    ip: Option<&str>,
) -> AppResult<(TagResponse, bool)> {
    req.validate()?;
    let (name, slug) = name_and_slug(&req.name)?;
    if actor.can(Permission::MasterDataManage) {
        subject_or_404(ctx, subject_id, false).await?;
        let sort = req.sort_order.unwrap_or(100);
        let tag = repo::insert_tag(&ctx.db, subject_id, &name, &slug, sort, None)
            .await
            .map_err(|e| map_dup(e, &["tags_shared_slug_key"]))?;
        log(ctx, actor, "tag.create", "tag", tag.id, json!({ "name": tag.name, "subjectId": subject_id }), ip).await?;
        return Ok((tag.into(), true));
    }
    actor.require(Permission::MasterDataPropose)?;
    subject_or_404(ctx, subject_id, true).await?;
    if let Some(existing) = repo::find_visible_tag_by_slug(&ctx.db, subject_id, &slug, actor.id).await? {
        return Ok((existing.into(), false));
    }
    match repo::insert_tag(&ctx.db, subject_id, &name, &slug, 1000, Some(actor.id)).await {
        Ok(t) => Ok((t.into(), true)),
        Err(e) if is_unique_violation(&e, "tags_owner_slug_key") => {
            let t = repo::find_visible_tag_by_slug(&ctx.db, subject_id, &slug, actor.id)
                .await?
                .ok_or_else(duplicate_name)?;
            Ok((t.into(), false))
        }
        Err(e) => Err(e.into()),
    }
}

// ---------- Admin: môn học ----------

pub async fn admin_list_subjects(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<SubjectResponse>> {
    actor.require(Permission::MasterDataManage)?;
    Ok(repo::list_subjects(&ctx.db, true).await?.into_iter().map(Into::into).collect())
}

pub async fn create_subject(
    ctx: &AppContext,
    actor: &Actor,
    req: CreateSubjectRequest,
    ip: Option<&str>,
) -> AppResult<SubjectResponse> {
    actor.require(Permission::MasterDataManage)?;
    req.validate()?;
    let (name, slug) = name_and_slug(&req.name)?;
    let subject = repo::insert_subject(&ctx.db, &name, &slug, req.sort_order.unwrap_or(100))
        .await
        .map_err(|e| map_dup(e, &["subjects_slug_key"]))?;
    log(ctx, actor, "subject.create", "subject", subject.id, json!({ "name": subject.name }), ip).await?;
    Ok(subject.into())
}

pub async fn update_subject(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    req: UpdateItemRequest,
    ip: Option<&str>,
) -> AppResult<SubjectResponse> {
    actor.require(Permission::MasterDataManage)?;
    req.validate()?;
    subject_or_404(ctx, id, false).await?;
    let ns = req.name.as_deref().map(name_and_slug).transpose()?;
    let subject = repo::update_subject(
        &ctx.db,
        id,
        ns.as_ref().map(|(n, s)| (n.as_str(), s.as_str())),
        req.sort_order,
        req.is_active,
    )
    .await
    .map_err(|e| map_dup(e, &["subjects_slug_key"]))?;
    log(
        ctx,
        actor,
        "subject.update",
        "subject",
        id,
        json!({ "name": req.name, "sortOrder": req.sort_order, "isActive": req.is_active }),
        ip,
    )
    .await?;
    Ok(subject.into())
}

// ---------- Admin: lớp ----------

pub async fn admin_list_grades(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<GradeResponse>> {
    actor.require(Permission::MasterDataManage)?;
    Ok(repo::list_shared_grades(&ctx.db).await?.into_iter().map(Into::into).collect())
}

pub async fn update_grade(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    req: UpdateItemRequest,
    ip: Option<&str>,
) -> AppResult<GradeResponse> {
    actor.require(Permission::MasterDataManage)?;
    req.validate()?;
    let grade = grade_or_404(ctx, id).await?;
    if grade.is_system {
        return Err(AppError::conflict("SYSTEM_ITEM", "Lớp 1–12 là mục hệ thống, không sửa được."));
    }
    let ns = req.name.as_deref().map(name_and_slug).transpose()?;
    let updated = repo::update_grade(
        &ctx.db,
        id,
        ns.as_ref().map(|(n, s)| (n.as_str(), s.as_str())),
        req.sort_order,
        req.is_active,
    )
    .await
    .map_err(|e| map_dup(e, &["grades_shared_slug_key", "grades_owner_slug_key"]))?;
    log(
        ctx,
        actor,
        "grade.update",
        "grade",
        id,
        json!({ "name": req.name, "sortOrder": req.sort_order, "isActive": req.is_active }),
        ip,
    )
    .await?;
    Ok(updated.into())
}

pub async fn promote_grade(ctx: &AppContext, actor: &Actor, id: Uuid, ip: Option<&str>) -> AppResult<GradeResponse> {
    actor.require(Permission::MasterDataManage)?;
    let grade = grade_or_404(ctx, id).await?;
    if grade.owner_id.is_none() {
        return Err(AppError::conflict("ALREADY_SHARED", "Mục này đã dùng chung."));
    }
    let promoted = repo::promote_grade(&ctx.db, id).await.map_err(|e| {
        if is_unique_violation(&e, "grades_shared_slug_key") {
            AppError::conflict("DUPLICATE_NAME", "Đã có lớp dùng chung trùng tên — hãy gộp vào lớp đó.")
        } else {
            e.into()
        }
    })?;
    log(ctx, actor, "grade.promote", "grade", id, json!({ "name": grade.name, "ownerId": grade.owner_id }), ip).await?;
    Ok(promoted.into())
}

/// Gộp lớp `id` vào lớp dùng chung `into_id`.
/// Khi các module sau có bảng tham chiếu tới lớp (lớp học, câu hỏi…), phần chuyển tham chiếu đặt ở đây.
pub async fn merge_grade(ctx: &AppContext, actor: &Actor, id: Uuid, into_id: Uuid, ip: Option<&str>) -> AppResult<()> {
    actor.require(Permission::MasterDataManage)?;
    if id == into_id {
        return Err(AppError::field("intoId", "Không thể gộp một mục vào chính nó."));
    }
    let source = grade_or_404(ctx, id).await?;
    if source.is_system {
        return Err(AppError::conflict("SYSTEM_ITEM", "Lớp 1–12 là mục hệ thống, không gộp được."));
    }
    let target = grade_or_404(ctx, into_id).await?;
    if target.owner_id.is_some() || !target.is_active {
        return Err(AppError::field("intoId", "Chỉ gộp được vào một lớp dùng chung đang hoạt động."));
    }
    let mut tx = ctx.db.begin().await?;
    repo::merge_grade(&mut *tx, id, into_id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "grade.merge",
            target_type: "grade",
            target_id: Some(id),
            data: json!({ "from": source.name, "into": target.name, "intoId": into_id }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

// ---------- Admin: danh mục ----------

pub async fn admin_list_tags(ctx: &AppContext, actor: &Actor, subject_id: Uuid) -> AppResult<Vec<TagResponse>> {
    actor.require(Permission::MasterDataManage)?;
    subject_or_404(ctx, subject_id, false).await?;
    Ok(repo::list_shared_tags(&ctx.db, subject_id).await?.into_iter().map(Into::into).collect())
}

pub async fn update_tag(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    req: UpdateItemRequest,
    ip: Option<&str>,
) -> AppResult<TagResponse> {
    actor.require(Permission::MasterDataManage)?;
    req.validate()?;
    tag_or_404(ctx, id).await?;
    let ns = req.name.as_deref().map(name_and_slug).transpose()?;
    let updated = repo::update_tag(
        &ctx.db,
        id,
        ns.as_ref().map(|(n, s)| (n.as_str(), s.as_str())),
        req.sort_order,
        req.is_active,
    )
    .await
    .map_err(|e| map_dup(e, &["tags_shared_slug_key", "tags_owner_slug_key"]))?;
    log(
        ctx,
        actor,
        "tag.update",
        "tag",
        id,
        json!({ "name": req.name, "sortOrder": req.sort_order, "isActive": req.is_active }),
        ip,
    )
    .await?;
    Ok(updated.into())
}

pub async fn promote_tag(ctx: &AppContext, actor: &Actor, id: Uuid, ip: Option<&str>) -> AppResult<TagResponse> {
    actor.require(Permission::MasterDataManage)?;
    let tag = tag_or_404(ctx, id).await?;
    if tag.owner_id.is_none() {
        return Err(AppError::conflict("ALREADY_SHARED", "Mục này đã dùng chung."));
    }
    let promoted = repo::promote_tag(&ctx.db, id).await.map_err(|e| {
        if is_unique_violation(&e, "tags_shared_slug_key") {
            AppError::conflict(
                "DUPLICATE_NAME",
                "Môn này đã có danh mục dùng chung trùng tên — hãy gộp vào danh mục đó.",
            )
        } else {
            e.into()
        }
    })?;
    log(ctx, actor, "tag.promote", "tag", id, json!({ "name": tag.name, "ownerId": tag.owner_id }), ip).await?;
    Ok(promoted.into())
}

pub async fn merge_tag(ctx: &AppContext, actor: &Actor, id: Uuid, into_id: Uuid, ip: Option<&str>) -> AppResult<()> {
    actor.require(Permission::MasterDataManage)?;
    if id == into_id {
        return Err(AppError::field("intoId", "Không thể gộp một mục vào chính nó."));
    }
    let source = tag_or_404(ctx, id).await?;
    let target = tag_or_404(ctx, into_id).await?;
    if target.owner_id.is_some() || !target.is_active || target.subject_id != source.subject_id {
        return Err(AppError::field("intoId", "Chỉ gộp được vào danh mục dùng chung, đang hoạt động, cùng môn."));
    }
    let mut tx = ctx.db.begin().await?;
    repo::merge_tag(&mut *tx, id, into_id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "tag.merge",
            target_type: "tag",
            target_id: Some(id),
            data: json!({ "from": source.name, "into": target.name, "intoId": into_id }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn pending(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<PendingItemResponse>> {
    actor.require(Permission::MasterDataManage)?;
    Ok(repo::list_pending(&ctx.db).await?.into_iter().map(Into::into).collect())
}

// ---------- Dùng cho module khác ----------

/// Kiểm tra mọi môn trong danh sách tồn tại và đang dùng.
pub async fn ensure_subjects_active(ctx: &AppContext, ids: &[Uuid]) -> AppResult<()> {
    let count = repo::subjects_exist(&ctx.db, ids).await?;
    if count != ids.len() as i64 {
        return Err(AppError::field("subjectIds", "Có môn học không tồn tại hoặc đã ẩn."));
    }
    Ok(())
}

/// Kiểm tra mọi lớp trong danh sách tồn tại và người dùng `viewer` thấy được.
pub async fn ensure_grades_visible(ctx: &AppContext, ids: &[Uuid], viewer: Uuid) -> AppResult<()> {
    let count = repo::count_visible_grades(&ctx.db, ids, viewer).await?;
    if count != ids.len() as i64 {
        return Err(AppError::field("gradeIds", "Có lớp không tồn tại."));
    }
    Ok(())
}

/// Tên các môn theo id (giữ thứ tự sort của môn).
pub async fn subject_names(ctx: &AppContext, ids: &[Uuid]) -> AppResult<Vec<String>> {
    let all = repo::list_subjects(&ctx.db, true).await?;
    Ok(all.into_iter().filter(|s| ids.contains(&s.id)).map(|s| s.name).collect())
}

/// Kiểm tra mọi danh mục thuộc môn `subject_id` và `viewer` thấy được.
pub async fn ensure_tags_visible(ctx: &AppContext, subject_id: Uuid, ids: &[Uuid], viewer: Uuid) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut unique = ids.to_vec();
    unique.sort();
    unique.dedup();
    let count = repo::count_visible_tags(&ctx.db, subject_id, &unique, viewer).await?;
    if count != unique.len() as i64 {
        return Err(AppError::field("tagIds", "Có danh mục không tồn tại hoặc không thuộc môn học đã chọn."));
    }
    Ok(())
}
