//! Nghiệp vụ hồ sơ giáo viên.

use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{ApplicationListQuery, ApplicationResponse, RejectApplicationRequest, SubmitApplicationRequest},
    model::ApplicationStatus,
    repository::{self as repo, NewApplication},
};
use crate::{
    context::AppContext,
    db::is_unique_violation,
    error::{AppError, AppResult},
    jobs::Job,
    mail,
    modules::{
        audit::{model::AuditEntry, service as audit},
        auth::service as auth,
        catalog::service as catalog,
        users::{
            model::{UserRole, UserStatus},
            service as users,
        },
    },
    pagination::{Page, PageRequest},
    security::{Actor, Permission},
};

fn dedup(ids: &[Uuid]) -> Vec<Uuid> {
    let mut out: Vec<Uuid> = Vec::with_capacity(ids.len());
    for id in ids {
        if !out.contains(id) {
            out.push(*id);
        }
    }
    out
}

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy hồ sơ.")
}

pub async fn submit(ctx: &AppContext, actor: &Actor, req: SubmitApplicationRequest) -> AppResult<ApplicationResponse> {
    if actor.role == UserRole::Teacher {
        return Err(AppError::conflict("ALREADY_TEACHER", "Bạn đã là giáo viên."));
    }
    actor.require(Permission::TeacherApplicationsSubmit)?;
    req.validate()?;
    let user = users::find_by_id(&ctx.db, actor.id)
        .await?
        .ok_or_else(|| AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập."))?;
    if user.status != UserStatus::Active {
        return Err(AppError::forbidden("EMAIL_NOT_VERIFIED", "Tài khoản cần được xác minh email trước."));
    }
    let subject_ids = dedup(&req.subject_ids);
    let grade_ids = dedup(&req.grade_ids);
    catalog::ensure_subjects_active(ctx, &subject_ids).await?;
    if !grade_ids.is_empty() {
        catalog::ensure_grades_visible(ctx, &grade_ids, actor.id).await?;
    }

    let id = repo::insert(
        &ctx.db,
        &NewApplication {
            user_id: actor.id,
            subject_ids: &subject_ids,
            grade_ids: &grade_ids,
            bio: req.bio.trim(),
            experience: req.experience.trim(),
        },
    )
    .await
    .map_err(|e| {
        if is_unique_violation(&e, "teacher_applications_one_pending_key") {
            AppError::conflict("APPLICATION_PENDING", "Bạn đã có một hồ sơ đang chờ duyệt.")
        } else {
            e.into()
        }
    })?;
    let row = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    Ok(row.into())
}

pub async fn my_applications(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<ApplicationResponse>> {
    Ok(repo::list_for_user(&ctx.db, actor.id).await?.into_iter().map(Into::into).collect())
}

pub async fn admin_list(
    ctx: &AppContext,
    actor: &Actor,
    q: ApplicationListQuery,
) -> AppResult<Page<ApplicationResponse>> {
    actor.require(Permission::TeacherApplicationsReview)?;
    let page = PageRequest::new(q.page, q.page_size);
    let total = repo::count(&ctx.db, q.status).await?;
    let rows = repo::list(&ctx.db, q.status, page.limit(), page.offset()).await?;
    Ok(Page::new(rows.into_iter().map(Into::into).collect(), total, page))
}

fn not_pending() -> AppError {
    AppError::conflict("NOT_PENDING", "Hồ sơ này đã được xử lý.")
}

/// Duyệt: user thành giáo viên, được dạy các môn trong hồ sơ; phiên cũ bị thu hồi để token mới mang vai trò mới.
pub async fn approve(ctx: &AppContext, actor: &Actor, id: Uuid, ip: Option<&str>) -> AppResult<ApplicationResponse> {
    actor.require(Permission::TeacherApplicationsReview)?;
    let mut tx = ctx.db.begin().await?;
    match repo::lock_status(&mut *tx, id).await? {
        None => return Err(not_found()),
        Some(ApplicationStatus::Pending) => {}
        Some(_) => return Err(not_pending()),
    }
    let app = repo::find(&mut *tx, id).await?.ok_or_else(not_found)?;
    repo::set_reviewed(&mut *tx, id, ApplicationStatus::Approved, actor.id, None).await?;
    let user = users::find_by_id(&mut *tx, app.user_id).await?.ok_or_else(not_found)?;
    if user.role == UserRole::Student {
        users::set_role(&mut *tx, user.id, UserRole::Teacher).await?;
        auth::revoke_all_sessions(&mut *tx, user.id).await?;
    }
    repo::add_teacher_subjects(&mut *tx, user.id, &app.subject_ids).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "teacher_application.approve",
            target_type: "user",
            target_id: Some(user.id),
            data: json!({ "applicationId": id, "subjectIds": app.subject_ids }),
            ip,
        },
    )
    .await?;
    let row = repo::find(&mut *tx, id).await?.ok_or_else(not_found)?;
    tx.commit().await?;

    if let Err(e) =
        ctx.jobs.publish(Job::SendEmail(mail::teacher_application_approved(&user.email, &user.full_name))).await
    {
        tracing::error!(error = %e, "không gửi được email duyệt hồ sơ");
    }
    Ok(row.into())
}

pub async fn reject(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    req: RejectApplicationRequest,
    ip: Option<&str>,
) -> AppResult<ApplicationResponse> {
    actor.require(Permission::TeacherApplicationsReview)?;
    req.validate()?;
    let reason = req.reason.trim();
    let mut tx = ctx.db.begin().await?;
    match repo::lock_status(&mut *tx, id).await? {
        None => return Err(not_found()),
        Some(ApplicationStatus::Pending) => {}
        Some(_) => return Err(not_pending()),
    }
    repo::set_reviewed(&mut *tx, id, ApplicationStatus::Rejected, actor.id, Some(reason)).await?;
    let row = repo::find(&mut *tx, id).await?.ok_or_else(not_found)?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "teacher_application.reject",
            target_type: "user",
            target_id: Some(row.user_id),
            data: json!({ "applicationId": id, "reason": reason }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;

    if let Err(e) =
        ctx.jobs.publish(Job::SendEmail(mail::teacher_application_rejected(&row.email, &row.full_name, reason))).await
    {
        tracing::error!(error = %e, "không gửi được email từ chối hồ sơ");
    }
    Ok(row.into())
}

/// Môn giáo viên được dạy (G2 dùng khi tạo lớp học).
pub async fn teacher_subject_ids(ctx: &AppContext, teacher_id: Uuid) -> AppResult<Vec<Uuid>> {
    Ok(repo::teacher_subject_ids(&ctx.db, teacher_id).await?)
}
