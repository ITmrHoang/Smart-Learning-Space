//! M3 — admin quản lý user.

mod common;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp};
use serde_json::json;
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;

// 1
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn danh_sach_phan_trang_loc_tim_kiem(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    for name in ["hocsinh.an", "hocsinh.binh", "hocsinh.chi"] {
        app.create_user(name, UserRole::Student).await;
    }
    app.create_user("giaovien.dung", UserRole::Teacher).await;

    let res = app.get("/api/admin/users?role=student&pageSize=2", Some(&admin)).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["total"], 3);
    assert_eq!(res.body["items"].as_array().unwrap().len(), 2);
    assert_eq!(res.body["pageSize"], 2);

    let res = app.get("/api/admin/users?role=student&pageSize=2&page=2", Some(&admin)).await;
    assert_eq!(res.body["items"].as_array().unwrap().len(), 1);

    let res = app.get("/api/admin/users?q=BINH", Some(&admin)).await;
    assert_eq!(res.body["total"], 1);
    assert_eq!(res.body["items"][0]["username"], "hocsinh.binh");

    // Ký tự đặc biệt của LIKE không làm lọc sai.
    let res = app.get("/api/admin/users?q=%25", Some(&admin)).await;
    assert_eq!(res.body["total"], 0);

    let res = app.get("/api/admin/users?role=sai", Some(&admin)).await;
    assert_eq!((res.status, res.code()), (StatusCode::BAD_REQUEST, "INVALID_QUERY"));
}

// 2
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn admin_tao_user(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let body = json!({ "username": "giaovien1", "email": "gv1@example.com", "fullName": "Trần Thị B", "password": PASSWORD, "role": "teacher" });

    let res = app.post("/api/admin/users", body.clone(), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.body);
    assert_eq!(res.body["status"], "active");
    assert_eq!(app.login("giaovien1", PASSWORD).await.body["user"]["role"], "teacher");

    let res = app.post("/api/admin/users", body, Some(&admin)).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "USERNAME_TAKEN"));
}

// 3, 8
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn doi_vai_tro_va_ghi_nhat_ky(pool: PgPool) {
    let app = TestApp::new(pool);
    let (admin_id, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let student_id = app.create_user("hocsinh1", UserRole::Student).await;
    let cookie = app.login("hocsinh1", PASSWORD).await.refresh_cookie().unwrap();

    let res = app
        .patch(
            &format!("/api/admin/users/{student_id}"),
            json!({ "role": "teacher", "fullName": "Tên Mới" }),
            Some(&admin),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
    assert_eq!(res.body["role"], "teacher");
    assert_eq!(res.body["fullName"], "Tên Mới");
    assert_eq!(app.refresh(&cookie).await.status, StatusCode::UNAUTHORIZED, "đổi vai trò thì đăng nhập lại");

    let logs = app.get("/api/admin/audit-logs?action=user.update", Some(&admin)).await;
    assert_eq!(logs.body["total"], 1);
    let entry = &logs.body["items"][0];
    assert_eq!(entry["actorId"], admin_id.to_string());
    assert_eq!(entry["actorUsername"], "quantri1");
    assert_eq!(entry["targetId"], student_id.to_string());
    assert_eq!(entry["data"]["role"]["to"], "teacher");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn email_trung_khi_sua(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let id = app.create_user("hocsinh1", UserRole::Student).await;
    app.create_user("hocsinh2", UserRole::Student).await;
    let res =
        app.patch(&format!("/api/admin/users/{id}"), json!({ "email": "HocSinh2@example.com" }), Some(&admin)).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "EMAIL_TAKEN"));
}

// 4
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn admin_khong_tu_khoa_hay_tu_ha_vai_tro(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let (admin_id, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let res = app.patch(&format!("/api/admin/users/{admin_id}"), json!({ "role": "student" }), Some(&admin)).await;
    assert_eq!(res.code(), "CANNOT_MODIFY_SELF");
    let res = app.post(&format!("/api/admin/users/{admin_id}/lock"), json!({}), Some(&admin)).await;
    assert_eq!(res.code(), "CANNOT_MODIFY_SELF");

    // Admin thứ hai (đã bị khóa nhưng token còn hạn) không thể hạ admin cuối cùng đang hoạt động.
    let (other_id, other) = app.login_as("quantri2", UserRole::Admin).await;
    sqlx::query("UPDATE users SET status = 'locked' WHERE id = $1").bind(other_id).execute(&pool).await.unwrap();
    let res = app.patch(&format!("/api/admin/users/{admin_id}"), json!({ "role": "teacher" }), Some(&other)).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "LAST_ADMIN"));
}

// 5, 6
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn khoa_va_mo_khoa(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let id = app.create_user("hocsinh1", UserRole::Student).await;
    let cookie = app.login("hocsinh1", PASSWORD).await.refresh_cookie().unwrap();

    let res = app.post(&format!("/api/admin/users/{id}/lock"), json!({}), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["status"], "locked");
    assert_eq!(app.refresh(&cookie).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(app.login("hocsinh1", PASSWORD).await.code(), "ACCOUNT_LOCKED");

    let res = app.post(&format!("/api/admin/users/{id}/unlock"), json!({}), Some(&admin)).await;
    assert_eq!(res.body["status"], "active");
    assert_eq!(app.login("hocsinh1", PASSWORD).await.status, StatusCode::OK);

    let logs = app.get("/api/admin/audit-logs", Some(&admin)).await;
    let actions: Vec<_> = logs.body["items"].as_array().unwrap().iter().map(|e| e["action"].clone()).collect();
    assert_eq!(actions, vec![json!("user.unlock"), json!("user.lock")]);
}

// 7
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn gui_email_dat_lai_mat_khau(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let id = app.create_user("hocsinh1", UserRole::Student).await;
    let res = app.post(&format!("/api/admin/users/{id}/send-password-reset"), json!({}), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::ACCEPTED);
    let emails = app.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].to, "hocsinh1@example.com");

    let res = app.get(&format!("/api/admin/users/{}", uuid::Uuid::now_v7()), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = app.get("/api/admin/users/khong-phai-uuid", Some(&admin)).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}
