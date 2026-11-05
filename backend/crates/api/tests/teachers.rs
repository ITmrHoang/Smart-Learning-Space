//! M5 — đăng ký làm giáo viên và admin duyệt.

mod common;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp};
use serde_json::json;
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;

const BIO: &str = "Giáo viên Toán THCS, 5 năm luyện thi vào 10.";

// 1, 2
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn gui_ho_so(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, student) = app.login_as("hocsinh1", UserRole::Student).await;
    let toan = app.subject_id("toan").await;

    let res = app.post("/api/teacher-applications", json!({ "subjectIds": [], "bio": "ngắn" }), Some(&student)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    let res = app
        .post("/api/teacher-applications", json!({ "subjectIds": [uuid::Uuid::now_v7()], "bio": BIO }), Some(&student))
        .await;
    assert!(res.body["error"]["fields"]["subjectIds"].is_array(), "{:?}", res.body);

    let res =
        app.post("/api/teacher-applications", json!({ "subjectIds": [toan, toan], "bio": BIO }), Some(&student)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.body);
    assert_eq!(res.body["status"], "pending");
    assert_eq!(res.body["subjectIds"], json!([toan]), "bỏ môn trùng");

    let res = app.post("/api/teacher-applications", json!({ "subjectIds": [toan], "bio": BIO }), Some(&student)).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "APPLICATION_PENDING"));

    let mine = app.get("/api/teacher-applications/me", Some(&student)).await;
    assert_eq!(mine.body.as_array().unwrap().len(), 1);
}

// 3
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn admin_duyet_ho_so(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let (_, student) = app.login_as("hocsinh1", UserRole::Student).await;
    let cookie = app.login("hocsinh1", PASSWORD).await.refresh_cookie().unwrap();
    let toan = app.subject_id("toan").await;
    let app_id = app
        .post("/api/teacher-applications", json!({ "subjectIds": [toan], "bio": BIO }), Some(&student))
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();

    let list = app.get("/api/admin/teacher-applications?status=pending", Some(&admin)).await;
    assert_eq!(list.body["total"], 1);
    assert_eq!(list.body["items"][0]["applicant"]["username"], "hocsinh1");

    // Học sinh không tự duyệt được.
    let res = app.post(&format!("/api/admin/teacher-applications/{app_id}/approve"), json!({}), Some(&student)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);

    let res = app.post(&format!("/api/admin/teacher-applications/{app_id}/approve"), json!({}), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
    assert_eq!(res.body["status"], "approved");
    assert!(app.emails().iter().any(|e| e.to == "hocsinh1@example.com" && e.subject.contains("duyệt")));

    // Phiên cũ bị thu hồi; đăng nhập lại thì token mang vai trò giáo viên.
    assert_eq!(app.refresh(&cookie).await.status, StatusCode::UNAUTHORIZED);
    let res = app.login("hocsinh1", PASSWORD).await;
    assert_eq!(res.body["user"]["role"], "teacher");
    assert_eq!(res.body["user"]["permissions"], json!(["master_data.propose", "teaching.manage"]));

    let res = app.post(&format!("/api/admin/teacher-applications/{app_id}/approve"), json!({}), Some(&admin)).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "NOT_PENDING"));
    let res = app
        .post("/api/teacher-applications", json!({ "subjectIds": [toan], "bio": BIO }), Some(&res_token(&app).await))
        .await;
    assert_eq!(res.code(), "ALREADY_TEACHER");
}

async fn res_token(app: &TestApp) -> String {
    app.login("hocsinh1", PASSWORD).await.token()
}

// 4
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn admin_tu_choi_ho_so(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let (_, student) = app.login_as("hocsinh1", UserRole::Student).await;
    let toan = app.subject_id("toan").await;
    let id = app
        .post("/api/teacher-applications", json!({ "subjectIds": [toan], "bio": BIO }), Some(&student))
        .await
        .body["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res =
        app.post(&format!("/api/admin/teacher-applications/{id}/reject"), json!({ "reason": "" }), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = app
        .post(
            &format!("/api/admin/teacher-applications/{id}/reject"),
            json!({ "reason": "Thiếu bằng cấp sư phạm" }),
            Some(&admin),
        )
        .await;
    assert_eq!(res.body["status"], "rejected");
    assert_eq!(res.body["rejectReason"], "Thiếu bằng cấp sư phạm");
    assert!(app.emails().iter().any(|e| e.text.contains("Thiếu bằng cấp sư phạm")));

    // Được gửi lại hồ sơ mới.
    let res = app.post("/api/teacher-applications", json!({ "subjectIds": [toan], "bio": BIO }), Some(&student)).await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(app.get("/api/auth/me", Some(&student)).await.body["role"], "student");
}
