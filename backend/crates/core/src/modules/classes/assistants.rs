//! Trợ giảng (M32): chủ lớp mời giáo viên khác cùng lo lớp học (`class_members.role = 'assistant'`).
//!
//! Trợ giảng được: xem lớp, danh sách học sinh, duyệt / từ chối ghi danh, tiến độ, báo cáo; chấm bài tập về nhà;
//! trả lời / ghim / ẩn hỏi đáp; vào phòng học trực tuyến của lớp với quyền điều khiển như giáo viên (trừ kết thúc
//! buổi và tạo link cho khách).
//! Không được: sửa thông tin / lộ trình, đổi mã mời, thêm / mời ra học sinh, lưu trữ / sao chép lớp, mời trợ giảng.
//! Chỉ tài khoản giáo viên mới làm trợ giảng được (trang dạy học cần quyền `teaching.manage`).

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::AddMemberRequest,
    model::ClassRow,
    repository as repo,
    service::{StaffRole, load_own, load_staff, not_found},
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        notifications::{self, NotificationKind},
        users::{model::UserStatus, service as users},
    },
    security::{Actor, Permission, permissions::has_permission},
};

/// Tối đa số trợ giảng mỗi lớp.
pub const MAX_ASSISTANTS: i64 = 5;

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AssistantView {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub email: String,
    pub added_at: DateTime<Utc>,
}

async fn rows(ctx: &AppContext, class_id: Uuid) -> AppResult<Vec<AssistantView>> {
    Ok(sqlx::query_as::<_, AssistantView>(
        "SELECT u.id AS user_id, u.username, u.full_name, u.email, m.updated_at AS added_at \
         FROM class_members m JOIN users u ON u.id = m.user_id \
         WHERE m.class_id = $1 AND m.role = 'assistant' AND m.status = 'active' ORDER BY u.full_name",
    )
    .bind(class_id)
    .fetch_all(&ctx.db)
    .await?)
}

/// Trợ giảng của lớp (chủ lớp và các trợ giảng đều xem được).
pub async fn list(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<Vec<AssistantView>> {
    load_staff(ctx, actor, class_id).await?;
    rows(ctx, class_id).await
}

/// Chủ lớp mời một giáo viên (username / email) làm trợ giảng.
pub async fn add(ctx: &AppContext, actor: &Actor, class_id: Uuid, req: AddMemberRequest) -> AppResult<AssistantView> {
    req.validate()?;
    let class = load_own(ctx, actor, class_id).await?;
    if class.archived_at.is_some() {
        return Err(AppError::conflict("CLASS_ARCHIVED", "Lớp học đã lưu trữ."));
    }
    let user = users::find_by_login(&ctx.db, &req.login.trim().to_lowercase())
        .await?
        .filter(|u| u.status == UserStatus::Active)
        .ok_or_else(|| AppError::field("login", "Không tìm thấy tài khoản có username / email này."))?;
    if user.id == actor.id {
        return Err(AppError::field("login", "Bạn là giáo viên chủ lớp."));
    }
    if !has_permission(user.role, Permission::TeachingManage) {
        return Err(AppError::field("login", "Chỉ tài khoản giáo viên mới làm trợ giảng được."));
    }
    match repo::membership(&ctx.db, class_id, user.id).await? {
        Some(m) if m.role == "assistant" && m.status == "active" => {
            return Err(AppError::field("login", "Người này đã là trợ giảng của lớp."));
        }
        Some(m) if m.role == "student" && matches!(m.status.as_str(), "active" | "pending") => {
            return Err(AppError::field("login", "Người này đang là học sinh của lớp."));
        }
        _ => {}
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM class_members WHERE class_id = $1 AND role = 'assistant' AND status = 'active'",
    )
    .bind(class_id)
    .fetch_one(&ctx.db)
    .await?;
    if count >= MAX_ASSISTANTS {
        return Err(AppError::field("login", format!("Mỗi lớp tối đa {MAX_ASSISTANTS} trợ giảng.")));
    }
    // Học sinh đã rời / bị mời ra trước đây: chuyển hẳn sang vai trợ giảng.
    sqlx::query(
        "INSERT INTO class_members (class_id, user_id, role, status) VALUES ($1, $2, 'assistant', 'active') \
         ON CONFLICT (class_id, user_id) DO UPDATE SET role = 'assistant', status = 'active', updated_at = now()",
    )
    .bind(class_id)
    .bind(user.id)
    .execute(&ctx.db)
    .await?;
    notifications::notify(
        &ctx.db,
        &[user.id],
        NotificationKind::AssistantAdded,
        &format!("Bạn được mời làm trợ giảng lớp \"{}\"", class.title),
        &format!("Giáo viên {} mời bạn cùng chấm bài, trả lời hỏi đáp và theo dõi học sinh.", class.owner_name),
        Some(&format!("/day-hoc/lop-hoc/{class_id}")),
    )
    .await?;
    rows(ctx, class_id)
        .await?
        .into_iter()
        .find(|a| a.user_id == user.id)
        .ok_or_else(|| AppError::internal("vừa thêm mà không thấy trợ giảng"))
}

/// Chủ lớp bỏ một trợ giảng, hoặc trợ giảng tự rời lớp.
pub async fn remove(ctx: &AppContext, actor: &Actor, class_id: Uuid, user_id: Uuid) -> AppResult<()> {
    let (_, role) = load_staff(ctx, actor, class_id).await?;
    if role == StaffRole::Assistant && user_id != actor.id {
        return Err(not_found());
    }
    let res = sqlx::query("DELETE FROM class_members WHERE class_id = $1 AND user_id = $2 AND role = 'assistant'")
        .bind(class_id)
        .bind(user_id)
        .execute(&ctx.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("Không tìm thấy trợ giảng."));
    }
    Ok(())
}

/// Người nhận thông báo việc của lớp: chủ lớp + các trợ giảng.
pub async fn staff_ids(ctx: &AppContext, class: &ClassRow) -> AppResult<Vec<Uuid>> {
    let mut ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM class_members WHERE class_id = $1 AND role = 'assistant' AND status = 'active'",
    )
    .bind(class.id)
    .fetch_all(&ctx.db)
    .await?;
    ids.insert(0, class.owner_id);
    Ok(ids)
}
