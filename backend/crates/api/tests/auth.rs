//! M1 — đăng ký, OTP, đăng nhập, phiên, mật khẩu. Số thứ tự theo `docs/implementation-plan.md`.

mod common;

use axum::http::StatusCode;
use chrono::Utc;
use common::{PASSWORD, TestApp, otp_from, reset_token_from};
use serde_json::json;
use sls_core::{modules::users::model::UserRole, rate_limit::RateLimiter, security::jwt::Claims};
use sqlx::PgPool;
use uuid::Uuid;

// 1
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn dang_ky_hop_le_gui_otp(pool: PgPool) {
    let app = TestApp::new(pool);
    let res = app.register("hocsinh1", "HS1@Example.com").await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.body);
    assert_eq!(res.body["verificationRequired"], true);
    assert_eq!(res.body["email"], "hs1@example.com", "email được chuẩn hóa chữ thường");

    let emails = app.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].to, "hs1@example.com");
    assert_eq!(otp_from(&emails[0]).len(), 6);
}

// 2
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn trung_username_hoac_email(pool: PgPool) {
    let app = TestApp::new(pool);
    assert_eq!(app.register("hocsinh1", "a@example.com").await.status, StatusCode::CREATED);

    let res = app.register("hocsinh1", "b@example.com").await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "USERNAME_TAKEN"));

    let res = app.register("hocsinh2", "A@EXAMPLE.COM").await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "EMAIL_TAKEN"));
}

// 3
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn du_lieu_sai_tra_loi_tung_truong(pool: PgPool) {
    let app = TestApp::new(pool);
    let res = app
        .post(
            "/api/auth/register",
            json!({ "username": "A", "email": "sai-email", "fullName": "N", "password": "123" }),
            None,
        )
        .await;
    assert_eq!((res.status, res.code()), (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_FAILED"));
    let fields = res.body["error"]["fields"].as_object().unwrap();
    for f in ["username", "email", "fullName", "password"] {
        assert!(fields.contains_key(f), "thiếu lỗi trường {f}: {fields:?}");
    }
}

// 4
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn otp_sai_nhieu_lan_thi_bi_huy(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register("hocsinh1", "hs1@example.com").await;
    let code = otp_from(&app.emails()[0]);
    let wrong = if code == "000000" { "111111" } else { "000000" };

    for _ in 0..4 {
        let res = app.post("/api/auth/verify-email", json!({ "email": "hs1@example.com", "code": wrong }), None).await;
        assert_eq!(res.code(), "INVALID_CODE");
    }
    let res = app.post("/api/auth/verify-email", json!({ "email": "hs1@example.com", "code": wrong }), None).await;
    assert_eq!(res.code(), "TOO_MANY_ATTEMPTS");
    // Mã đúng cũng không dùng được nữa.
    let res = app.post("/api/auth/verify-email", json!({ "email": "hs1@example.com", "code": code }), None).await;
    assert_eq!(res.code(), "INVALID_CODE");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn otp_het_han(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    app.register("hocsinh1", "hs1@example.com").await;
    let code = otp_from(&app.emails()[0]);
    sqlx::query("UPDATE email_verifications SET expires_at = now() - interval '1 minute'")
        .execute(&pool)
        .await
        .unwrap();
    let res = app.post("/api/auth/verify-email", json!({ "email": "hs1@example.com", "code": code }), None).await;
    assert_eq!((res.status, res.code()), (StatusCode::BAD_REQUEST, "CODE_EXPIRED"));
}

// 5
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn otp_dung_thi_kich_hoat_va_dang_nhap(pool: PgPool) {
    let app = TestApp::new(pool);
    let res = app.register_verified("hocsinh1").await;
    assert!(!res.token().is_empty());
    assert_eq!(res.body["user"]["status"], "active");
    assert_eq!(res.body["user"]["emailVerified"], true);
    assert_eq!(res.body["user"]["permissions"], json!(["teacher_applications.submit"]));

    let cookie = res.set_cookie_header().unwrap();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    assert!(cookie.contains("Path=/api/auth"));

    // Xác minh lần nữa → đã xác minh rồi.
    let again =
        app.post("/api/auth/verify-email", json!({ "email": "hocsinh1@example.com", "code": "123456" }), None).await;
    assert_eq!(again.code(), "ALREADY_VERIFIED");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn gui_lai_otp_co_thoi_gian_cho(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    app.register("hocsinh1", "hs1@example.com").await;
    app.emails();
    let res = app.post("/api/auth/resend-verification", json!({ "email": "hs1@example.com" }), None).await;
    assert_eq!(res.status, StatusCode::ACCEPTED);
    assert!(app.emails().is_empty(), "chưa đủ 60 giây thì không gửi lại");

    sqlx::query("UPDATE email_verifications SET sent_at = now() - interval '2 minutes'").execute(&pool).await.unwrap();
    app.post("/api/auth/resend-verification", json!({ "email": "hs1@example.com" }), None).await;
    assert_eq!(app.emails().len(), 1);

    // Email không tồn tại vẫn 202, không gửi gì.
    let res = app.post("/api/auth/resend-verification", json!({ "email": "khongco@example.com" }), None).await;
    assert_eq!(res.status, StatusCode::ACCEPTED);
    assert!(app.emails().is_empty());
}

// 6
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn dang_nhap_khi_chua_xac_minh(pool: PgPool) {
    let app = TestApp::new(pool);
    app.register("hocsinh1", "hs1@example.com").await;
    let res = app.login("hocsinh1", PASSWORD).await;
    assert_eq!((res.status, res.code()), (StatusCode::FORBIDDEN, "EMAIL_NOT_VERIFIED"));
}

// 7, 8
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn dang_nhap_bang_username_hoac_email(pool: PgPool) {
    let app = TestApp::new(pool);
    app.create_user("giaovien1", UserRole::Teacher).await;

    let res = app.login("GiaoVien1", PASSWORD).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["tokenType"], "Bearer");
    assert_eq!(res.body["expiresIn"], 900);
    assert_eq!(res.body["user"]["role"], "teacher");
    assert!(res.refresh_cookie().is_some_and(|c| c.len() > 20));

    assert_eq!(app.login("giaovien1@example.com", PASSWORD).await.status, StatusCode::OK);

    let wrong_pass = app.login("giaovien1", "SaiMatKhau1").await;
    let no_user = app.login("khongton tai", PASSWORD).await;
    assert_eq!((wrong_pass.status, wrong_pass.code()), (StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS"));
    assert_eq!(wrong_pass.body, no_user.body, "không để lộ user nào tồn tại");
}

// 9
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tai_khoan_bi_khoa(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    app.create_user("hocsinh1", UserRole::Student).await;
    sqlx::query("UPDATE users SET status = 'locked'").execute(&pool).await.unwrap();
    let res = app.login("hocsinh1", PASSWORD).await;
    assert_eq!((res.status, res.code()), (StatusCode::FORBIDDEN, "ACCOUNT_LOCKED"));
}

// 10
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn dang_nhap_sai_qua_nhieu_lan_bi_chan(pool: PgPool) {
    let app = TestApp::with_limiter(pool, RateLimiter::memory());
    app.create_user("hocsinh1", UserRole::Student).await;
    for _ in 0..10 {
        assert_eq!(app.login("hocsinh1", "SaiMatKhau1").await.code(), "INVALID_CREDENTIALS");
    }
    let res = app.login("hocsinh1", PASSWORD).await;
    assert_eq!((res.status, res.code()), (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"));
    assert!(res.headers.contains_key("retry-after"));
}

// 11, 12
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn refresh_xoay_vong_va_phat_hien_dung_lai(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    app.create_user("hocsinh1", UserRole::Student).await;
    let c1 = app.login("hocsinh1", PASSWORD).await.refresh_cookie().unwrap();

    let res = app.refresh(&c1).await;
    assert_eq!(res.status, StatusCode::OK);
    let c2 = res.refresh_cookie().unwrap();
    assert_ne!(c1, c2);
    assert_eq!(app.get("/api/auth/me", Some(&res.token())).await.status, StatusCode::OK);

    // Dùng lại c1 ngay sau khi xoay (hai tab cùng refresh): bị từ chối nhưng không thu hồi phiên.
    let res = app.refresh(&c1).await;
    assert_eq!((res.status, res.code()), (StatusCode::UNAUTHORIZED, "INVALID_REFRESH_TOKEN"));
    assert_eq!(res.refresh_cookie().as_deref(), Some(""), "cookie hỏng bị xóa");
    let res = app.refresh(&c2).await;
    assert_eq!(res.status, StatusCode::OK);
    let c3 = res.refresh_cookie().unwrap();

    // Dùng lại token cũ sau khoảng ân hạn → coi như bị đánh cắp: thu hồi cả chuỗi phiên.
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() - interval '1 hour' WHERE revoked_at IS NOT NULL")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(app.refresh(&c1).await.code(), "REFRESH_TOKEN_REUSED");
    assert_eq!(app.refresh(&c3).await.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn refresh_khong_co_cookie_hoac_het_han(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let res = app.send("POST", "/api/auth/refresh", None, None, None).await;
    assert_eq!(res.code(), "NO_REFRESH_TOKEN");
    assert_eq!(app.refresh("token-khong-ton-tai").await.code(), "INVALID_REFRESH_TOKEN");

    app.create_user("hocsinh1", UserRole::Student).await;
    let c = app.login("hocsinh1", PASSWORD).await.refresh_cookie().unwrap();
    sqlx::query("UPDATE refresh_tokens SET expires_at = now() - interval '1 second'").execute(&pool).await.unwrap();
    assert_eq!(app.refresh(&c).await.code(), "SESSION_EXPIRED");
}

// 13
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn dang_xuat(pool: PgPool) {
    let app = TestApp::new(pool);
    app.create_user("hocsinh1", UserRole::Student).await;
    let c1 = app.login("hocsinh1", PASSWORD).await.refresh_cookie().unwrap();
    let login2 = app.login("hocsinh1", PASSWORD).await;
    let (c2, token2) = (login2.refresh_cookie().unwrap(), login2.token());

    let res = app.send("POST", "/api/auth/logout", None, None, Some(&c1)).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(app.refresh(&c1).await.status, StatusCode::UNAUTHORIZED);
    // Phiên khác vẫn còn.
    let res = app.refresh(&c2).await;
    assert_eq!(res.status, StatusCode::OK);
    let c2 = res.refresh_cookie().unwrap();

    // Đăng xuất mọi thiết bị.
    let res = app.send("POST", "/api/auth/logout-all", None, Some(&token2), None).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(app.refresh(&c2).await.status, StatusCode::UNAUTHORIZED);
}

// 14, 15
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn quen_va_dat_lai_mat_khau(pool: PgPool) {
    let app = TestApp::new(pool);
    app.create_user("hocsinh1", UserRole::Student).await;
    let old_cookie = app.login("hocsinh1", PASSWORD).await.refresh_cookie().unwrap();

    let res = app.post("/api/auth/forgot-password", json!({ "email": "khongco@example.com" }), None).await;
    assert_eq!(res.status, StatusCode::ACCEPTED);
    assert!(app.emails().is_empty());

    let res = app.post("/api/auth/forgot-password", json!({ "email": "hocsinh1@example.com" }), None).await;
    assert_eq!(res.status, StatusCode::ACCEPTED);
    let emails = app.emails();
    assert_eq!(emails.len(), 1);
    assert!(emails[0].text.contains("/dat-lai-mat-khau?token="));
    let token = reset_token_from(&emails[0]);

    let body = json!({ "token": token, "newPassword": "MatKhauMoi456" });
    assert_eq!(app.post("/api/auth/reset-password", body.clone(), None).await.status, StatusCode::NO_CONTENT);
    assert_eq!(app.login("hocsinh1", PASSWORD).await.code(), "INVALID_CREDENTIALS");
    assert_eq!(app.login("hocsinh1", "MatKhauMoi456").await.status, StatusCode::OK);
    assert_eq!(app.refresh(&old_cookie).await.status, StatusCode::UNAUTHORIZED, "phiên cũ bị thu hồi");

    let res = app.post("/api/auth/reset-password", body, None).await;
    assert_eq!((res.status, res.code()), (StatusCode::BAD_REQUEST, "INVALID_RESET_TOKEN"));
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn link_dat_lai_cu_bi_vo_hieu_khi_xin_link_moi(pool: PgPool) {
    let app = TestApp::new(pool);
    app.create_user("hocsinh1", UserRole::Student).await;
    app.post("/api/auth/forgot-password", json!({ "email": "hocsinh1@example.com" }), None).await;
    let first = reset_token_from(&app.emails()[0]);
    app.post("/api/auth/forgot-password", json!({ "email": "hocsinh1@example.com" }), None).await;
    let res =
        app.post("/api/auth/reset-password", json!({ "token": first, "newPassword": "MatKhauMoi456" }), None).await;
    assert_eq!(res.code(), "INVALID_RESET_TOKEN");
}

// 16
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn me_can_token_hop_le(pool: PgPool) {
    let app = TestApp::new(pool);
    assert_eq!(app.get("/api/auth/me", None).await.code(), "UNAUTHENTICATED");
    assert_eq!(app.get("/api/auth/me", Some("abc.def.ghi")).await.code(), "INVALID_TOKEN");
    let res = app.send("GET", "/api/auth/me", None, None, None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    let (id, token) = app.login_as("hocsinh1", UserRole::Student).await;
    let res = app.get("/api/auth/me", Some(&token)).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["id"], id.to_string());
    assert!(res.body.get("passwordHash").is_none(), "không lộ hash mật khẩu");

    let now = Utc::now().timestamp();
    let expired = app
        .ctx
        .jwt
        .encode(&Claims { sub: id, role: UserRole::Student, sid: Uuid::now_v7(), iat: now - 2000, exp: now - 1000 })
        .unwrap();
    assert_eq!(app.get("/api/auth/me", Some(&expired)).await.code(), "TOKEN_EXPIRED");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn doi_mat_khau(pool: PgPool) {
    let app = TestApp::new(pool);
    app.create_user("hocsinh1", UserRole::Student).await;
    let other_cookie = app.login("hocsinh1", PASSWORD).await.refresh_cookie().unwrap();
    let current = app.login("hocsinh1", PASSWORD).await;
    let (token, cookie) = (current.token(), current.refresh_cookie().unwrap());

    let res = app
        .post(
            "/api/auth/change-password",
            json!({ "currentPassword": "SaiMatKhau1", "newPassword": "MatKhauMoi456" }),
            Some(&token),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res.body["error"]["fields"]["currentPassword"].is_array());

    let res = app
        .post(
            "/api/auth/change-password",
            json!({ "currentPassword": PASSWORD, "newPassword": "MatKhauMoi456" }),
            Some(&token),
        )
        .await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(app.refresh(&other_cookie).await.status, StatusCode::UNAUTHORIZED, "thiết bị khác bị đăng xuất");
    assert_eq!(app.refresh(&cookie).await.status, StatusCode::OK, "thiết bị đang dùng vẫn giữ phiên");
    assert_eq!(app.login("hocsinh1", "MatKhauMoi456").await.status, StatusCode::OK);
}
