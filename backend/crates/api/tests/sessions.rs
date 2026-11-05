//! M6.2: giới hạn số thiết bị đăng nhập cùng lúc, đặt mật khẩu lần đầu cho tài khoản tạo từ SSO.

mod common;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp};
use serde_json::json;
use sls_core::{config::Config, modules::users::model::UserRole, rate_limit::RateLimiter};
use sqlx::PgPool;

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn thiet_bi_thu_ba_dang_xuat_thiet_bi_lau_khong_dung_nhat(pool: PgPool) {
    let app = TestApp::new(pool);
    assert_eq!(app.ctx.config.max_sessions, 2);
    let id = app.create_user("an", UserRole::Student).await;

    let a = app.login("an", PASSWORD).await.refresh_cookie().unwrap();
    let b = app.login("an", PASSWORD).await.refresh_cookie().unwrap();
    // Thiết bị A dùng lại sau B → B thành thiết bị lâu không dùng nhất. Xoay vòng token không tính là thiết bị mới.
    let a = app.refresh(&a).await.refresh_cookie().unwrap();
    let a = app.refresh(&a).await.refresh_cookie().unwrap();

    let c = app.login("an", PASSWORD).await.refresh_cookie().unwrap();
    let res = app.refresh(&b).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.code(), "SESSION_LIMIT");
    assert_eq!(res.refresh_cookie().as_deref(), Some(""), "cookie hỏng bị xóa");

    assert_eq!(app.refresh(&a).await.status, StatusCode::OK);
    assert_eq!(app.refresh(&c).await.status, StatusCode::OK);
    assert_eq!(sls_core::modules::auth::service::active_session_count(&app.ctx, id).await.unwrap(), 2);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn gioi_han_bang_0_la_khong_gioi_han(pool: PgPool) {
    let mut config = Config::for_tests("postgres://test");
    config.max_sessions = 0;
    let app = TestApp::with_config(pool, config, RateLimiter::Disabled);
    let id = app.create_user("an", UserRole::Student).await;
    let first = app.login("an", PASSWORD).await.refresh_cookie().unwrap();
    for _ in 0..4 {
        app.login("an", PASSWORD).await;
    }
    assert_eq!(app.refresh(&first).await.status, StatusCode::OK);
    assert_eq!(sls_core::modules::auth::service::active_session_count(&app.ctx, id).await.unwrap(), 5);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tai_khoan_sso_dat_mat_khau_lan_dau(pool: PgPool) {
    let (app, fake) = TestApp::with_sso(pool).await;
    let (url, cookie) = {
        let res = app.post("/api/auth/sso/google/start", json!({}), None).await;
        (res.body["authorizeUrl"].as_str().unwrap().to_string(), res.cookie("sls_sso_state").unwrap())
    };
    let (code, state) = fake.authorize(&url, fake.google_claims("g-9", "chi@gmail.com"));
    let res = app
        .request(
            "GET",
            &format!("/api/auth/sso/google/callback?code={code}&state={state}"),
            None,
            None,
            Some(&format!("sls_sso_state={cookie}")),
        )
        .await;
    let session = app.refresh(&res.refresh_cookie().unwrap()).await;
    let token = session.token();
    let username = session.body["user"]["username"].as_str().unwrap().to_string();

    // Chưa có mật khẩu → không đăng nhập bằng mật khẩu được; đổi mật khẩu cũng không được.
    assert_eq!(app.login(&username, PASSWORD).await.status, StatusCode::UNAUTHORIZED);
    let weak = app.post("/api/auth/set-password", json!({ "newPassword": "123" }), Some(&token)).await;
    assert_eq!(weak.status, StatusCode::UNPROCESSABLE_ENTITY);

    let set = app.post("/api/auth/set-password", json!({ "newPassword": PASSWORD }), Some(&token)).await;
    assert_eq!(set.status, StatusCode::NO_CONTENT, "{:?}", set.body);
    assert_eq!(app.get("/api/auth/me", Some(&token)).await.body["hasPassword"], true);
    assert_eq!(app.login(&username, PASSWORD).await.status, StatusCode::OK);

    let again = app.post("/api/auth/set-password", json!({ "newPassword": "MatKhauMoi456" }), Some(&token)).await;
    assert_eq!(again.status, StatusCode::CONFLICT);
    assert_eq!(again.code(), "PASSWORD_ALREADY_SET");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tai_khoan_da_co_mat_khau_khong_dung_set_password(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, token) = app.login_as("dung", UserRole::Student).await;
    assert_eq!(app.get("/api/auth/me", Some(&token)).await.body["hasPassword"], true);
    let res = app.post("/api/auth/set-password", json!({ "newPassword": "MatKhauMoi456" }), Some(&token)).await;
    assert_eq!(res.code(), "PASSWORD_ALREADY_SET");
    assert_eq!(
        app.post("/api/auth/set-password", json!({ "newPassword": PASSWORD }), None).await.status,
        StatusCode::UNAUTHORIZED
    );
}
