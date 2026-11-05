//! Lớp học + ghi danh (M8). Lộ trình + tiến độ ở `path.rs`.

use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{
        AddMemberRequest, ClassCard, ClassDetail, ClassInput, ClassQuery, ClassResponse, CopyRequest, EnrollResponse,
        JoinRequest, MemberQuery, MemberResponse, MyClass, OutlineStep, ProgressSummary,
    },
    model::{ClassRow, Membership},
    path,
    repository::{self as repo, ClassFields, ClassFilter, ClassScope},
};
use crate::{
    context::AppContext,
    db::is_unique_violation,
    error::{AppError, AppResult},
    modules::{
        audit::{model::AuditEntry, service as audit},
        catalog::service as catalog,
        lessons::service as lessons,
        teachers::service as teachers,
        users::service as users,
    },
    pagination::{Page, PageRequest},
    security::{Actor, Permission, token::random_code},
};

pub(super) fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy lớp học.")
}

/// Lớp học của chính giáo viên (cả lớp đã lưu trữ).
pub(super) async fn load_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<ClassRow> {
    actor.require(Permission::TeachingManage)?;
    let class = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if class.owner_id != actor.id {
        return Err(not_found());
    }
    Ok(class)
}

async fn validate_input(ctx: &AppContext, actor: &Actor, input: &ClassInput) -> AppResult<()> {
    input.validate()?;
    teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    let mut grades = input.grade_ids.clone();
    grades.sort();
    grades.dedup();
    catalog::ensure_grades_visible(ctx, &grades, actor.id).await?;
    catalog::ensure_tags_visible(ctx, input.subject_id, &input.tag_ids, actor.id).await
}

fn fields(input: &ClassInput) -> ClassFields<'_> {
    ClassFields {
        subject_id: input.subject_id,
        title: input.title.trim(),
        description: input.description.trim(),
        mode: input.mode.as_str(),
        visibility: input.visibility.as_str(),
        enrollment: input.enrollment.as_str(),
    }
}

/// Thêm lớp học với mã mời ngẫu nhiên (thử lại nếu trùng mã).
async fn insert_with_code(
    conn: &mut sqlx::PgConnection,
    owner_id: Uuid,
    f: &ClassFields<'_>,
    copied_from: Option<Uuid>,
) -> AppResult<Uuid> {
    for _ in 0..5 {
        let code = random_code();
        match repo::insert(&mut *conn, owner_id, f, &code, copied_from).await {
            Ok(id) => return Ok(id),
            Err(e) if is_unique_violation(&e, "classes_invite_code_key") => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(AppError::internal("không tạo được mã mời duy nhất"))
}

// ---------- Giáo viên ----------

pub async fn list_own(ctx: &AppContext, actor: &Actor, query: ClassQuery) -> AppResult<Page<ClassResponse>> {
    actor.require(Permission::TeachingManage)?;
    let page = PageRequest::new(query.page, query.page_size);
    let filter = ClassFilter {
        scope: ClassScope::Owner(actor.id, query.archived),
        q: query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()),
        subject_id: query.subject_id,
        grade_id: query.grade_id,
    };
    let items = repo::list(&ctx.db, &filter, page.limit(), page.offset()).await?;
    let total = repo::count(&ctx.db, &filter).await?;
    Ok(Page::new(items.into_iter().map(Into::into).collect(), total, page))
}

pub async fn get_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<ClassResponse> {
    Ok(load_own(ctx, actor, id).await?.into())
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: ClassInput, ip: Option<&str>) -> AppResult<ClassResponse> {
    actor.require(Permission::TeachingManage)?;
    validate_input(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    let id = insert_with_code(&mut tx, actor.id, &fields(&input), None).await?;
    repo::set_grades_tags(&mut tx, id, &input.grade_ids, &input.tag_ids).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "class.create",
            target_type: "class",
            target_id: Some(id),
            data: json!({ "title": input.title.trim() }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    get_own(ctx, actor, id).await
}

pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: ClassInput) -> AppResult<ClassResponse> {
    load_own(ctx, actor, id).await?;
    validate_input(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    repo::update(&mut *tx, id, &fields(&input)).await?;
    repo::set_grades_tags(&mut tx, id, &input.grade_ids, &input.tag_ids).await?;
    tx.commit().await?;
    get_own(ctx, actor, id).await
}

/// Lưu trữ (ẩn khỏi danh sách, học sinh không vào học được nữa) / khôi phục.
pub async fn set_archived(ctx: &AppContext, actor: &Actor, id: Uuid, archived: bool) -> AppResult<ClassResponse> {
    load_own(ctx, actor, id).await?;
    repo::set_archived(&ctx.db, id, archived).await?;
    get_own(ctx, actor, id).await
}

/// Đổi mã mời (mã cũ hết dùng được).
pub async fn regenerate_code(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<ClassResponse> {
    load_own(ctx, actor, id).await?;
    for _ in 0..5 {
        match repo::set_invite_code(&ctx.db, id, &random_code()).await {
            Ok(()) => return get_own(ctx, actor, id).await,
            Err(e) if is_unique_violation(&e, "classes_invite_code_key") => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Err(AppError::internal("không tạo được mã mời duy nhất"))
}

/// Tạo lớp học mới từ lớp học cũ: chép thông tin + bản nháp lộ trình (không chép học sinh, tiến độ).
pub async fn copy(ctx: &AppContext, actor: &Actor, id: Uuid, req: CopyRequest) -> AppResult<ClassResponse> {
    req.validate()?;
    let src = load_own(ctx, actor, id).await?;
    let title = req.title.map(|t| t.trim().to_string()).unwrap_or_else(|| format!("{} (bản sao)", src.title));
    let f = ClassFields {
        subject_id: src.subject_id,
        title: &title,
        description: &src.description,
        mode: &src.mode,
        visibility: &src.visibility,
        enrollment: &src.enrollment,
    };
    let steps = repo::draft_steps(&ctx.db, id).await?;
    let mut tx = ctx.db.begin().await?;
    let new_id = insert_with_code(&mut tx, actor.id, &f, Some(id)).await?;
    repo::set_grades_tags(&mut tx, new_id, &src.grade_ids, &src.tag_ids).await?;
    let copied: Vec<_> = steps.into_iter().map(|s| super::model::Step { id: Uuid::now_v7(), ..s }).collect();
    repo::replace_steps(&mut tx, new_id, &copied).await?;
    tx.commit().await?;
    get_own(ctx, actor, new_id).await
}

// ---------- Thành viên (giáo viên) ----------

pub async fn members(ctx: &AppContext, actor: &Actor, id: Uuid, q: MemberQuery) -> AppResult<Vec<MemberResponse>> {
    load_own(ctx, actor, id).await?;
    Ok(repo::members(&ctx.db, id, q.status.as_deref()).await?.into_iter().map(Into::into).collect())
}

/// Giáo viên thêm học sinh bằng username / email → vào học ngay.
pub async fn add_member(ctx: &AppContext, actor: &Actor, id: Uuid, req: AddMemberRequest) -> AppResult<MemberResponse> {
    req.validate()?;
    load_own(ctx, actor, id).await?;
    let user = users::find_by_login(&ctx.db, &req.login.trim().to_lowercase())
        .await?
        .ok_or_else(|| AppError::field("login", "Không tìm thấy tài khoản có username / email này."))?;
    if user.id == actor.id {
        return Err(AppError::field("login", "Bạn là giáo viên của lớp học này."));
    }
    repo::upsert_student(&ctx.db, id, user.id, "active").await?;
    repo::members(&ctx.db, id, None)
        .await?
        .into_iter()
        .find(|m| m.user_id == user.id)
        .map(Into::into)
        .ok_or_else(|| AppError::internal("vừa thêm mà không thấy thành viên"))
}

/// approve: chờ duyệt → đang học · reject: chờ duyệt → bị từ chối · remove: đang học → bị mời ra.
pub async fn change_member(ctx: &AppContext, actor: &Actor, id: Uuid, user_id: Uuid, action: &str) -> AppResult<()> {
    load_own(ctx, actor, id).await?;
    let (from, to): (&[&str], &str) = match action {
        "approve" => (&["pending"], "active"),
        "reject" => (&["pending"], "rejected"),
        "remove" => (&["active", "pending"], "removed"),
        _ => return Err(AppError::not_found("API không tồn tại.")),
    };
    if !repo::transition(&ctx.db, id, user_id, from, to).await? {
        return Err(AppError::conflict("INVALID_MEMBER_STATE", "Trạng thái của học sinh đã thay đổi, hãy tải lại."));
    }
    Ok(())
}

// ---------- Học sinh ----------

async fn membership_of(ctx: &AppContext, class_id: Uuid, viewer: Option<&Actor>) -> AppResult<Option<Membership>> {
    match viewer {
        Some(a) => Ok(repo::membership(&ctx.db, class_id, a.id).await?),
        None => Ok(None),
    }
}

/// Danh sách lớp học công khai.
pub async fn catalog(ctx: &AppContext, viewer: Option<&Actor>, query: ClassQuery) -> AppResult<Page<ClassCard>> {
    let page = PageRequest::new(query.page, query.page_size);
    let filter = ClassFilter {
        scope: ClassScope::Public,
        q: query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()),
        subject_id: query.subject_id,
        grade_id: query.grade_id,
    };
    let rows = repo::list(&ctx.db, &filter, page.limit(), page.offset()).await?;
    let total = repo::count(&ctx.db, &filter).await?;
    let mut items = Vec::with_capacity(rows.len());
    for c in rows {
        let status = membership_of(ctx, c.id, viewer).await?.filter(|m| m.role == "student").map(|m| m.status);
        items.push(ClassCard::new(c, status, viewer.map(|a| a.id)));
    }
    Ok(Page::new(items, total, page))
}

/// Trang giới thiệu lớp học. Lớp riêng tư chỉ thành viên / giáo viên xem được.
pub async fn detail(ctx: &AppContext, viewer: Option<&Actor>, id: Uuid) -> AppResult<ClassDetail> {
    let class = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    let member = membership_of(ctx, id, viewer).await?;
    let is_owner = viewer.is_some_and(|a| a.id == class.owner_id);
    let is_member = member.as_ref().is_some_and(|m| m.status == "active" || m.status == "pending");
    if (class.archived_at.is_some() && !is_owner && !is_member)
        || (class.visibility == "private" && !is_owner && !is_member)
    {
        return Err(not_found());
    }
    let outline = match repo::latest_version(&ctx.db, id).await? {
        Some(v) => outline_of(ctx, &v.steps.0).await?,
        None => Vec::new(),
    };
    let status = member.filter(|m| m.role == "student").map(|m| m.status);
    Ok(ClassDetail { card: ClassCard::new(class, status, viewer.map(|a| a.id)), outline })
}

async fn outline_of(ctx: &AppContext, steps: &[super::model::Step]) -> AppResult<Vec<OutlineStep>> {
    let lesson_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.lesson_id).collect();
    let refs = lessons::refs(ctx, &lesson_ids).await?;
    let exam_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.exam_id).collect();
    let exams = crate::modules::exams::service::refs(ctx, &exam_ids).await?;
    Ok(steps
        .iter()
        .map(|s| {
            let lesson = s.lesson_id.and_then(|id| refs.iter().find(|r| r.id == id));
            let exam = s.exam_id.and_then(|id| exams.iter().find(|e| e.id == id));
            let fallback =
                lesson.map(|l| l.title.clone()).or_else(|| exam.map(|e| e.title.clone())).unwrap_or_default();
            OutlineStep {
                kind: s.kind,
                title: if s.title.trim().is_empty() { fallback } else { s.title.clone() },
                duration_minutes: lesson.and_then(|l| l.duration_minutes),
            }
        })
        .collect())
}

fn already_member() -> AppError {
    AppError::conflict("ALREADY_MEMBER", "Bạn đã ở trong lớp học này.")
}

/// Ghi danh theo chế độ của lớp học.
pub async fn enroll(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<EnrollResponse> {
    let class = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if class.archived_at.is_some() {
        return Err(not_found());
    }
    if class.owner_id == actor.id {
        return Err(AppError::conflict("IS_OWNER", "Bạn là giáo viên của lớp học này."));
    }
    let member = repo::membership(&ctx.db, id, actor.id).await?;
    match member.as_ref().map(|m| m.status.as_str()) {
        Some("active") => return Err(already_member()),
        Some("pending") => return Ok(EnrollResponse { class_id: id, status: "pending".into() }),
        Some("removed") => {
            return Err(AppError::forbidden(
                "REMOVED_FROM_CLASS",
                "Bạn đã bị mời ra khỏi lớp học này. Hãy liên hệ giáo viên.",
            ));
        }
        _ => {}
    }
    if class.visibility == "private" {
        return Err(not_found());
    }
    let status = match class.enrollment.as_str() {
        "open" => "active",
        "approval" => "pending",
        _ => {
            return Err(AppError::forbidden("INVITE_REQUIRED", "Lớp học chỉ nhận học sinh qua mã mời của giáo viên."));
        }
    };
    repo::upsert_student(&ctx.db, id, actor.id, status).await?;
    Ok(EnrollResponse { class_id: id, status: status.into() })
}

/// Vào lớp học bằng mã mời → vào học ngay.
pub async fn join_by_code(ctx: &AppContext, actor: &Actor, req: JoinRequest) -> AppResult<EnrollResponse> {
    req.validate()?;
    let code = req.code.trim().to_uppercase();
    let class = repo::find_by_code(&ctx.db, &code)
        .await?
        .filter(|c| c.archived_at.is_none())
        .ok_or_else(|| AppError::field("code", "Mã mời không đúng hoặc lớp học đã đóng."))?;
    if class.owner_id == actor.id {
        return Err(AppError::conflict("IS_OWNER", "Bạn là giáo viên của lớp học này."));
    }
    match repo::membership(&ctx.db, class.id, actor.id).await?.map(|m| m.status) {
        Some(s) if s == "active" => return Err(already_member()),
        Some(s) if s == "removed" => {
            return Err(AppError::forbidden(
                "REMOVED_FROM_CLASS",
                "Bạn đã bị mời ra khỏi lớp học này. Hãy liên hệ giáo viên.",
            ));
        }
        _ => {}
    }
    repo::upsert_student(&ctx.db, class.id, actor.id, "active").await?;
    Ok(EnrollResponse { class_id: class.id, status: "active".into() })
}

pub async fn leave(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    if !repo::transition(&ctx.db, id, actor.id, &["active", "pending"], "left").await? {
        return Err(AppError::conflict("NOT_MEMBER", "Bạn không ở trong lớp học này."));
    }
    Ok(())
}

/// Lớp học của tôi (đang học / chờ duyệt) kèm % tiến độ.
pub async fn my_classes(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<MyClass>> {
    let rows = repo::list_for_student(&ctx.db, actor.id).await?;
    let mut out = Vec::with_capacity(rows.len());
    for (class, status) in rows {
        let progress = if status == "active" {
            path::progress_summary(ctx, class.id, actor.id).await?
        } else {
            ProgressSummary::new(0, class.step_count)
        };
        out.push(MyClass { class: ClassCard::new(class, Some(status), Some(actor.id)), progress });
    }
    Ok(out)
}
