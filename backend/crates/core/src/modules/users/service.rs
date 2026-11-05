//! Nghiệp vụ người dùng.
//!
//! - Phần đầu: hàm cho module khác dùng (auth, teachers) — bọc repository, đổi lỗi UNIQUE thành 409.
//! - Phần sau: admin quản lý user (M3) — luôn kiểm tra quyền và ghi nhật ký.

use serde_json::json;
use sqlx::PgExecutor;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{CreateUserRequest, UpdateUserRequest, UserListQuery, UserResponse, validate_password},
    model::{User, UserRole, UserStatus},
    repository::{self, NewUser, UserFilter},
};
use crate::{
    context::AppContext,
    db::{escape_like, is_unique_violation},
    error::{AppError, AppResult},
    modules::{
        audit::{model::AuditEntry, service as audit},
        auth::service as auth,
    },
    pagination::{Page, PageRequest},
    security::{Actor, Permission, password::hash_password},
};

// ---------- Dùng chung cho các module khác ----------

pub fn username_taken() -> AppError {
    AppError::conflict("USERNAME_TAKEN", "Username đã có người dùng.")
}

pub fn email_taken_error() -> AppError {
    AppError::conflict("EMAIL_TAKEN", "Email đã được dùng cho tài khoản khác.")
}

pub async fn find_by_id<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> AppResult<Option<User>> {
    Ok(repository::find_by_id(db, id).await?)
}

pub async fn find_by_login<'e, E: PgExecutor<'e>>(db: E, login: &str) -> AppResult<Option<User>> {
    Ok(repository::find_by_login(db, login).await?)
}

pub async fn find_by_email<'e, E: PgExecutor<'e>>(db: E, email: &str) -> AppResult<Option<User>> {
    Ok(repository::find_by_email(db, email).await?)
}

pub async fn username_exists<'e, E: PgExecutor<'e>>(db: E, username: &str) -> AppResult<bool> {
    Ok(repository::username_exists(db, username).await?)
}

pub async fn email_taken<'e, E: PgExecutor<'e>>(db: E, email: &str, except: Option<Uuid>) -> AppResult<bool> {
    Ok(repository::email_taken(db, email, except).await?)
}

/// Tạo user. Hai request trùng username/email chạy song song: ràng buộc UNIQUE của DB chặn lại → 409.
pub async fn create<'e, E: PgExecutor<'e>>(db: E, new: &NewUser<'_>) -> AppResult<User> {
    repository::insert(db, new).await.map_err(|e| {
        if is_unique_violation(&e, "users_username_key") {
            username_taken()
        } else if is_unique_violation(&e, "users_email_key") {
            email_taken_error()
        } else {
            e.into()
        }
    })
}

pub async fn mark_email_verified<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> AppResult<()> {
    Ok(repository::mark_email_verified(db, id).await?)
}

pub async fn set_password<'e, E: PgExecutor<'e>>(db: E, id: Uuid, hash: &str) -> AppResult<()> {
    Ok(repository::update_password(db, id, hash).await?)
}

pub async fn clear_password<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> AppResult<()> {
    Ok(repository::clear_password(db, id).await?)
}

pub async fn set_role<'e, E: PgExecutor<'e>>(db: E, id: Uuid, role: UserRole) -> AppResult<()> {
    Ok(repository::set_role(db, id, role).await?)
}

pub async fn touch_login<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> AppResult<()> {
    Ok(repository::touch_login(db, id).await?)
}

// ---------- Admin đầu tiên ----------

/// Tạo admin đầu tiên từ `BOOTSTRAP_ADMIN_*` nếu hệ thống chưa có admin nào.
pub async fn bootstrap_admin(ctx: &AppContext) -> AppResult<Option<User>> {
    let Some(cfg) = ctx.config.bootstrap_admin.clone() else {
        return Ok(None);
    };
    if repository::admin_exists(&ctx.db).await? {
        return Ok(None);
    }
    validate_password(&cfg.password)
        .map_err(|_| AppError::internal("BOOTSTRAP_ADMIN_PASSWORD phải 8–128 ký tự, có cả chữ và số"))?;
    let hash = hash_password(cfg.password).await?;
    let user = create(
        &ctx.db,
        &NewUser {
            username: &cfg.username,
            email: &cfg.email.trim().to_lowercase(),
            full_name: &cfg.full_name,
            password_hash: Some(&hash),
            role: UserRole::Admin,
            status: UserStatus::Active,
            email_verified: true,
        },
    )
    .await?;
    tracing::info!(username = %user.username, "đã tạo tài khoản admin đầu tiên");
    Ok(Some(user))
}

// ---------- Admin quản lý user (M3) ----------

async fn get_or_404<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> AppResult<User> {
    repository::find_by_id(db, id).await?.ok_or_else(|| AppError::not_found("Không tìm thấy người dùng."))
}

fn cannot_modify_self() -> AppError {
    AppError::conflict("CANNOT_MODIFY_SELF", "Bạn không thể tự khóa hoặc tự đổi vai trò của chính mình.")
}

fn last_admin() -> AppError {
    AppError::conflict("LAST_ADMIN", "Hệ thống phải còn ít nhất một admin đang hoạt động.")
}

pub async fn list(ctx: &AppContext, actor: &Actor, q: UserListQuery) -> AppResult<Page<UserResponse>> {
    actor.require(Permission::UsersRead)?;
    let page = PageRequest::new(q.page, q.page_size);
    let search = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(escape_like);
    let filter = UserFilter { q: search.as_deref(), role: q.role, status: q.status };
    let total = repository::count(&ctx.db, &filter).await?;
    let users = repository::list(&ctx.db, &filter, page.limit(), page.offset()).await?;
    Ok(Page::new(users.into_iter().map(Into::into).collect(), total, page))
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<UserResponse> {
    actor.require(Permission::UsersRead)?;
    Ok(get_or_404(&ctx.db, id).await?.into())
}

pub async fn admin_create(
    ctx: &AppContext,
    actor: &Actor,
    req: CreateUserRequest,
    ip: Option<&str>,
) -> AppResult<UserResponse> {
    actor.require(Permission::UsersManage)?;
    req.validate()?;
    let username = req.username.trim();
    let email = req.email.trim().to_lowercase();
    if repository::username_exists(&ctx.db, username).await? {
        return Err(username_taken());
    }
    if repository::email_taken(&ctx.db, &email, None).await? {
        return Err(email_taken_error());
    }
    let hash = hash_password(req.password).await?;

    let mut tx = ctx.db.begin().await?;
    let user = create(
        &mut *tx,
        &NewUser {
            username,
            email: &email,
            full_name: req.full_name.trim(),
            password_hash: Some(&hash),
            role: req.role,
            status: UserStatus::Active,
            email_verified: true,
        },
    )
    .await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "user.create",
            target_type: "user",
            target_id: Some(user.id),
            data: json!({ "username": user.username, "role": user.role }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(user.into())
}

pub async fn admin_update(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    req: UpdateUserRequest,
    ip: Option<&str>,
) -> AppResult<UserResponse> {
    actor.require(Permission::UsersManage)?;
    req.validate()?;
    let user = get_or_404(&ctx.db, id).await?;

    let new_role = req.role.filter(|r| *r != user.role);
    if new_role.is_some() {
        if id == actor.id {
            return Err(cannot_modify_self());
        }
        if user.role == UserRole::Admin
            && user.status == UserStatus::Active
            && repository::count_active_admins(&ctx.db).await? <= 1
        {
            return Err(last_admin());
        }
    }
    let new_email = req.email.as_deref().map(|e| e.trim().to_lowercase()).filter(|e| *e != user.email);
    if let Some(email) = &new_email
        && repository::email_taken(&ctx.db, email, Some(id)).await?
    {
        return Err(email_taken_error());
    }
    let new_name = req.full_name.as_deref().map(str::trim).filter(|n| *n != user.full_name);

    let mut tx = ctx.db.begin().await?;
    let updated = repository::update(&mut *tx, id, new_email.as_deref(), new_name, new_role)
        .await
        .map_err(|e| if is_unique_violation(&e, "users_email_key") { email_taken_error() } else { e.into() })?;
    if new_role.is_some() {
        // Vai trò mới có hiệu lực ở lần đăng nhập sau.
        auth::revoke_all_sessions(&mut *tx, id).await?;
    }
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "user.update",
            target_type: "user",
            target_id: Some(id),
            data: json!({
                "email": new_email.as_ref().map(|e| json!({ "from": user.email, "to": e })),
                "fullName": new_name.map(|n| json!({ "from": user.full_name, "to": n })),
                "role": new_role.map(|r| json!({ "from": user.role, "to": r })),
            }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(updated.into())
}

pub async fn lock(ctx: &AppContext, actor: &Actor, id: Uuid, ip: Option<&str>) -> AppResult<UserResponse> {
    actor.require(Permission::UsersManage)?;
    if id == actor.id {
        return Err(cannot_modify_self());
    }
    let user = get_or_404(&ctx.db, id).await?;
    if user.status == UserStatus::Locked {
        return Ok(user.into());
    }
    if user.role == UserRole::Admin && repository::count_active_admins(&ctx.db).await? <= 1 {
        return Err(last_admin());
    }
    let mut tx = ctx.db.begin().await?;
    repository::set_status(&mut *tx, id, UserStatus::Locked).await?;
    auth::revoke_all_sessions(&mut *tx, id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "user.lock",
            target_type: "user",
            target_id: Some(id),
            data: json!({ "username": user.username }),
            ip,
        },
    )
    .await?;
    let user = get_or_404(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(user.into())
}

pub async fn unlock(ctx: &AppContext, actor: &Actor, id: Uuid, ip: Option<&str>) -> AppResult<UserResponse> {
    actor.require(Permission::UsersManage)?;
    let user = get_or_404(&ctx.db, id).await?;
    if user.status != UserStatus::Locked {
        return Ok(user.into());
    }
    let status = if user.email_verified_at.is_some() { UserStatus::Active } else { UserStatus::PendingVerification };
    let mut tx = ctx.db.begin().await?;
    repository::set_status(&mut *tx, id, status).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "user.unlock",
            target_type: "user",
            target_id: Some(id),
            data: json!({ "username": user.username }),
            ip,
        },
    )
    .await?;
    let user = get_or_404(&mut *tx, id).await?;
    tx.commit().await?;
    Ok(user.into())
}

pub async fn send_password_reset(ctx: &AppContext, actor: &Actor, id: Uuid, ip: Option<&str>) -> AppResult<()> {
    actor.require(Permission::UsersManage)?;
    let user = get_or_404(&ctx.db, id).await?;
    if user.status == UserStatus::Locked {
        return Err(AppError::conflict("ACCOUNT_LOCKED", "Tài khoản đang bị khóa, hãy mở khóa trước."));
    }
    auth::start_password_reset(ctx, &user).await?;
    audit::record(
        &ctx.db,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "user.password_reset_sent",
            target_type: "user",
            target_id: Some(id),
            data: json!({ "email": user.email }),
            ip,
        },
    )
    .await?;
    Ok(())
}
