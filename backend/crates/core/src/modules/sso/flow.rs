//! Luồng OIDC (M6.2): chuyển người dùng tới Google / Microsoft rồi xử lý callback.
//!
//! 1. `start`: frontend gọi `POST /api/auth/sso/{provider}/start` → tạo `state` + `nonce` + PKCE, lưu ở
//!    `sso_login_states` 10 phút, trả URL đăng nhập. `state` cũng được đặt vào cookie `HttpOnly` để chỉ
//!    đúng trình duyệt đã bắt đầu mới hoàn tất được (chặn kiểu ép người khác đăng nhập vào tài khoản lạ).
//! 2. `finish`: nhà cung cấp chuyển về `GET /api/auth/sso/{provider}/callback?code&state` → kiểm tra `state`
//!    (khớp cookie, dùng một lần), đổi `code` lấy `id_token`, kiểm tra `id_token`, rồi:
//!    - chế độ đăng nhập → `service::complete_login` (đăng nhập / tạo tài khoản / yêu cầu xác nhận liên kết);
//!    - chế độ liên kết (đã đăng nhập, bấm "Liên kết" trong Hồ sơ) → gắn danh tính vào tài khoản đang dùng.

use chrono::{TimeDelta, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::LinkRequired,
    model::{ProviderIdentity, SsoProvider},
    oidc::pkce_challenge,
    repository::{self as repo, NewLoginState},
    service::{self as sso, SsoOutcome},
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        audit::{model::AuditEntry, service as audit},
        auth::service::{self as auth, ClientInfo, Session},
        users::{model::UserStatus, service as users},
    },
    security::{
        Actor,
        token::{random_token, sha256_hex},
    },
};

const STATE_TTL: TimeDelta = TimeDelta::minutes(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlowMode {
    #[default]
    Login,
    /// Người dùng đang đăng nhập liên kết thêm Google / Microsoft.
    Link,
}

#[derive(Debug, Deserialize, Validate, Default)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    #[serde(default)]
    pub mode: FlowMode,
    /// Trang quay về sau khi đăng nhập (chỉ nhận đường dẫn nội bộ, ví dụ `/lop-hoc`).
    #[validate(length(max = 300))]
    pub next: Option<String>,
}

pub struct StartedFlow {
    pub authorize_url: String,
    /// Giá trị đặt vào cookie `sls_sso_state`.
    pub state: String,
}

/// Tham số nhà cung cấp gửi về callback.
#[derive(Debug, Deserialize, Default)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    /// Ví dụ `access_denied` khi người dùng bấm Hủy ở trang của nhà cung cấp.
    pub error: Option<String>,
}

pub enum CallbackOutcome {
    LoggedIn {
        session: Session,
        created: bool,
        next: Option<String>,
    },
    LinkRequired {
        link: LinkRequired,
        next: Option<String>,
    },
    /// Chế độ liên kết thành công.
    Linked {
        provider: SsoProvider,
    },
}

pub struct CallbackResult {
    pub mode: FlowMode,
    pub result: AppResult<CallbackOutcome>,
}

/// Chỉ nhận đường dẫn nội bộ: bắt đầu bằng `/`, không phải `//` hay `/\` (sẽ thành địa chỉ trang khác).
pub fn sanitize_next(next: Option<&str>) -> Option<String> {
    let next = next?.trim();
    let ok = next.starts_with('/')
        && !next.starts_with("//")
        && !next.starts_with("/\\")
        && next.len() <= 300
        && !next.chars().any(|c| c.is_control());
    ok.then(|| next.to_string())
}

/// Địa chỉ trang frontend để callback chuyển về: `{APP_BASE_URL}{path}?query#fragment`.
/// Dữ liệu nhạy cảm (token liên kết) đặt ở `fragment`: trình duyệt không gửi phần này lên server
/// và không đưa vào header `Referer`.
pub fn frontend_url(ctx: &AppContext, path: &str, query: &[(&str, &str)], fragment: &[(&str, &str)]) -> String {
    let base = format!("{}{path}", ctx.config.app_base_url);
    let Ok(mut url) = reqwest::Url::parse(&base) else {
        return base;
    };
    if !query.is_empty() {
        url.query_pairs_mut().extend_pairs(query);
    }
    if !fragment.is_empty() {
        let mut encoder = reqwest::Url::parse("http://x/").expect("URL hợp lệ");
        encoder.query_pairs_mut().extend_pairs(fragment);
        url.set_fragment(encoder.query());
    }
    url.into()
}

fn not_enabled(provider: SsoProvider) -> AppError {
    AppError::not_found(format!("Đăng nhập bằng {} chưa được bật.", provider.label()))
}

fn state_invalid() -> AppError {
    AppError::bad_request("SSO_STATE_INVALID", "Lượt đăng nhập đã hết hạn hoặc không hợp lệ. Vui lòng thử lại.")
}

/// Các nhà cung cấp đã cấu hình (frontend chỉ hiện nút của các nhà cung cấp này).
pub fn enabled_providers(ctx: &AppContext) -> Vec<SsoProvider> {
    ctx.sso.enabled()
}

pub async fn start(
    ctx: &AppContext,
    provider: &str,
    actor: Option<&Actor>,
    req: StartRequest,
    client: &ClientInfo,
) -> AppResult<StartedFlow> {
    req.validate()?;
    let provider = sso::parse_provider(provider)?;
    let oidc = ctx.sso.get(provider).ok_or_else(|| not_enabled(provider))?;
    ctx.limiter
        .hit_and_check(
            &format!("sso-start:{}", client.ip.as_deref().unwrap_or("unknown")),
            30,
            std::time::Duration::from_secs(600),
        )
        .await?;
    let link_user_id = match req.mode {
        FlowMode::Login => None,
        FlowMode::Link => {
            Some(actor.ok_or_else(|| AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập."))?.id)
        }
    };

    let state = random_token();
    let nonce = random_token();
    let code_verifier = random_token();
    // Tạo URL trước khi ghi DB: nhà cung cấp không kết nối được thì không để lại bản ghi thừa.
    let authorize_url = oidc.authorize_url(&state, &nonce, &pkce_challenge(&code_verifier)).await?;
    let next = sanitize_next(req.next.as_deref());
    repo::delete_expired_login_states(&ctx.db).await?;
    repo::insert_login_state(
        &ctx.db,
        &NewLoginState {
            state_hash: &sha256_hex(&state),
            provider: provider.as_str(),
            nonce: &nonce,
            code_verifier: &code_verifier,
            link_user_id,
            next_path: next.as_deref(),
            expires_at: Utc::now() + STATE_TTL,
        },
    )
    .await?;
    Ok(StartedFlow { authorize_url, state })
}

pub async fn finish(
    ctx: &AppContext,
    provider: &str,
    params: CallbackParams,
    cookie_state: Option<&str>,
    client: &ClientInfo,
) -> CallbackResult {
    let mut mode = FlowMode::Login;
    let result = finish_inner(ctx, provider, params, cookie_state, client, &mut mode).await;
    CallbackResult { mode, result }
}

async fn finish_inner(
    ctx: &AppContext,
    provider: &str,
    params: CallbackParams,
    cookie_state: Option<&str>,
    client: &ClientInfo,
    mode: &mut FlowMode,
) -> AppResult<CallbackOutcome> {
    let provider = sso::parse_provider(provider)?;
    let oidc = ctx.sso.get(provider).ok_or_else(|| not_enabled(provider))?;

    // `state` phải khớp cookie của chính trình duyệt này, và chỉ dùng được một lần.
    let state = params.state.as_deref().filter(|s| !s.is_empty()).ok_or_else(state_invalid)?;
    if cookie_state != Some(state) {
        return Err(state_invalid());
    }
    let saved = repo::take_login_state(&ctx.db, &sha256_hex(state)).await?.ok_or_else(state_invalid)?;
    if saved.link_user_id.is_some() {
        *mode = FlowMode::Link;
    }
    if saved.provider != provider.as_str() || saved.expires_at < Utc::now() {
        return Err(state_invalid());
    }

    if let Some(error) = params.error.as_deref() {
        return Err(if error == "access_denied" {
            AppError::bad_request("SSO_CANCELLED", "Bạn đã hủy đăng nhập.")
        } else {
            tracing::warn!(%provider, error, "nhà cung cấp SSO trả lỗi");
            AppError::bad_request(
                "SSO_FAILED",
                format!("Không đăng nhập được bằng {}. Vui lòng thử lại.", provider.label()),
            )
        });
    }
    let code = params.code.as_deref().filter(|c| !c.is_empty()).ok_or_else(state_invalid)?;
    let identity = oidc.exchange(code, &saved.code_verifier, &saved.nonce).await?;

    match saved.link_user_id {
        Some(user_id) => link_to_user(ctx, user_id, identity, client).await,
        None => Ok(match sso::complete_login(ctx, identity, client).await? {
            SsoOutcome::LoggedIn { session, created } => {
                CallbackOutcome::LoggedIn { session, created, next: saved.next_path }
            }
            SsoOutcome::LinkRequired(link) => CallbackOutcome::LinkRequired { link, next: saved.next_path },
        }),
    }
}

/// Người dùng đã đăng nhập (đã chứng minh sở hữu tài khoản) liên kết thêm một danh tính.
async fn link_to_user(
    ctx: &AppContext,
    user_id: Uuid,
    identity: ProviderIdentity,
    client: &ClientInfo,
) -> AppResult<CallbackOutcome> {
    let provider = identity.provider;
    let user = users::find_by_id(&ctx.db, user_id)
        .await?
        .ok_or_else(|| AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập."))?;
    if user.status == UserStatus::Locked {
        return Err(auth::account_locked());
    }
    if let Some(existing) = repo::find_identity(&ctx.db, provider.as_str(), &identity.subject).await? {
        if existing.user_id == user.id {
            return Ok(CallbackOutcome::Linked { provider });
        }
        return Err(sso::already_linked_elsewhere(provider));
    }
    let email = identity.email.as_deref().map(|e| e.trim().to_lowercase());
    let mut tx = ctx.db.begin().await?;
    sso::link_identity(&mut tx, user.id, provider, &identity.subject, email.as_deref()).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(user.id),
            action: "identity.link",
            target_type: "user",
            target_id: Some(user.id),
            data: json!({ "provider": provider, "method": "session" }),
            ip: client.ip.as_deref(),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(CallbackOutcome::Linked { provider })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chi_nhan_duong_dan_noi_bo() {
        assert_eq!(sanitize_next(Some("/lop-hoc?id=1")), Some("/lop-hoc?id=1".into()));
        for bad in ["//evil.com", "/\\evil.com", "https://evil.com", "lop-hoc", "/a\nb", ""] {
            assert_eq!(sanitize_next(Some(bad)), None, "{bad}");
        }
        assert_eq!(sanitize_next(None), None);
    }
}
