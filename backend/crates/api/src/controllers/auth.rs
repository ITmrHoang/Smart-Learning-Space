//! `/api/auth/*` — đăng ký, xác minh email, đăng nhập, phiên, mật khẩu.
//!
//! Refresh token nằm trong cookie `HttpOnly` (`sls_refresh`, chỉ gửi kèm các request `/api/auth/*`);
//! access token trả trong JSON, frontend giữ trong bộ nhớ.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sls_core::{
    AppContext, AppResult,
    modules::auth::{
        dto::{
            AuthResponse, ChangePasswordRequest, EmailRequest, LoginRequest, MeResponse, RegisterRequest,
            RegisterResponse, ResetPasswordRequest, VerifyEmailRequest,
        },
        service::{self as auth, Session},
    },
    security::Actor,
};

use crate::extract::{Client, ValidatedJson};

pub const REFRESH_COOKIE: &str = "sls_refresh";
const COOKIE_PATH: &str = "/api/auth";

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/register", post(register))
        .route("/verify-email", post(verify_email))
        .route("/resend-verification", post(resend_verification))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/logout-all", post(logout_all))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        .route("/change-password", post(change_password))
        .route("/me", get(me))
}

fn refresh_cookie(ctx: &AppContext, token: String) -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Strict)
        .secure(ctx.config.cookie_secure)
        .path(COOKIE_PATH)
        .max_age(time::Duration::seconds(ctx.config.refresh_ttl.as_secs() as i64))
        .build()
}

fn removal_cookie() -> Cookie<'static> {
    Cookie::build((REFRESH_COOKIE, "")).path(COOKIE_PATH).build()
}

pub(crate) fn session_response(ctx: &AppContext, jar: CookieJar, session: Session) -> (CookieJar, Json<AuthResponse>) {
    (jar.add(refresh_cookie(ctx, session.refresh_token)), Json(session.response))
}

async fn register(
    State(ctx): State<AppContext>,
    Client(client): Client,
    ValidatedJson(req): ValidatedJson<RegisterRequest>,
) -> AppResult<(StatusCode, Json<RegisterResponse>)> {
    Ok((StatusCode::CREATED, Json(auth::register(&ctx, req, &client).await?)))
}

async fn verify_email(
    State(ctx): State<AppContext>,
    Client(client): Client,
    jar: CookieJar,
    ValidatedJson(req): ValidatedJson<VerifyEmailRequest>,
) -> AppResult<(CookieJar, Json<AuthResponse>)> {
    let session = auth::verify_email(&ctx, req, &client).await?;
    Ok(session_response(&ctx, jar, session))
}

async fn resend_verification(
    State(ctx): State<AppContext>,
    Client(client): Client,
    ValidatedJson(req): ValidatedJson<EmailRequest>,
) -> AppResult<StatusCode> {
    auth::resend_verification(&ctx, req, &client).await?;
    Ok(StatusCode::ACCEPTED)
}

async fn login(
    State(ctx): State<AppContext>,
    Client(client): Client,
    jar: CookieJar,
    ValidatedJson(req): ValidatedJson<LoginRequest>,
) -> AppResult<(CookieJar, Json<AuthResponse>)> {
    let session = auth::login(&ctx, req, &client).await?;
    Ok(session_response(&ctx, jar, session))
}

/// Refresh lỗi thì xóa luôn cookie để trình duyệt không gửi lại token hỏng.
async fn refresh(State(ctx): State<AppContext>, Client(client): Client, jar: CookieJar) -> Response {
    let token = jar.get(REFRESH_COOKIE).map(|c| c.value().to_string());
    match auth::refresh(&ctx, token.as_deref(), &client).await {
        Ok(session) => session_response(&ctx, jar, session).into_response(),
        Err(e) => (jar.remove(removal_cookie()), e).into_response(),
    }
}

async fn logout(State(ctx): State<AppContext>, jar: CookieJar) -> AppResult<(CookieJar, StatusCode)> {
    auth::logout(&ctx, jar.get(REFRESH_COOKIE).map(|c| c.value())).await?;
    Ok((jar.remove(removal_cookie()), StatusCode::NO_CONTENT))
}

async fn logout_all(State(ctx): State<AppContext>, actor: Actor, jar: CookieJar) -> AppResult<(CookieJar, StatusCode)> {
    auth::logout_all(&ctx, &actor).await?;
    Ok((jar.remove(removal_cookie()), StatusCode::NO_CONTENT))
}

async fn forgot_password(
    State(ctx): State<AppContext>,
    Client(client): Client,
    ValidatedJson(req): ValidatedJson<EmailRequest>,
) -> AppResult<StatusCode> {
    auth::forgot_password(&ctx, req, &client).await?;
    Ok(StatusCode::ACCEPTED)
}

async fn reset_password(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<ResetPasswordRequest>,
) -> AppResult<StatusCode> {
    auth::reset_password(&ctx, req).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn change_password(
    State(ctx): State<AppContext>,
    actor: Actor,
    ValidatedJson(req): ValidatedJson<ChangePasswordRequest>,
) -> AppResult<StatusCode> {
    auth::change_password(&ctx, &actor, req).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn me(State(ctx): State<AppContext>, actor: Actor) -> AppResult<Json<MeResponse>> {
    Ok(Json(auth::me(&ctx, &actor).await?))
}
