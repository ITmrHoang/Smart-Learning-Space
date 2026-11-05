//! Nghiệp vụ SSO. Xem chính sách ở `mod.rs`.

use std::time::Duration;

use chrono::{TimeDelta, Utc};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{ConfirmLinkRequest, IdentityResponse, LinkRequired, LinkTokenRequest},
    model::{LinkRequest, ProviderIdentity, SsoProvider},
    repository::{self as repo, NewLinkRequest},
};
use crate::{
    context::AppContext,
    db::is_unique_violation,
    error::{AppError, AppResult},
    jobs::Job,
    mail,
    modules::{
        audit::{model::AuditEntry, service as audit},
        auth::service::{self as auth, ClientInfo, Session},
        users::{
            dto::validate_username,
            model::{User, UserRole, UserStatus},
            repository::NewUser,
            service as users,
        },
    },
    security::{
        Actor, Permission,
        password::verify_password,
        token::{random_otp, random_token, sha256_hex},
    },
};

const LINK_TTL: TimeDelta = TimeDelta::minutes(15);
const LINK_MAX_ATTEMPTS: i32 = 5;
const CODE_COOLDOWN: TimeDelta = TimeDelta::seconds(60);

/// Kết quả đăng nhập SSO.
pub enum SsoOutcome {
    /// Đã đăng nhập. `created = true` khi vừa tạo tài khoản mới từ SSO.
    LoggedIn { session: Session, created: bool },
    /// Email trùng tài khoản đã có → cần nhập mật khẩu hoặc mã OTP rồi gọi `confirm_link`.
    LinkRequired(LinkRequired),
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// `nguyen.van.a@gmail.com` → `n***@gmail.com`
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let first: String = local.chars().take(1).collect();
            format!("{first}***@{domain}")
        }
        None => "***".to_string(),
    }
}

/// Gợi ý username từ email: chữ thường không dấu, số, `.`, `_`; bắt đầu bằng chữ; 3–24 ký tự.
pub fn username_base(email: &str) -> String {
    let local = email.split('@').next().unwrap_or_default();
    let mut s = String::new();
    for c in deunicode::deunicode(local).to_lowercase().chars() {
        let c = if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' { c } else { '.' };
        if c == '.' && (s.is_empty() || s.ends_with('.')) {
            continue;
        }
        s.push(c);
    }
    let mut s = s.trim_end_matches('.').to_string();
    if !s.starts_with(|c: char| c.is_ascii_lowercase()) {
        s.insert(0, 'u');
    }
    s.truncate(24);
    let mut s = s.trim_end_matches('.').to_string();
    while s.len() < 3 {
        s.push('x');
    }
    s
}

async fn unique_username(ctx: &AppContext, email: &str) -> AppResult<String> {
    let base = username_base(email);
    if !users::username_exists(&ctx.db, &base).await? {
        return Ok(base);
    }
    for _ in 0..20 {
        let n = getrandom::u32().map_err(AppError::internal)? % 10_000;
        let candidate = format!("{base}{n}");
        if !users::username_exists(&ctx.db, &candidate).await? {
            return Ok(candidate);
        }
    }
    Err(AppError::internal("không tạo được username duy nhất"))
}

fn link_expired(provider: Option<SsoProvider>) -> AppError {
    let label = provider.map(SsoProvider::label).unwrap_or("Google / Microsoft");
    AppError::bad_request(
        "SSO_LINK_EXPIRED",
        format!("Yêu cầu liên kết đã hết hạn hoặc không hợp lệ. Hãy đăng nhập lại bằng {label}."),
    )
}

fn too_many_attempts() -> AppError {
    AppError::bad_request("TOO_MANY_ATTEMPTS", "Nhập sai quá nhiều lần, yêu cầu liên kết đã bị hủy. Hãy đăng nhập lại.")
}

fn link_code_hash(token_hash: &str, code: &str) -> String {
    sha256_hex(&format!("{token_hash}:{code}"))
}

/// Ghi liên kết; đổi lỗi UNIQUE thành thông báo dễ hiểu.
pub(super) async fn link_identity(
    conn: &mut sqlx::PgConnection,
    user_id: Uuid,
    provider: SsoProvider,
    subject: &str,
    email: Option<&str>,
) -> AppResult<()> {
    repo::insert_identity(&mut *conn, user_id, provider.as_str(), subject, email).await.map_err(|e| {
        if is_unique_violation(&e, "user_identities_provider_subject_key") {
            already_linked_elsewhere(provider)
        } else if is_unique_violation(&e, "user_identities_user_provider_key") {
            provider_already_linked(provider)
        } else {
            e.into()
        }
    })
}

pub(super) fn already_linked_elsewhere(provider: SsoProvider) -> AppError {
    AppError::conflict(
        "SSO_ALREADY_LINKED",
        format!("Tài khoản {} này đã liên kết với một tài khoản khác.", provider.label()),
    )
}

fn provider_already_linked(provider: SsoProvider) -> AppError {
    AppError::conflict(
        "SSO_PROVIDER_ALREADY_LINKED",
        format!("Tài khoản của bạn đã liên kết với một tài khoản {} khác.", provider.label()),
    )
}

// ---------- Đăng nhập ----------

/// Gọi sau khi nhà cung cấp đã xác thực người dùng (phần OIDC).
pub async fn complete_login(
    ctx: &AppContext,
    identity: ProviderIdentity,
    client: &ClientInfo,
) -> AppResult<SsoOutcome> {
    let provider = identity.provider;
    let email_claim = identity.email.as_deref().map(normalize_email);

    // 1. Danh tính đã liên kết → đăng nhập.
    if let Some(linked) = repo::find_identity(&ctx.db, provider.as_str(), &identity.subject).await? {
        let user = users::find_by_id(&ctx.db, linked.user_id)
            .await?
            .ok_or_else(|| AppError::internal("liên kết SSO trỏ tới user không tồn tại"))?;
        if user.status == UserStatus::Locked {
            return Err(auth::account_locked());
        }
        let mut tx = ctx.db.begin().await?;
        repo::touch_identity(&mut *tx, linked.id, email_claim.as_deref()).await?;
        users::touch_login(&mut *tx, user.id).await?;
        let session = auth::start_session(ctx, &mut tx, &user, client).await?;
        tx.commit().await?;
        return Ok(SsoOutcome::LoggedIn { session, created: false });
    }

    // 2. Chỉ tin email đã được nhà cung cấp xác minh.
    let Some(email) = email_claim.filter(|_| identity.email_verified) else {
        return Err(AppError::bad_request(
            "SSO_EMAIL_UNVERIFIED",
            format!("Tài khoản {} chưa xác minh email. Hãy đăng ký bằng email và mật khẩu.", provider.label()),
        ));
    };

    match users::find_by_email(&ctx.db, &email).await? {
        // 3. Chưa có tài khoản → tạo mới.
        None => create_account(ctx, &identity, &email, client).await,
        Some(user) => match user.status {
            UserStatus::Locked => Err(auth::account_locked()),
            // 5. Tài khoản chưa xác minh → chủ email thật giành lại.
            UserStatus::PendingVerification => claim_unverified(ctx, &identity, &email, user, client).await,
            // 4. Tài khoản đã xác minh → phải chứng minh sở hữu.
            UserStatus::Active => {
                let linked = repo::list_for_user(&ctx.db, user.id).await?;
                if linked.iter().any(|i| i.provider == provider.as_str()) {
                    return Err(provider_already_linked(provider));
                }
                request_link(ctx, &identity, &user).await
            }
        },
    }
}

async fn create_account(
    ctx: &AppContext,
    identity: &ProviderIdentity,
    email: &str,
    client: &ClientInfo,
) -> AppResult<SsoOutcome> {
    let username = unique_username(ctx, email).await?;
    debug_assert!(validate_username(&username).is_ok());
    let full_name = identity
        .full_name
        .as_deref()
        .map(str::trim)
        .filter(|n| (2..=100).contains(&n.chars().count()))
        .map(str::to_string)
        .unwrap_or_else(|| email.split('@').next().unwrap_or("Học sinh").to_string());

    let mut tx = ctx.db.begin().await?;
    let user = users::create(
        &mut *tx,
        &NewUser {
            username: &username,
            email,
            full_name: &full_name,
            password_hash: None,
            role: UserRole::Student,
            status: UserStatus::Active,
            email_verified: true,
        },
    )
    .await?;
    link_identity(&mut tx, user.id, identity.provider, &identity.subject, Some(email)).await?;
    users::touch_login(&mut *tx, user.id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(user.id),
            action: "identity.signup",
            target_type: "user",
            target_id: Some(user.id),
            data: json!({ "provider": identity.provider, "username": user.username }),
            ip: client.ip.as_deref(),
        },
    )
    .await?;
    let session = auth::start_session(ctx, &mut tx, &user, client).await?;
    tx.commit().await?;
    tracing::info!(user_id = %user.id, provider = %identity.provider, "tạo tài khoản mới từ SSO");
    Ok(SsoOutcome::LoggedIn { session, created: true })
}

async fn claim_unverified(
    ctx: &AppContext,
    identity: &ProviderIdentity,
    email: &str,
    user: User,
    client: &ClientInfo,
) -> AppResult<SsoOutcome> {
    let mut tx = ctx.db.begin().await?;
    users::clear_password(&mut *tx, user.id).await?;
    users::mark_email_verified(&mut *tx, user.id).await?;
    auth::revoke_credentials(&mut tx, user.id).await?;
    link_identity(&mut tx, user.id, identity.provider, &identity.subject, Some(email)).await?;
    users::touch_login(&mut *tx, user.id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(user.id),
            action: "identity.claim_unverified",
            target_type: "user",
            target_id: Some(user.id),
            data: json!({ "provider": identity.provider }),
            ip: client.ip.as_deref(),
        },
    )
    .await?;
    let user = users::find_by_id(&mut *tx, user.id).await?.ok_or_else(|| AppError::internal("user biến mất"))?;
    let session = auth::start_session(ctx, &mut tx, &user, client).await?;
    tx.commit().await?;
    tracing::warn!(user_id = %user.id, "chủ email giành lại tài khoản chưa xác minh qua SSO");
    Ok(SsoOutcome::LoggedIn { session, created: false })
}

async fn request_link(ctx: &AppContext, identity: &ProviderIdentity, user: &User) -> AppResult<SsoOutcome> {
    repo::delete_expired_link_requests(&ctx.db).await?;
    let token = random_token();
    repo::insert_link_request(
        &ctx.db,
        &NewLinkRequest {
            token_hash: &sha256_hex(&token),
            user_id: user.id,
            provider: identity.provider.as_str(),
            subject: &identity.subject,
            email: &user.email,
            expires_at: Utc::now() + LINK_TTL,
        },
    )
    .await?;
    Ok(SsoOutcome::LinkRequired(LinkRequired {
        link_token: token,
        provider: identity.provider,
        email_hint: mask_email(&user.email),
        can_use_password: user.password_hash.is_some(),
        expires_in: LINK_TTL.num_seconds(),
    }))
}

// ---------- Xác nhận liên kết ----------

async fn load_link(ctx: &AppContext, token: &str) -> AppResult<(String, LinkRequest, SsoProvider)> {
    let token_hash = sha256_hex(token.trim());
    let lr = repo::find_link_request(&ctx.db, &token_hash).await?.ok_or_else(|| link_expired(None))?;
    let provider: SsoProvider = lr.provider.parse().map_err(|_| AppError::internal("nhà cung cấp SSO lạ"))?;
    if lr.expires_at < Utc::now() {
        repo::delete_link_request(&ctx.db, &token_hash).await?;
        return Err(link_expired(Some(provider)));
    }
    Ok((token_hash, lr, provider))
}

/// Gửi mã OTP về email của tài khoản cần liên kết (cách nhau ít nhất 60 giây).
pub async fn send_link_code(ctx: &AppContext, req: LinkTokenRequest, client: &ClientInfo) -> AppResult<()> {
    req.validate()?;
    ctx.limiter
        .hit_and_check(
            &format!("sso-code:{}", client.ip.as_deref().unwrap_or("unknown")),
            10,
            Duration::from_secs(3600),
        )
        .await?;
    let (token_hash, lr, provider) = load_link(ctx, &req.token).await?;
    if lr.code_sent_at.is_some_and(|t| Utc::now() - t < CODE_COOLDOWN) {
        return Ok(());
    }
    let user = users::find_by_id(&ctx.db, lr.user_id).await?.ok_or_else(|| link_expired(Some(provider)))?;
    let code = random_otp();
    repo::set_link_code(&ctx.db, &token_hash, &link_code_hash(&token_hash, &code)).await?;
    let email = mail::sso_link_code(&user.email, &user.full_name, &code, provider.label());
    if let Err(e) = ctx.jobs.publish(Job::SendEmail(email)).await {
        tracing::error!(error = %e, "không đẩy được email mã liên kết vào hàng đợi");
    }
    Ok(())
}

/// Xác nhận bằng mật khẩu hoặc mã OTP → liên kết và đăng nhập.
pub async fn confirm_link(ctx: &AppContext, req: ConfirmLinkRequest, client: &ClientInfo) -> AppResult<Session> {
    req.validate()?;
    let (token_hash, lr, provider) = load_link(ctx, &req.token).await?;
    if lr.attempts >= LINK_MAX_ATTEMPTS {
        repo::delete_link_request(&ctx.db, &token_hash).await?;
        return Err(too_many_attempts());
    }
    let user = users::find_by_id(&ctx.db, lr.user_id).await?.ok_or_else(|| link_expired(Some(provider)))?;
    if user.status == UserStatus::Locked {
        return Err(auth::account_locked());
    }

    let ok = match (&req.password, &req.code) {
        (Some(password), _) => verify_password(password.clone(), user.password_hash.clone()).await?,
        (None, Some(code)) => lr.code_hash.as_deref() == Some(link_code_hash(&token_hash, code.trim()).as_str()),
        (None, None) => return Err(AppError::field("password", "Nhập mật khẩu hoặc mã xác minh.")),
    };
    if !ok {
        let attempts = repo::increment_link_attempts(&ctx.db, &token_hash).await?;
        if attempts >= LINK_MAX_ATTEMPTS {
            repo::delete_link_request(&ctx.db, &token_hash).await?;
            return Err(too_many_attempts());
        }
        return Err(if req.password.is_some() {
            AppError::unauthorized("INVALID_CREDENTIALS", "Mật khẩu không đúng.")
        } else {
            AppError::bad_request("INVALID_CODE", "Mã xác minh không đúng.")
        });
    }

    let mut tx = ctx.db.begin().await?;
    // Xóa trước: hai request xác nhận song song thì chỉ một request thành công.
    if !repo::delete_link_request(&mut *tx, &token_hash).await? {
        return Err(link_expired(Some(provider)));
    }
    link_identity(&mut tx, user.id, provider, &lr.subject, Some(&lr.email)).await?;
    users::touch_login(&mut *tx, user.id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(user.id),
            action: "identity.link",
            target_type: "user",
            target_id: Some(user.id),
            data: json!({ "provider": provider, "method": if req.password.is_some() { "password" } else { "email_code" } }),
            ip: client.ip.as_deref(),
        },
    )
    .await?;
    let session = auth::start_session(ctx, &mut tx, &user, client).await?;
    tx.commit().await?;
    Ok(session)
}

// ---------- Quản lý liên kết ----------

pub(super) fn parse_provider(provider: &str) -> AppResult<SsoProvider> {
    provider.parse().map_err(|_| AppError::not_found("Không có nhà cung cấp đăng nhập này."))
}

pub async fn list_identities(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<IdentityResponse>> {
    Ok(repo::list_for_user(&ctx.db, actor.id).await?.into_iter().map(Into::into).collect())
}

/// Không cho gỡ liên kết cuối cùng khi tài khoản chưa có mật khẩu (sẽ không đăng nhập được nữa).
async fn ensure_can_unlink(ctx: &AppContext, user: &User) -> AppResult<()> {
    if user.password_hash.is_none() && repo::count_for_user(&ctx.db, user.id).await? <= 1 {
        return Err(AppError::conflict(
            "LAST_LOGIN_METHOD",
            "Tài khoản chưa có mật khẩu. Hãy đặt mật khẩu (dùng chức năng Quên mật khẩu) trước khi gỡ liên kết cuối cùng.",
        ));
    }
    Ok(())
}

fn not_linked() -> AppError {
    AppError::not_found("Tài khoản chưa liên kết với nhà cung cấp này.")
}

/// Người dùng tự gỡ liên kết; các thiết bị khác bị đăng xuất.
pub async fn unlink(ctx: &AppContext, actor: &Actor, provider: &str, ip: Option<&str>) -> AppResult<()> {
    let provider = parse_provider(provider)?;
    let user = users::find_by_id(&ctx.db, actor.id)
        .await?
        .ok_or_else(|| AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập."))?;
    ensure_can_unlink(ctx, &user).await?;
    let mut tx = ctx.db.begin().await?;
    if !repo::delete_identity(&mut *tx, user.id, provider.as_str()).await? {
        return Err(not_linked());
    }
    auth::revoke_other_sessions(&mut *tx, user.id, actor.session_id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "identity.unlink",
            target_type: "user",
            target_id: Some(user.id),
            data: json!({ "provider": provider }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn admin_list_identities(ctx: &AppContext, actor: &Actor, user_id: Uuid) -> AppResult<Vec<IdentityResponse>> {
    actor.require(Permission::UsersRead)?;
    users::find_by_id(&ctx.db, user_id).await?.ok_or_else(|| AppError::not_found("Không tìm thấy người dùng."))?;
    Ok(repo::list_for_user(&ctx.db, user_id).await?.into_iter().map(Into::into).collect())
}

/// Admin gỡ liên kết của một user; mọi phiên của user đó bị thu hồi.
pub async fn admin_unlink(
    ctx: &AppContext,
    actor: &Actor,
    user_id: Uuid,
    provider: &str,
    ip: Option<&str>,
) -> AppResult<()> {
    actor.require(Permission::UsersManage)?;
    let provider = parse_provider(provider)?;
    let user =
        users::find_by_id(&ctx.db, user_id).await?.ok_or_else(|| AppError::not_found("Không tìm thấy người dùng."))?;
    ensure_can_unlink(ctx, &user).await?;
    let mut tx = ctx.db.begin().await?;
    if !repo::delete_identity(&mut *tx, user.id, provider.as_str()).await? {
        return Err(not_linked());
    }
    auth::revoke_all_sessions(&mut *tx, user.id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "identity.unlink",
            target_type: "user",
            target_id: Some(user.id),
            data: json!({ "provider": provider, "byAdmin": true }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn che_email() {
        assert_eq!(mask_email("nguyen.van.a@gmail.com"), "n***@gmail.com");
        assert_eq!(mask_email("x"), "***");
    }

    #[test]
    fn goi_y_username_hop_le() {
        for (email, expected) in [
            ("Nguyen.Van.A@gmail.com", "nguyen.van.a"),
            ("2008hocsinh@x.com", "u2008hocsinh"),
            ("a@x.com", "axx"),
            ("trần-thị+test@x.com", "tran.thi.test"),
            ("..__@x.com", "u__"),
        ] {
            let got = username_base(email);
            assert_eq!(got, expected, "{email}");
            assert!(validate_username(&got).is_ok(), "{got}");
        }
        let long = username_base(&format!("{}@x.com", "a".repeat(50)));
        assert_eq!(long.len(), 24);
    }
}
