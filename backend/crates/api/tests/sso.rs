//! M6 phần 1 — SSO: chính sách liên kết tài khoản và quản lý liên kết.
//! Danh tính "đã được nhà cung cấp xác thực" được tạo trực tiếp (phần OIDC thật làm ở phần 2).

mod common;

use axum::http::StatusCode;
use common::{PASSWORD, Res, TestApp, otp_from};
use serde_json::json;
use sls_core::{
    AppResult,
    modules::{
        auth::service::{ClientInfo, Session},
        sso::{
            dto::LinkRequired,
            model::{ProviderIdentity, SsoProvider},
            service::{self as sso, SsoOutcome},
        },
        users::model::UserRole,
    },
};
use sqlx::PgPool;

fn identity(provider: SsoProvider, subject: &str, email: Option<&str>, verified: bool) -> ProviderIdentity {
    ProviderIdentity {
        provider,
        subject: subject.to_string(),
        email: email.map(str::to_string),
        email_verified: verified,
        full_name: Some("Nguyễn Văn Google".to_string()),
    }
}

fn google(subject: &str, email: &str) -> ProviderIdentity {
    identity(SsoProvider::Google, subject, Some(email), true)
}

async fn sso_login(app: &TestApp, id: ProviderIdentity) -> AppResult<SsoOutcome> {
    sso::complete_login(&app.ctx, id, &ClientInfo::default()).await
}

fn logged_in(outcome: AppResult<SsoOutcome>) -> (Session, bool) {
    match outcome {
        Ok(SsoOutcome::LoggedIn { session, created }) => (session, created),
        Ok(SsoOutcome::LinkRequired(_)) => panic!("mong đăng nhập, nhận yêu cầu liên kết"),
        Err(e) => panic!("mong đăng nhập, nhận lỗi {e}"),
    }
}

fn link_required(outcome: AppResult<SsoOutcome>) -> LinkRequired {
    match outcome {
        Ok(SsoOutcome::LinkRequired(link)) => link,
        Ok(SsoOutcome::LoggedIn { .. }) => panic!("mong yêu cầu liên kết, nhận đăng nhập"),
        Err(e) => panic!("mong yêu cầu liên kết, nhận lỗi {e}"),
    }
}

fn error_code(outcome: AppResult<SsoOutcome>) -> &'static str {
    match outcome {
        Err(e) => e.code,
        Ok(_) => panic!("mong lỗi"),
    }
}

async fn confirm(app: &TestApp, body: serde_json::Value) -> Res {
    app.post("/api/auth/sso/link", body, None).await
}

/// Tài khoản có mật khẩu đã liên kết Google `g-1`. Trả về response của bước xác nhận (có token + cookie).
async fn linked_password_user(app: &TestApp, username: &str) -> Res {
    app.create_user(username, UserRole::Student).await;
    let link = link_required(sso_login(app, google("g-1", &format!("{username}@example.com"))).await);
    let res = confirm(app, json!({ "token": link.link_token, "password": PASSWORD })).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
    res
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn email_moi_tao_tai_khoan_hoc_sinh(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let (session, created) = logged_in(sso_login(&app, google("g-1", "Moi.HocSinh@gmail.com")).await);
    assert!(created);
    let user = &session.response.user.user;
    assert_eq!(user.email, "moi.hocsinh@gmail.com");
    assert_eq!(user.username, "moi.hocsinh");
    assert_eq!(user.full_name, "Nguyễn Văn Google");
    assert_eq!(user.role, UserRole::Student);
    assert!(user.email_verified);

    let has_password: bool = sqlx::query_scalar("SELECT password_hash IS NOT NULL FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!has_password, "tài khoản tạo từ SSO chưa có mật khẩu");

    // Lần sau: đăng nhập thẳng, không tạo thêm tài khoản.
    let (again, created) = logged_in(sso_login(&app, google("g-1", "moi.hocsinh@gmail.com")).await);
    assert!(!created);
    assert_eq!(again.response.user.user.id, user.id);

    let res = app.get("/api/auth/identities", Some(&session.response.access_token)).await;
    assert_eq!(res.body[0]["provider"], "google");
    assert_eq!(res.body[0]["email"], "moi.hocsinh@gmail.com");

    // Username trùng → thêm số phía sau.
    let (other, _) = logged_in(sso_login(&app, google("g-2", "moi.hocsinh@yahoo.com")).await);
    assert!(other.response.user.user.username.starts_with("moi.hocsinh"));
    assert_ne!(other.response.user.user.username, "moi.hocsinh");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn nha_cung_cap_chua_xac_minh_email(pool: PgPool) {
    let app = TestApp::new(pool);
    let unverified = identity(SsoProvider::Microsoft, "m-1", Some("a@example.com"), false);
    assert_eq!(error_code(sso_login(&app, unverified).await), "SSO_EMAIL_UNVERIFIED");
    let no_email = identity(SsoProvider::Microsoft, "m-2", None, true);
    assert_eq!(error_code(sso_login(&app, no_email).await), "SSO_EMAIL_UNVERIFIED");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn trung_email_phai_nhap_mat_khau_moi_lien_ket(pool: PgPool) {
    let app = TestApp::new(pool);
    let user_id = app.create_user("hocsinh1", UserRole::Student).await;

    let link = link_required(sso_login(&app, google("g-1", "HocSinh1@example.com")).await);
    assert_eq!(link.email_hint, "h***@example.com");
    assert!(link.can_use_password);
    assert_eq!(link.expires_in, 900);

    let res = confirm(&app, json!({ "token": link.link_token, "password": "SaiMatKhau1" })).await;
    assert_eq!((res.status, res.code()), (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"));

    let res = confirm(&app, json!({ "token": link.link_token, "password": PASSWORD })).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
    assert_eq!(res.body["user"]["id"], user_id.to_string());
    assert!(res.refresh_cookie().is_some_and(|c| !c.is_empty()));

    // Đã liên kết → lần sau đăng nhập thẳng; token liên kết đã dùng thì hết hiệu lực.
    let (session, created) = logged_in(sso_login(&app, google("g-1", "hocsinh1@example.com")).await);
    assert!(!created);
    assert_eq!(session.response.user.user.id, user_id);
    let res = confirm(&app, json!({ "token": link.link_token, "password": PASSWORD })).await;
    assert_eq!(res.code(), "SSO_LINK_EXPIRED");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn xac_nhan_lien_ket_bang_ma_otp(pool: PgPool) {
    let app = TestApp::new(pool);
    app.create_user("hocsinh1", UserRole::Student).await;
    let link = link_required(sso_login(&app, google("g-1", "hocsinh1@example.com")).await);

    let res = app.post("/api/auth/sso/link/send-code", json!({ "token": link.link_token }), None).await;
    assert_eq!(res.status, StatusCode::ACCEPTED);
    let emails = app.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].to, "hocsinh1@example.com");
    assert!(emails[0].subject.contains("Google"));
    let code = otp_from(&emails[0]);

    // Gửi lại ngay → chưa đủ 60 giây, không gửi thêm.
    app.post("/api/auth/sso/link/send-code", json!({ "token": link.link_token }), None).await;
    assert!(app.emails().is_empty());

    let wrong = if code == "000000" { "111111" } else { "000000" };
    let res = confirm(&app, json!({ "token": link.link_token, "code": wrong })).await;
    assert_eq!(res.code(), "INVALID_CODE");
    let res = confirm(&app, json!({ "token": link.link_token, "code": code })).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn sai_qua_nhieu_lan_hoac_het_han(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    app.create_user("hocsinh1", UserRole::Student).await;

    let link = link_required(sso_login(&app, google("g-1", "hocsinh1@example.com")).await);
    let res = confirm(&app, json!({ "token": link.link_token })).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "thiếu cả mật khẩu lẫn mã");
    for _ in 0..4 {
        assert_eq!(
            confirm(&app, json!({ "token": link.link_token, "password": "Sai12345" })).await.code(),
            "INVALID_CREDENTIALS"
        );
    }
    assert_eq!(
        confirm(&app, json!({ "token": link.link_token, "password": "Sai12345" })).await.code(),
        "TOO_MANY_ATTEMPTS"
    );
    assert_eq!(
        confirm(&app, json!({ "token": link.link_token, "password": PASSWORD })).await.code(),
        "SSO_LINK_EXPIRED"
    );

    let link = link_required(sso_login(&app, google("g-1", "hocsinh1@example.com")).await);
    sqlx::query("UPDATE sso_link_requests SET expires_at = now() - interval '1 second'").execute(&pool).await.unwrap();
    assert_eq!(
        confirm(&app, json!({ "token": link.link_token, "password": PASSWORD })).await.code(),
        "SSO_LINK_EXPIRED"
    );
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tai_khoan_chua_xac_minh_bi_chu_email_gianh_lai(pool: PgPool) {
    let app = TestApp::new(pool);
    // Ai đó đăng ký trước bằng email của người khác và không xác minh được.
    assert_eq!(app.register("kecap", "nannhan@example.com").await.status, StatusCode::CREATED);
    let stale_code = otp_from(&app.emails()[0]);

    let (session, created) = logged_in(sso_login(&app, google("g-1", "nannhan@example.com")).await);
    assert!(!created);
    assert_eq!(session.response.user.user.status, sls_core::modules::users::model::UserStatus::Active);

    // Mật khẩu và mã OTP của người đăng ký trước không còn dùng được.
    assert_eq!(app.login("kecap", PASSWORD).await.code(), "INVALID_CREDENTIALS");
    let res =
        app.post("/api/auth/verify-email", json!({ "email": "nannhan@example.com", "code": stale_code }), None).await;
    assert_ne!(res.status, StatusCode::OK);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tai_khoan_bi_khoa_khong_dang_nhap_sso(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let id = app.create_user("hocsinh1", UserRole::Student).await;
    sqlx::query("UPDATE users SET status = 'locked' WHERE id = $1").bind(id).execute(&pool).await.unwrap();
    assert_eq!(error_code(sso_login(&app, google("g-1", "hocsinh1@example.com")).await), "ACCOUNT_LOCKED");

    // Đã liên kết rồi mới bị khóa.
    let (session, _) = logged_in(sso_login(&app, google("g-9", "khac@example.com")).await);
    sqlx::query("UPDATE users SET status = 'locked' WHERE id = $1")
        .bind(session.response.user.user.id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(error_code(sso_login(&app, google("g-9", "khac@example.com")).await), "ACCOUNT_LOCKED");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn moi_nha_cung_cap_mot_lien_ket(pool: PgPool) {
    let app = TestApp::new(pool);
    linked_password_user(&app, "hocsinh1").await;

    // Một tài khoản Google khác có cùng email → không liên kết thêm.
    assert_eq!(error_code(sso_login(&app, google("g-2", "hocsinh1@example.com")).await), "SSO_PROVIDER_ALREADY_LINKED");
    // Nhà cung cấp khác thì vẫn liên kết được (sau khi xác minh).
    let microsoft = identity(SsoProvider::Microsoft, "m-1", Some("hocsinh1@example.com"), true);
    link_required(sso_login(&app, microsoft).await);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tu_go_lien_ket(pool: PgPool) {
    let app = TestApp::new(pool);

    // Tài khoản chỉ có SSO, chưa có mật khẩu → không gỡ được liên kết cuối cùng.
    let (sso_only, _) = logged_in(sso_login(&app, google("g-5", "chisso@example.com")).await);
    let res =
        app.send("DELETE", "/api/auth/identities/google", None, Some(&sso_only.response.access_token), None).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "LAST_LOGIN_METHOD"));

    // Tài khoản có mật khẩu: gỡ được, thiết bị khác bị đăng xuất, thiết bị đang dùng vẫn giữ phiên.
    let other_device = linked_password_user(&app, "hocsinh1").await.refresh_cookie().unwrap();
    let current = app.login("hocsinh1", PASSWORD).await;
    let (token, cookie) = (current.token(), current.refresh_cookie().unwrap());

    let res = app.send("DELETE", "/api/auth/identities/google", None, Some(&token), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(app.refresh(&other_device).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(app.refresh(&cookie).await.status, StatusCode::OK);
    assert!(app.get("/api/auth/identities", Some(&token)).await.body.as_array().unwrap().is_empty());
    link_required(sso_login(&app, google("g-1", "hocsinh1@example.com")).await);

    let res = app.send("DELETE", "/api/auth/identities/google", None, Some(&token), None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = app.send("DELETE", "/api/auth/identities/facebook", None, Some(&token), None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn admin_xem_va_go_lien_ket(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let (_, teacher) = app.login_as("giaovien1", UserRole::Teacher).await;
    let linked = linked_password_user(&app, "hocsinh1").await;
    let user_id = linked.body["user"]["id"].as_str().unwrap().to_string();
    let cookie = linked.refresh_cookie().unwrap();

    let path = format!("/api/admin/users/{user_id}/identities");
    let res = app.get(&path, Some(&admin)).await;
    assert_eq!(res.body.as_array().unwrap().len(), 1);
    assert_eq!(app.get(&path, Some(&teacher)).await.status, StatusCode::FORBIDDEN);

    let res = app.send("DELETE", &format!("{path}/google"), None, Some(&admin), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(app.refresh(&cookie).await.status, StatusCode::UNAUTHORIZED, "mọi phiên của user bị thu hồi");

    let logs = app.get("/api/admin/audit-logs?action=identity.unlink", Some(&admin)).await;
    assert_eq!(logs.body["total"], 1);
    assert_eq!(logs.body["items"][0]["data"]["byAdmin"], true);
    let logs = app.get("/api/admin/audit-logs?action=identity.link", Some(&admin)).await;
    assert_eq!(logs.body["items"][0]["data"]["method"], "password");
}
