//! M0 — health check, request id, lỗi 404 dạng JSON.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use sqlx::PgPool;

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn health_tra_ve_ok_va_request_id(pool: PgPool) {
    let app = TestApp::new(pool);
    let res = app.get("/api/health", None).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["status"], "ok");
    assert_eq!(res.body["checks"]["database"], true);
    assert!(res.headers.contains_key("x-request-id"));
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn api_khong_ton_tai_tra_404_json(pool: PgPool) {
    let app = TestApp::new(pool);
    let res = app.get("/api/khong-co", None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.code(), "NOT_FOUND");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn json_hong_tra_400(pool: PgPool) {
    let app = TestApp::new(pool);
    let res = app.send("POST", "/api/auth/login", Some(serde_json::json!({ "login": 1 })), None, None).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(res.code(), "INVALID_JSON");
}
