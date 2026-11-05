//! SSO: đăng nhập bằng Google / Microsoft (luồng OIDC), xác nhận liên kết tài khoản, quản lý liên kết.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::{delete, get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Serialize;
use sls_core::{
    AppContext, AppResult,
    modules::{
        auth::dto::AuthResponse,
        sso::{
            dto::{ConfirmLinkRequest, IdentityResponse, LinkTokenRequest},
            flow::{self, CallbackOutcome, CallbackParams, FlowMode, StartRequest},
            model::SsoProvider,
            service as sso,
        },
    },
    security::Actor,
};
use uuid::Uuid;

use super::auth::{refresh_cookie, session_response};
use crate::extract::{Client, PathParam, QueryParams, ValidatedJson};

/// Cookie giữ `state` của lượt đăng nhập SSO: `SameSite=Lax` để trình duyệt gửi kèm khi
/// Google / Microsoft chuyển về callback (điều hướng từ trang khác sang).
const STATE_COOKIE: &str = "sls_sso_state";
const STATE_COOKIE_PATH: &str = "/api/auth/sso";

/// Gắn dưới `/api/auth`.
pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/sso/providers", get(providers))
        .route("/sso/{provider}/start", post(start))
        .route("/sso/{provider}/callback", get(callback))
        .route("/sso/link/send-code", post(send_link_code))
        .route("/sso/link", post(confirm_link))
        .route("/identities", get(list_identities))
        .route("/identities/{provider}", delete(unlink))
}

/// Gắn dưới `/api/admin`.
pub fn admin_routes() -> Router<AppContext> {
    Router::new()
        .route("/users/{id}/identities", get(admin_list))
        .route("/users/{id}/identities/{provider}", delete(admin_unlink))
}

#[derive(Serialize)]
struct ProvidersResponse {
    providers: Vec<SsoProvider>,
}

async fn providers(State(ctx): State<AppContext>) -> Json<ProvidersResponse> {
    Json(ProvidersResponse { providers: flow::enabled_providers(&ctx) })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartResponse {
    authorize_url: String,
}

/// Frontend gọi rồi chuyển trình duyệt tới `authorizeUrl`. Chế độ `link` cần đăng nhập.
async fn start(
    State(ctx): State<AppContext>,
    actor: Option<Actor>,
    Client(client): Client,
    PathParam(provider): PathParam<String>,
    jar: CookieJar,
    ValidatedJson(req): ValidatedJson<StartRequest>,
) -> AppResult<(CookieJar, Json<StartResponse>)> {
    let started = flow::start(&ctx, &provider, actor.as_ref(), req, &client).await?;
    let cookie = Cookie::build((STATE_COOKIE, started.state))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(ctx.config.cookie_secure)
        .path(STATE_COOKIE_PATH)
        .max_age(time::Duration::minutes(10))
        .build();
    Ok((jar.add(cookie), Json(StartResponse { authorize_url: started.authorize_url })))
}

/// Nhà cung cấp chuyển trình duyệt về đây. Luôn trả chuyển hướng (303) về một trang của frontend:
/// - đăng nhập xong → `/sso/xong?next=…` (cookie refresh đã đặt, frontend tự khôi phục phiên);
/// - cần xác nhận liên kết → `/sso/lien-ket#token=…&provider=…&email=…&password=1|0`;
/// - liên kết thêm thành công → `/ho-so?ssoLinked=google`;
/// - lỗi → `/dang-nhap?ssoError=MÃ_LỖI` (hoặc `/ho-so?ssoError=…` khi đang liên kết).
async fn callback(
    State(ctx): State<AppContext>,
    Client(client): Client,
    PathParam(provider): PathParam<String>,
    QueryParams(params): QueryParams<CallbackParams>,
    jar: CookieJar,
) -> Response {
    let cookie_state = jar.get(STATE_COOKIE).map(|c| c.value().to_string());
    let res = flow::finish(&ctx, &provider, params, cookie_state.as_deref(), &client).await;
    let jar = jar.remove(Cookie::build((STATE_COOKIE, "")).path(STATE_COOKIE_PATH).build());
    let to = |path: &str, query: &[(&str, &str)], fragment: &[(&str, &str)]| {
        Redirect::to(&flow::frontend_url(&ctx, path, query, fragment))
    };

    match res.result {
        Ok(CallbackOutcome::LoggedIn { session, created, next }) => {
            let mut query = Vec::new();
            if let Some(next) = next.as_deref() {
                query.push(("next", next));
            }
            if created {
                query.push(("new", "1"));
            }
            let jar = jar.add(refresh_cookie(&ctx, session.refresh_token));
            (jar, to("/sso/xong", &query, &[])).into_response()
        }
        Ok(CallbackOutcome::LinkRequired { link, next }) => {
            let expires_in = link.expires_in.to_string();
            let mut fragment = vec![
                ("token", link.link_token.as_str()),
                ("provider", link.provider.as_str()),
                ("email", link.email_hint.as_str()),
                ("password", if link.can_use_password { "1" } else { "0" }),
                ("expiresIn", expires_in.as_str()),
            ];
            if let Some(next) = next.as_deref() {
                fragment.push(("next", next));
            }
            (jar, to("/sso/lien-ket", &[], &fragment)).into_response()
        }
        Ok(CallbackOutcome::Linked { provider }) => {
            (jar, to("/ho-so", &[("ssoLinked", provider.as_str())], &[])).into_response()
        }
        Err(e) => {
            let path = if res.mode == FlowMode::Link { "/ho-so" } else { "/dang-nhap" };
            (jar, to(path, &[("ssoError", e.code)], &[])).into_response()
        }
    }
}

async fn send_link_code(
    State(ctx): State<AppContext>,
    Client(client): Client,
    ValidatedJson(req): ValidatedJson<LinkTokenRequest>,
) -> AppResult<StatusCode> {
    sso::send_link_code(&ctx, req, &client).await?;
    Ok(StatusCode::ACCEPTED)
}

async fn confirm_link(
    State(ctx): State<AppContext>,
    Client(client): Client,
    jar: CookieJar,
    ValidatedJson(req): ValidatedJson<ConfirmLinkRequest>,
) -> AppResult<(CookieJar, Json<AuthResponse>)> {
    let session = sso::confirm_link(&ctx, req, &client).await?;
    Ok(session_response(&ctx, jar, session))
}

async fn list_identities(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<Vec<IdentityResponse>>> {
    Ok(Json(sso::list_identities(&ctx, &actor).await?))
}

async fn unlink(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam(provider): PathParam<String>,
) -> AppResult<StatusCode> {
    sso::unlink(&ctx, &actor, &provider, client.ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn admin_list(
    State(ctx): State<AppContext>,
    actor: Actor,
    PathParam(id): PathParam<Uuid>,
) -> AppResult<Json<Vec<IdentityResponse>>> {
    Ok(Json(sso::admin_list_identities(&ctx, &actor, id).await?))
}

async fn admin_unlink(
    State(ctx): State<AppContext>,
    actor: Actor,
    Client(client): Client,
    PathParam((id, provider)): PathParam<(Uuid, String)>,
) -> AppResult<StatusCode> {
    sso::admin_unlink(&ctx, &actor, id, &provider, client.ip.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}
