//! Nghiệp vụ xác thực. Controller chỉ đọc request / cookie rồi gọi các hàm ở đây.

use std::time::Duration;

use chrono::{TimeDelta, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::{
    dto::{
        AuthResponse, ChangePasswordRequest, EmailRequest, LoginRequest, MeResponse, RegisterRequest, RegisterResponse,
        ResetPasswordRequest, SetPasswordRequest, VerifyEmailRequest,
    },
    model::NewRefreshToken,
    repository,
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    jobs::Job,
    mail,
    modules::users::{
        model::{User, UserRole, UserStatus},
        repository::NewUser,
        service as users,
    },
    security::{
        Actor,
        password::{hash_password, verify_password},
        token::{random_otp, random_token, sha256_hex},
    },
};

const OTP_TTL: TimeDelta = TimeDelta::minutes(15);
const OTP_MAX_ATTEMPTS: i32 = 5;
const OTP_RESEND_COOLDOWN: TimeDelta = TimeDelta::seconds(60);
const RESET_TTL: TimeDelta = TimeDelta::minutes(30);
/// Hai tab cùng refresh một lúc: token vừa bị thay trong khoảng này thì không coi là bị đánh cắp.
const REFRESH_RACE_GRACE: TimeDelta = TimeDelta::seconds(30);

/// Giá trị `refresh_tokens.revoked_reason` khi phiên bị đăng xuất vì vượt số thiết bị.
const SESSION_LIMIT_REASON: &str = "session_limit";

const LOGIN_MAX_FAILS: u32 = 10;
const LOGIN_WINDOW: Duration = Duration::from_secs(15 * 60);

/// Thông tin thiết bị gọi API (ghi vào phiên đăng nhập, dùng cho giới hạn tần suất).
#[derive(Debug, Clone, Default)]
pub struct ClientInfo {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

impl ClientInfo {
    fn ip_key(&self) -> &str {
        self.ip.as_deref().unwrap_or("unknown")
    }
}

/// Kết quả đăng nhập: JSON trả về + refresh token để controller đặt vào cookie.
pub struct Session {
    pub response: AuthResponse,
    pub refresh_token: String,
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn hash_otp(user_id: Uuid, code: &str) -> String {
    sha256_hex(&format!("{user_id}:{code}"))
}

fn invalid_code() -> AppError {
    AppError::bad_request("INVALID_CODE", "Mã xác minh không đúng.")
}

fn invalid_refresh() -> AppError {
    AppError::unauthorized("INVALID_REFRESH_TOKEN", "Phiên đăng nhập không hợp lệ, vui lòng đăng nhập lại.")
}

fn invalid_reset() -> AppError {
    AppError::bad_request("INVALID_RESET_TOKEN", "Link đặt lại mật khẩu không hợp lệ hoặc đã hết hạn.")
}

/// Gửi email qua hàng đợi. Lỗi hàng đợi chỉ ghi log: người dùng có thể bấm "gửi lại".
async fn send_email(ctx: &AppContext, email: crate::jobs::Email) {
    if let Err(e) = ctx.jobs.publish(Job::SendEmail(email)).await {
        tracing::error!(error = %e, "không đẩy được email vào hàng đợi");
    }
}

// ---------- Đăng ký & xác minh email ----------

pub async fn register(ctx: &AppContext, req: RegisterRequest, client: &ClientInfo) -> AppResult<RegisterResponse> {
    ctx.limiter.hit_and_check(&format!("register:{}", client.ip_key()), 10, Duration::from_secs(3600)).await?;

    let username = req.username.trim();
    let email = normalize_email(&req.email);
    let full_name = req.full_name.trim();

    if users::username_exists(&ctx.db, username).await? {
        return Err(users::username_taken());
    }
    if users::email_taken(&ctx.db, &email, None).await? {
        return Err(users::email_taken_error());
    }

    let password_hash = hash_password(req.password).await?;
    let code = random_otp();

    let mut tx = ctx.db.begin().await?;
    let user = users::create(
        &mut *tx,
        &NewUser {
            username,
            email: &email,
            full_name,
            password_hash: Some(&password_hash),
            role: UserRole::Student,
            status: UserStatus::PendingVerification,
            email_verified: false,
        },
    )
    .await?;
    repository::upsert_verification(&mut *tx, user.id, &hash_otp(user.id, &code), Utc::now() + OTP_TTL).await?;
    tx.commit().await?;

    send_email(ctx, mail::verification_code(&user.email, &user.full_name, &code)).await;
    tracing::info!(user_id = %user.id, "đăng ký tài khoản mới");

    Ok(RegisterResponse { user_id: user.id, email: user.email, verification_required: true })
}

pub async fn verify_email(ctx: &AppContext, req: VerifyEmailRequest, client: &ClientInfo) -> AppResult<Session> {
    // Chặn dò mã trên nhiều tài khoản từ cùng một IP.
    ctx.limiter.hit_and_check(&format!("verify:{}", client.ip_key()), 30, Duration::from_secs(15 * 60)).await?;

    let email = normalize_email(&req.email);
    let user = users::find_by_email(&ctx.db, &email).await?.ok_or_else(invalid_code)?;
    match user.status {
        UserStatus::Locked => return Err(account_locked()),
        UserStatus::Active => {
            return Err(AppError::bad_request("ALREADY_VERIFIED", "Email đã được xác minh, hãy đăng nhập."));
        }
        UserStatus::PendingVerification => {}
    }

    let v = repository::find_verification(&ctx.db, user.id).await?.ok_or_else(invalid_code)?;
    if v.expires_at < Utc::now() {
        return Err(AppError::bad_request("CODE_EXPIRED", "Mã xác minh đã hết hạn, hãy yêu cầu mã mới."));
    }
    if v.code_hash != hash_otp(user.id, req.code.trim()) {
        let attempts = repository::increment_verification_attempts(&ctx.db, user.id).await?;
        if attempts >= OTP_MAX_ATTEMPTS {
            repository::delete_verification(&ctx.db, user.id).await?;
            return Err(AppError::bad_request(
                "TOO_MANY_ATTEMPTS",
                "Nhập sai quá nhiều lần, mã đã bị hủy. Hãy yêu cầu mã mới.",
            ));
        }
        return Err(invalid_code());
    }

    let mut tx = ctx.db.begin().await?;
    users::mark_email_verified(&mut *tx, user.id).await?;
    repository::delete_verification(&mut *tx, user.id).await?;
    users::touch_login(&mut *tx, user.id).await?;
    let user = users::find_by_id(&mut *tx, user.id).await?.ok_or_else(invalid_code)?;
    let session = start_session(ctx, &mut tx, &user, client).await?;
    tx.commit().await?;
    Ok(session)
}

/// Gửi lại mã OTP. Luôn thành công (không cho biết email nào đã đăng ký).
pub async fn resend_verification(ctx: &AppContext, req: EmailRequest, client: &ClientInfo) -> AppResult<()> {
    ctx.limiter.hit_and_check(&format!("resend:{}", client.ip_key()), 10, Duration::from_secs(3600)).await?;
    let email = normalize_email(&req.email);
    let Some(user) = users::find_by_email(&ctx.db, &email).await? else {
        return Ok(());
    };
    if user.status != UserStatus::PendingVerification {
        return Ok(());
    }
    if let Some(v) = repository::find_verification(&ctx.db, user.id).await?
        && Utc::now() - v.sent_at < OTP_RESEND_COOLDOWN
    {
        return Ok(());
    }
    let code = random_otp();
    repository::upsert_verification(&ctx.db, user.id, &hash_otp(user.id, &code), Utc::now() + OTP_TTL).await?;
    send_email(ctx, mail::verification_code(&user.email, &user.full_name, &code)).await;
    Ok(())
}

// ---------- Đăng nhập & phiên ----------

pub(crate) fn account_locked() -> AppError {
    AppError::forbidden("ACCOUNT_LOCKED", "Tài khoản đã bị khóa. Vui lòng liên hệ quản trị viên.")
}

pub async fn login(ctx: &AppContext, req: LoginRequest, client: &ClientInfo) -> AppResult<Session> {
    let login = req.login.trim().to_lowercase();
    let key = format!("login:{}:{}", client.ip_key(), login);
    ctx.limiter.check(&key, LOGIN_MAX_FAILS).await?;

    let user = users::find_by_login(&ctx.db, &login).await?;
    let ok = verify_password(req.password, user.as_ref().and_then(|u| u.password_hash.clone())).await?;
    let Some(user) = user.filter(|_| ok) else {
        ctx.limiter.hit(&key, LOGIN_WINDOW).await;
        return Err(AppError::unauthorized("INVALID_CREDENTIALS", "Username/email hoặc mật khẩu không đúng."));
    };

    match user.status {
        UserStatus::Locked => return Err(account_locked()),
        UserStatus::PendingVerification => {
            return Err(AppError::forbidden(
                "EMAIL_NOT_VERIFIED",
                "Email chưa được xác minh. Hãy nhập mã đã gửi tới email của bạn.",
            ));
        }
        UserStatus::Active => {}
    }

    let mut tx = ctx.db.begin().await?;
    users::touch_login(&mut *tx, user.id).await?;
    let session = start_session(ctx, &mut tx, &user, client).await?;
    tx.commit().await?;
    Ok(session)
}

/// Đăng nhập trên một thiết bị mới: tạo chuỗi phiên mới, rồi giữ tối đa `MAX_SESSIONS_PER_USER` phiên —
/// phiên lâu không dùng nhất bị đăng xuất (thiết bị đó nhận lỗi `SESSION_LIMIT` khi refresh).
pub(crate) async fn start_session(
    ctx: &AppContext,
    conn: &mut sqlx::PgConnection,
    user: &User,
    client: &ClientInfo,
) -> AppResult<Session> {
    let session = issue_session(ctx, &mut *conn, user, Uuid::now_v7(), Uuid::now_v7(), client).await?;
    if ctx.config.max_sessions > 0 {
        let revoked = repository::revoke_beyond_limit(&mut *conn, user.id, i64::from(ctx.config.max_sessions)).await?;
        if revoked > 0 {
            tracing::info!(user_id = %user.id, revoked, "vượt số thiết bị, đăng xuất phiên cũ");
        }
    }
    Ok(session)
}

/// Tạo refresh token mới (id `token_id`, chuỗi phiên `family_id`) và access token.
pub(crate) async fn issue_session<'e, E: PgExecutor<'e>>(
    ctx: &AppContext,
    db: E,
    user: &User,
    family_id: Uuid,
    token_id: Uuid,
    client: &ClientInfo,
) -> AppResult<Session> {
    let refresh_token = random_token();
    let token_hash = sha256_hex(&refresh_token);
    let refresh_ttl = TimeDelta::from_std(ctx.config.refresh_ttl).map_err(AppError::internal)?;
    repository::insert_refresh(
        db,
        &NewRefreshToken {
            id: token_id,
            user_id: user.id,
            family_id,
            token_hash: &token_hash,
            expires_at: Utc::now() + refresh_ttl,
            user_agent: client.user_agent.as_deref(),
            ip: client.ip.as_deref(),
        },
    )
    .await?;
    let access_token = ctx.jwt.issue(user.id, user.role, family_id)?;
    Ok(Session {
        response: AuthResponse {
            access_token,
            token_type: "Bearer",
            expires_in: ctx.jwt.ttl_secs(),
            user: MeResponse::from(user.clone()),
        },
        refresh_token,
    })
}

/// Đổi refresh token lấy access token mới; refresh token được xoay vòng (token cũ hết dùng được).
pub async fn refresh(ctx: &AppContext, token: Option<&str>, client: &ClientInfo) -> AppResult<Session> {
    let token = token
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AppError::unauthorized("NO_REFRESH_TOKEN", "Bạn chưa đăng nhập hoặc phiên đã hết hạn."))?;
    let rt = repository::find_refresh_by_hash(&ctx.db, &sha256_hex(token)).await?.ok_or_else(invalid_refresh)?;

    if let Some(revoked_at) = rt.revoked_at {
        if rt.revoked_reason.as_deref() == Some(SESSION_LIMIT_REASON) {
            return Err(AppError::unauthorized(
                "SESSION_LIMIT",
                "Tài khoản vừa đăng nhập trên thiết bị khác nên thiết bị này đã bị đăng xuất.",
            ));
        }
        let recently_rotated = rt.replaced_by.is_some() && Utc::now() - revoked_at < REFRESH_RACE_GRACE;
        if !recently_rotated {
            // Token đã thu hồi mà vẫn được dùng → có thể bị đánh cắp: thu hồi cả chuỗi phiên.
            repository::revoke_family(&ctx.db, rt.family_id).await?;
            tracing::warn!(user_id = %rt.user_id, "phát hiện dùng lại refresh token, thu hồi chuỗi phiên");
            return Err(AppError::unauthorized(
                "REFRESH_TOKEN_REUSED",
                "Phiên đăng nhập đã bị thu hồi, vui lòng đăng nhập lại.",
            ));
        }
        return Err(invalid_refresh());
    }
    if rt.expires_at < Utc::now() {
        return Err(AppError::unauthorized("SESSION_EXPIRED", "Phiên đăng nhập đã hết hạn, vui lòng đăng nhập lại."));
    }

    let user = users::find_by_id(&ctx.db, rt.user_id).await?.ok_or_else(invalid_refresh)?;
    if user.status != UserStatus::Active {
        repository::revoke_family(&ctx.db, rt.family_id).await?;
        return Err(account_locked());
    }

    let new_id = Uuid::now_v7();
    let mut tx = ctx.db.begin().await?;
    if !repository::claim_refresh(&mut *tx, rt.id, new_id).await? {
        return Err(invalid_refresh());
    }
    let session = issue_session(ctx, &mut *tx, &user, rt.family_id, new_id, client).await?;
    tx.commit().await?;
    Ok(session)
}

/// Đăng xuất phiên hiện tại (theo refresh token trong cookie). Không có token cũng coi là thành công.
pub async fn logout(ctx: &AppContext, token: Option<&str>) -> AppResult<()> {
    if let Some(token) = token.filter(|t| !t.is_empty())
        && let Some(rt) = repository::find_refresh_by_hash(&ctx.db, &sha256_hex(token)).await?
    {
        repository::revoke_family(&ctx.db, rt.family_id).await?;
    }
    Ok(())
}

pub async fn logout_all(ctx: &AppContext, actor: &Actor) -> AppResult<()> {
    revoke_all_sessions(&ctx.db, actor.id).await
}

/// Thu hồi mọi phiên của user (dùng khi khóa tài khoản, đặt lại mật khẩu, đổi vai trò).
pub async fn revoke_all_sessions<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> AppResult<()> {
    repository::revoke_all_for_user(db, user_id).await?;
    Ok(())
}

pub async fn active_session_count(ctx: &AppContext, user_id: Uuid) -> AppResult<i64> {
    Ok(repository::count_active_sessions(&ctx.db, user_id).await?)
}

pub async fn me(ctx: &AppContext, actor: &Actor) -> AppResult<MeResponse> {
    let user = users::find_by_id(&ctx.db, actor.id)
        .await?
        .ok_or_else(|| AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập."))?;
    if user.status == UserStatus::Locked {
        return Err(account_locked());
    }
    Ok(user.into())
}

// ---------- Mật khẩu ----------

/// Quên mật khẩu. Luôn thành công (không cho biết email nào đã đăng ký).
pub async fn forgot_password(ctx: &AppContext, req: EmailRequest, client: &ClientInfo) -> AppResult<()> {
    ctx.limiter.hit_and_check(&format!("forgot:{}", client.ip_key()), 10, Duration::from_secs(3600)).await?;
    let email = normalize_email(&req.email);
    if let Some(user) = users::find_by_email(&ctx.db, &email).await?
        && user.status != UserStatus::Locked
    {
        start_password_reset(ctx, &user).await?;
    }
    Ok(())
}

/// Tạo link đặt lại mật khẩu và gửi email (link cũ chưa dùng bị vô hiệu hóa).
pub async fn start_password_reset(ctx: &AppContext, user: &User) -> AppResult<()> {
    let token = random_token();
    let mut tx = ctx.db.begin().await?;
    repository::invalidate_password_resets(&mut *tx, user.id).await?;
    repository::insert_password_reset(&mut *tx, &sha256_hex(&token), user.id, Utc::now() + RESET_TTL).await?;
    tx.commit().await?;
    let link = format!("{}/dat-lai-mat-khau?token={}", ctx.config.app_base_url, token);
    send_email(ctx, mail::password_reset(&user.email, &user.full_name, &link)).await;
    Ok(())
}

pub async fn reset_password(ctx: &AppContext, req: ResetPasswordRequest) -> AppResult<()> {
    let token_hash = sha256_hex(req.token.trim());
    let pr = repository::find_password_reset(&ctx.db, &token_hash).await?.ok_or_else(invalid_reset)?;
    if pr.used_at.is_some() || pr.expires_at < Utc::now() {
        return Err(invalid_reset());
    }
    let user = users::find_by_id(&ctx.db, pr.user_id).await?.ok_or_else(invalid_reset)?;
    if user.status == UserStatus::Locked {
        return Err(account_locked());
    }

    let new_hash = hash_password(req.new_password).await?;
    let mut tx = ctx.db.begin().await?;
    if !repository::mark_password_reset_used(&mut *tx, &token_hash).await? {
        return Err(invalid_reset());
    }
    users::set_password(&mut *tx, user.id, &new_hash).await?;
    // Mở được link trong email nghĩa là đã chứng minh sở hữu email.
    users::mark_email_verified(&mut *tx, user.id).await?;
    repository::delete_verification(&mut *tx, user.id).await?;
    repository::revoke_all_for_user(&mut *tx, user.id).await?;
    tx.commit().await?;
    tracing::info!(user_id = %user.id, "đặt lại mật khẩu");
    Ok(())
}

pub async fn change_password(ctx: &AppContext, actor: &Actor, req: ChangePasswordRequest) -> AppResult<()> {
    let user = users::find_by_id(&ctx.db, actor.id)
        .await?
        .ok_or_else(|| AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập."))?;
    if req.current_password == req.new_password {
        return Err(AppError::field("newPassword", "Mật khẩu mới phải khác mật khẩu hiện tại."));
    }
    if !verify_password(req.current_password, user.password_hash.clone()).await? {
        return Err(AppError::field("currentPassword", "Mật khẩu hiện tại không đúng."));
    }
    let new_hash = hash_password(req.new_password).await?;
    let mut tx = ctx.db.begin().await?;
    users::set_password(&mut *tx, user.id, &new_hash).await?;
    repository::invalidate_password_resets(&mut *tx, user.id).await?;
    repository::revoke_others_for_user(&mut *tx, user.id, actor.session_id).await?;
    tx.commit().await?;
    Ok(())
}

/// Đặt mật khẩu lần đầu cho tài khoản tạo từ SSO (chưa có mật khẩu). Các thiết bị khác bị đăng xuất.
pub async fn set_password(ctx: &AppContext, actor: &Actor, req: SetPasswordRequest) -> AppResult<()> {
    let user = users::find_by_id(&ctx.db, actor.id)
        .await?
        .ok_or_else(|| AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập."))?;
    if user.password_hash.is_some() {
        return Err(AppError::conflict(
            "PASSWORD_ALREADY_SET",
            "Tài khoản đã có mật khẩu. Hãy dùng chức năng Đổi mật khẩu.",
        ));
    }
    let new_hash = hash_password(req.new_password).await?;
    let mut tx = ctx.db.begin().await?;
    // Chỉ đặt khi vẫn chưa có mật khẩu (hai request song song thì một request thắng).
    if !users::set_password_if_empty(&mut *tx, user.id, &new_hash).await? {
        return Err(AppError::conflict("PASSWORD_ALREADY_SET", "Tài khoản đã có mật khẩu."));
    }
    repository::revoke_others_for_user(&mut *tx, user.id, actor.session_id).await?;
    tx.commit().await?;
    Ok(())
}

/// Hủy mọi thông tin đăng nhập cũ của user (mã OTP, link đặt lại mật khẩu, mọi phiên).
/// Dùng khi chủ email thật giành lại tài khoản chưa xác minh (xem module `sso`).
pub(crate) async fn revoke_credentials(conn: &mut sqlx::PgConnection, user_id: Uuid) -> AppResult<()> {
    repository::delete_verification(&mut *conn, user_id).await?;
    repository::invalidate_password_resets(&mut *conn, user_id).await?;
    repository::revoke_all_for_user(&mut *conn, user_id).await?;
    Ok(())
}

/// Thu hồi mọi phiên của user trừ chuỗi phiên `keep_family` (thiết bị đang dùng).
pub(crate) async fn revoke_other_sessions<'e, E: PgExecutor<'e>>(
    db: E,
    user_id: Uuid,
    keep_family: Uuid,
) -> AppResult<()> {
    repository::revoke_others_for_user(db, user_id, keep_family).await?;
    Ok(())
}
