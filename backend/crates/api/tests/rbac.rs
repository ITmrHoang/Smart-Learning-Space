//! M2 — phân quyền theo vai trò.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;

const ADMIN_ROUTES: &[&str] = &[
    "/api/admin/users",
    "/api/admin/audit-logs",
    "/api/admin/subjects",
    "/api/admin/grades",
    "/api/admin/master-data/pending",
    "/api/admin/teacher-applications",
];

// 2
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn khu_quan_tri_can_dang_nhap(pool: PgPool) {
    let app = TestApp::new(pool);
    for path in ADMIN_ROUTES {
        let res = app.get(path, None).await;
        assert_eq!((res.status, res.code()), (StatusCode::UNAUTHORIZED, "UNAUTHENTICATED"), "{path}");
    }
}

// 3, 4
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn chi_admin_vao_duoc_khu_quan_tri(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, student) = app.login_as("hocsinh1", UserRole::Student).await;
    let (_, teacher) = app.login_as("giaovien1", UserRole::Teacher).await;
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;

    for path in ADMIN_ROUTES {
        for token in [&student, &teacher] {
            let res = app.get(path, Some(token)).await;
            assert_eq!((res.status, res.code()), (StatusCode::FORBIDDEN, "FORBIDDEN"), "{path}");
        }
        assert_eq!(app.get(path, Some(&admin)).await.status, StatusCode::OK, "{path}");
    }
    // Thao tác ghi cũng bị chặn.
    let res = app.post("/api/admin/subjects", json!({ "name": "Âm nhạc" }), Some(&teacher)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}

// 5
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn me_tra_dung_quyen_theo_vai_tro(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, student) = app.login_as("hocsinh1", UserRole::Student).await;
    let (_, teacher) = app.login_as("giaovien1", UserRole::Teacher).await;
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;

    let perms = |token: String| {
        let app = &app;
        async move { app.get("/api/auth/me", Some(&token)).await.body["permissions"].clone() }
    };
    assert_eq!(perms(student).await, json!(["teacher_applications.submit"]));
    assert_eq!(perms(teacher).await, json!(["master_data.propose", "teaching.manage"]));
    let admin_perms = perms(admin).await;
    for p in ["admin.access", "users.manage", "master_data.manage", "teacher_applications.review"] {
        assert!(admin_perms.as_array().unwrap().contains(&json!(p)), "admin thiếu {p}");
    }
}
