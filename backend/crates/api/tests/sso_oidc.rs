//! M6.2: luồng OIDC thật (start → nhà cung cấp → callback) với nhà cung cấp giả `common::oidc`.

mod common;

use axum::http::StatusCode;
use common::{
    PASSWORD, TestApp,
    oidc::{AuthorizeRequest, CLIENT_ID, FakeOidc, MS_CONSUMER_TENANT},
};
use serde_json::{Value, json};
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;

const APP: &str = "http://127.0.0.1:5173";

/// `POST /start` → `(authorizeUrl, giá trị cookie state)`.
async fn start(app: &TestApp, provider: &str, body: Value, token: Option<&str>) -> (String, String) {
    let res = app.post(&format!("/api/auth/sso/{provider}/start"), body, token).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
    let url = res.body["authorizeUrl"].as_str().unwrap().to_string();
    let state = res.cookie("sls_sso_state").expect("có cookie state");
    (url, state)
}

async fn callback(app: &TestApp, provider: &str, query: &str, cookie_state: Option<&str>) -> common::Res {
    let cookie = cookie_state.map(|s| format!("sls_sso_state={s}"));
    app.request("GET", &format!("/api/auth/sso/{provider}/callback?{query}"), None, None, cookie.as_deref()).await
}

/// Cả luồng: start → người dùng đăng nhập ở nhà cung cấp (claims) → callback.
async fn sso(app: &TestApp, fake: &FakeOidc, provider: &str, claims: Value) -> common::Res {
    sso_with(app, fake, provider, json!({}), None, claims).await
}

async fn sso_with(
    app: &TestApp,
    fake: &FakeOidc,
    provider: &str,
    body: Value,
    token: Option<&str>,
    claims: Value,
) -> common::Res {
    let (url, cookie) = start(app, provider, body, token).await;
    let (code, state) = fake.authorize(&url, claims);
    let res = callback(app, provider, &format!("code={code}&state={state}"), Some(&cookie)).await;
    assert_eq!(res.status, StatusCode::SEE_OTHER, "{:?}", res.body);
    res
}

/// `…/dang-nhap?ssoError=CODE` → `CODE`.
fn sso_error(res: &common::Res) -> String {
    res.location().split("ssoError=").nth(1).unwrap_or("").to_string()
}

/// Đọc tham số trong phần `#…` của địa chỉ chuyển hướng.
fn fragment(res: &common::Res) -> AuthorizeRequest {
    let loc = res.location();
    AuthorizeRequest::parse(&format!("x?{}", loc.split_once('#').expect("có #").1))
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn danh_sach_nha_cung_cap(pool: PgPool) {
    let plain = TestApp::new(pool.clone());
    assert_eq!(plain.get("/api/auth/sso/providers", None).await.body, json!({ "providers": [] }));
    let res = plain.post("/api/auth/sso/google/start", json!({}), None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    let (app, _fake) = TestApp::with_sso(pool).await;
    assert_eq!(app.get("/api/auth/sso/providers", None).await.body, json!({ "providers": ["google", "microsoft"] }));
    assert_eq!(app.post("/api/auth/sso/facebook/start", json!({}), None).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn url_dang_nhap_co_pkce_state_nonce_va_cookie_an_toan(pool: PgPool) {
    let (app, fake) = TestApp::with_sso(pool).await;
    let res = app.post("/api/auth/sso/google/start", json!({ "next": "/lop-hoc" }), None).await;
    let cookie = res.set_cookie("sls_sso_state").unwrap();
    for attr in ["HttpOnly", "SameSite=Lax", "Path=/api/auth/sso", "Max-Age=600"] {
        assert!(cookie.contains(attr), "{cookie}");
    }
    let url = res.body["authorizeUrl"].as_str().unwrap();
    assert!(url.starts_with(&format!("{}/authorize?", fake.base)));
    let req = AuthorizeRequest::parse(url);
    assert_eq!(req.get("client_id"), CLIENT_ID);
    assert_eq!(req.get("response_type"), "code");
    assert_eq!(req.get("scope"), "openid email profile");
    assert_eq!(req.get("code_challenge_method"), "S256");
    assert_eq!(req.get("redirect_uri"), format!("{APP}/api/auth/sso/google/callback"));
    assert_eq!(req.get("state"), res.cookie("sls_sso_state").unwrap());
    assert!(req.get("nonce").len() >= 32 && req.get("code_challenge").len() == 43);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tao_tai_khoan_moi_va_dang_nhap(pool: PgPool) {
    let (app, fake) = TestApp::with_sso(pool).await;
    let claims = fake.google_claims("g-001", "Hoc.Sinh@Gmail.com");
    let res = sso_with(&app, &fake, "google", json!({ "next": "/lop-hoc" }), None, claims).await;
    assert_eq!(res.location(), format!("{APP}/sso/xong?next=%2Flop-hoc&new=1"));
    // Cookie state bị xóa, cookie refresh được đặt.
    assert_eq!(res.cookie("sls_sso_state").as_deref(), Some(""));
    let refresh = res.refresh_cookie().unwrap();
    assert!(!refresh.is_empty());

    let session = app.refresh(&refresh).await;
    assert_eq!(session.status, StatusCode::OK);
    let me = &session.body["user"];
    assert_eq!(me["email"], "hoc.sinh@gmail.com");
    assert_eq!(me["username"], "hoc.sinh");
    assert_eq!(me["role"], "student");
    assert_eq!(me["hasPassword"], false);

    // Lần sau: đăng nhập luôn, không tạo tài khoản mới; `next` lạ bị bỏ.
    let claims = fake.google_claims("g-001", "hoc.sinh@gmail.com");
    let res = sso_with(&app, &fake, "google", json!({ "next": "//evil.com" }), None, claims).await;
    assert_eq!(res.location(), format!("{APP}/sso/xong"));
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn state_phai_khop_cookie_va_chi_dung_mot_lan(pool: PgPool) {
    let (app, fake) = TestApp::with_sso(pool).await;

    // Không có cookie (link đăng nhập bị gửi sang trình duyệt khác).
    let (url, _) = start(&app, "google", json!({}), None).await;
    let (code, state) = fake.authorize(&url, fake.google_claims("g-1", "a@gmail.com"));
    let res = callback(&app, "google", &format!("code={code}&state={state}"), None).await;
    assert_eq!(res.status, StatusCode::SEE_OTHER);
    assert_eq!(res.location(), format!("{APP}/dang-nhap?ssoError=SSO_STATE_INVALID"));

    // Cookie của lượt khác.
    let (url, _) = start(&app, "google", json!({}), None).await;
    let (_, other_cookie) = start(&app, "google", json!({}), None).await;
    let (code, state) = fake.authorize(&url, fake.google_claims("g-1", "a@gmail.com"));
    let res = callback(&app, "google", &format!("code={code}&state={state}"), Some(&other_cookie)).await;
    assert_eq!(sso_error(&res), "SSO_STATE_INVALID");

    // Đúng → đăng nhập; gọi lại cùng state → bị từ chối.
    let (url, cookie) = start(&app, "google", json!({}), None).await;
    let (code, state) = fake.authorize(&url, fake.google_claims("g-1", "a@gmail.com"));
    let query = format!("code={code}&state={state}");
    assert!(callback(&app, "google", &query, Some(&cookie)).await.location().contains("/sso/xong"));
    assert_eq!(sso_error(&callback(&app, "google", &query, Some(&cookie)).await), "SSO_STATE_INVALID");

    // State của Google không dùng được cho callback Microsoft.
    let (url, cookie) = start(&app, "google", json!({}), None).await;
    let (code, state) = fake.authorize(&url, fake.google_claims("g-1", "a@gmail.com"));
    let res = callback(&app, "microsoft", &format!("code={code}&state={state}"), Some(&cookie)).await;
    assert_eq!(sso_error(&res), "SSO_STATE_INVALID");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn nguoi_dung_huy_o_trang_nha_cung_cap(pool: PgPool) {
    let (app, _fake) = TestApp::with_sso(pool).await;
    let (url, cookie) = start(&app, "google", json!({}), None).await;
    let state = AuthorizeRequest::parse(&url).get("state").to_string();
    let res = callback(&app, "google", &format!("error=access_denied&state={state}"), Some(&cookie)).await;
    assert_eq!(res.location(), format!("{APP}/dang-nhap?ssoError=SSO_CANCELLED"));
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn id_token_khong_hop_le_bi_tu_choi(pool: PgPool) {
    let (app, fake) = TestApp::with_sso(pool).await;
    let base = || fake.google_claims("g-bad", "bad@gmail.com");
    let now = chrono::Utc::now().timestamp();

    let cases: Vec<(&str, Value)> = vec![
        ("nonce sai", {
            let mut c = base();
            c["nonce"] = json!("nonce-khac");
            c
        }),
        ("aud sai", {
            let mut c = base();
            c["aud"] = json!("ung-dung-khac");
            c
        }),
        ("iss sai", {
            let mut c = base();
            c["iss"] = json!("https://evil.example");
            c
        }),
        ("hết hạn", {
            let mut c = base();
            c["exp"] = json!(now - 3600);
            c
        }),
    ];
    for (name, claims) in cases {
        let res = sso(&app, &fake, "google", claims).await;
        assert_eq!(sso_error(&res), "SSO_FAILED", "{name}");
    }

    // Chữ ký sai: sửa phần nội dung của token đã ký.
    let (url, cookie) = start(&app, "google", json!({}), None).await;
    let req = AuthorizeRequest::parse(&url);
    let mut claims = base();
    claims["nonce"] = json!(req.get("nonce"));
    let token = FakeOidc::sign(&claims);
    let parts: Vec<&str> = token.split('.').collect();
    claims["sub"] = json!("g-admin");
    let other = FakeOidc::sign(&claims);
    let forged = format!("{}.{}.{}", parts[0], other.split('.').nth(1).unwrap(), parts[2]);
    let (code, state) = fake.authorize_raw(&req, forged);
    let res = callback(&app, "google", &format!("code={code}&state={state}"), Some(&cookie)).await;
    assert_eq!(sso_error(&res), "SSO_FAILED");

    // Chưa xác minh email → theo chính sách M6.1.
    let mut claims = base();
    claims["email_verified"] = json!(false);
    assert_eq!(sso_error(&sso(&app, &fake, "google", claims).await), "SSO_EMAIL_UNVERIFIED");

    // Không có tài khoản nào được tạo.
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users").fetch_one(&app.ctx.db).await.unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn trung_email_tai_khoan_da_co_thi_xac_nhan_bang_mat_khau(pool: PgPool) {
    let (app, fake) = TestApp::with_sso(pool).await;
    app.create_user("lan", UserRole::Teacher).await;

    let res = sso(&app, &fake, "google", fake.google_claims("g-lan", "lan@example.com")).await;
    assert!(res.location().starts_with(&format!("{APP}/sso/lien-ket#")), "{}", res.location());
    assert!(res.refresh_cookie().is_none());
    let f = fragment(&res);
    assert_eq!(f.get("provider"), "google");
    assert_eq!(f.get("email"), "l***@example.com");
    assert_eq!(f.get("password"), "1");
    assert_eq!(f.get("expiresIn"), "900");

    let linked = app.post("/api/auth/sso/link", json!({ "token": f.get("token"), "password": PASSWORD }), None).await;
    assert_eq!(linked.status, StatusCode::OK, "{:?}", linked.body);
    assert_eq!(linked.body["user"]["username"], "lan");

    // Từ giờ đăng nhập Google vào thẳng tài khoản giáo viên.
    let res = sso(&app, &fake, "google", fake.google_claims("g-lan", "lan@example.com")).await;
    assert_eq!(res.location(), format!("{APP}/sso/xong"));
    let session = app.refresh(&res.refresh_cookie().unwrap()).await;
    assert_eq!(session.body["user"]["role"], "teacher");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn lien_ket_them_khi_da_dang_nhap(pool: PgPool) {
    let (app, fake) = TestApp::with_sso(pool).await;
    let (_, token) = app.login_as("minh", UserRole::Student).await;

    // Chế độ liên kết bắt buộc đăng nhập.
    let res = app.post("/api/auth/sso/google/start", json!({ "mode": "link" }), None).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);

    // Email Google khác email tài khoản vẫn liên kết được (người dùng đã đăng nhập).
    let claims = fake.google_claims("g-minh", "minh.ca.nhan@gmail.com");
    let res = sso_with(&app, &fake, "google", json!({ "mode": "link" }), Some(&token), claims).await;
    assert_eq!(res.location(), format!("{APP}/ho-so?ssoLinked=google"));
    assert!(res.refresh_cookie().is_none(), "liên kết không tạo phiên mới");

    let ids = app.get("/api/auth/identities", Some(&token)).await;
    assert_eq!(ids.body.as_array().unwrap().len(), 1);
    assert_eq!(ids.body[0]["email"], "minh.ca.nhan@gmail.com");

    // Liên kết lại đúng danh tính đó → vẫn báo thành công.
    let claims = fake.google_claims("g-minh", "minh.ca.nhan@gmail.com");
    let res = sso_with(&app, &fake, "google", json!({ "mode": "link" }), Some(&token), claims).await;
    assert_eq!(res.location(), format!("{APP}/ho-so?ssoLinked=google"));

    // Một Google khác → tài khoản đã có Google.
    let claims = fake.google_claims("g-khac", "khac@gmail.com");
    let res = sso_with(&app, &fake, "google", json!({ "mode": "link" }), Some(&token), claims).await;
    assert_eq!(res.location(), format!("{APP}/ho-so?ssoError=SSO_PROVIDER_ALREADY_LINKED"));

    // Danh tính đã thuộc tài khoản khác.
    let (_, other) = app.login_as("hoa", UserRole::Student).await;
    let claims = fake.google_claims("g-minh", "minh.ca.nhan@gmail.com");
    let res = sso_with(&app, &fake, "google", json!({ "mode": "link" }), Some(&other), claims).await;
    assert_eq!(res.location(), format!("{APP}/ho-so?ssoError=SSO_ALREADY_LINKED"));

    let logs: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE action = 'identity.link'")
        .fetch_one(&app.ctx.db)
        .await
        .unwrap();
    assert_eq!(logs, 1);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn microsoft_xac_minh_email_theo_tenant(pool: PgPool) {
    let (app, fake) = TestApp::with_sso(pool).await;

    // Tài khoản Microsoft cá nhân: email do Microsoft xác minh.
    let claims = fake.microsoft_claims("ms-1", "ban@outlook.com", MS_CONSUMER_TENANT);
    let res = sso(&app, &fake, "microsoft", claims).await;
    assert_eq!(res.location(), format!("{APP}/sso/xong?new=1"));

    // Tài khoản trường / công ty: email do quản trị tenant tự đặt → không tin, trừ khi có `xms_edov`.
    let tenant = "11111111-2222-3333-4444-555555555555";
    let claims = fake.microsoft_claims("ms-2", "gv@truong.edu.vn", tenant);
    assert_eq!(sso_error(&sso(&app, &fake, "microsoft", claims).await), "SSO_EMAIL_UNVERIFIED");
    let mut claims = fake.microsoft_claims("ms-2", "gv@truong.edu.vn", tenant);
    claims["xms_edov"] = json!(true);
    assert_eq!(sso(&app, &fake, "microsoft", claims).await.location(), format!("{APP}/sso/xong?new=1"));

    // `iss` phải đúng tenant trong `tid`.
    let mut claims = fake.microsoft_claims("ms-3", "x@outlook.com", MS_CONSUMER_TENANT);
    claims["iss"] = json!(format!("{}/{tenant}/v2.0", fake.base));
    assert_eq!(sso_error(&sso(&app, &fake, "microsoft", claims).await), "SSO_FAILED");
}
