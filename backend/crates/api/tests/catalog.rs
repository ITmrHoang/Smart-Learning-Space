//! M4 — master data: môn học, lớp, danh mục.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::{Value, json};
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;

fn names(list: &Value) -> Vec<String> {
    list.as_array().unwrap().iter().map(|g| g["name"].as_str().unwrap().to_string()).collect()
}

// 1
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn du_lieu_ban_dau(pool: PgPool) {
    let app = TestApp::new(pool);
    let res = app.get("/api/master-data", None).await;
    assert_eq!(res.status, StatusCode::OK);
    let grades = names(&res.body["grades"]);
    let expected: Vec<String> = (1..=12).map(|n| format!("Lớp {n}")).collect();
    assert_eq!(grades, expected);
    assert!(res.body["grades"][0]["isSystem"].as_bool().unwrap());
    assert!(names(&res.body["subjects"]).contains(&"Toán".to_string()));
    assert_eq!(res.body["levels"].as_array().unwrap().len(), 4);
    assert_eq!(res.body["levels"][3], json!({ "code": "van_dung_cao", "name": "Vận dụng cao" }));
}

// 2
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn khong_sua_lop_he_thong(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let grades = app.get("/api/admin/grades", Some(&admin)).await;
    let lop1 = grades.body[0]["id"].as_str().unwrap().to_string();
    let res = app.patch(&format!("/api/admin/grades/{lop1}"), json!({ "name": "Lớp Một" }), Some(&admin)).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "SYSTEM_ITEM"));
}

// 3, 4
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn admin_quan_ly_mon_hoc(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let (_, teacher) = app.login_as("giaovien1", UserRole::Teacher).await;

    let res = app.post("/api/admin/subjects", json!({ "name": "  toán " }), Some(&admin)).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "DUPLICATE_NAME"));

    let res = app.post("/api/admin/subjects", json!({ "name": "Âm nhạc", "sortOrder": 20 }), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["slug"], "am-nhac");
    let id = res.body["id"].as_str().unwrap().to_string();

    // Ẩn môn → không còn trong master data công khai, admin vẫn thấy.
    let res = app.patch(&format!("/api/admin/subjects/{id}"), json!({ "isActive": false }), Some(&admin)).await;
    assert_eq!(res.body["isActive"], false);
    assert!(!names(&app.get("/api/master-data", None).await.body["subjects"]).contains(&"Âm nhạc".to_string()));
    assert!(names(&app.get("/api/admin/subjects", Some(&admin)).await.body).contains(&"Âm nhạc".to_string()));

    let res = app.post("/api/admin/subjects", json!({ "name": "Mỹ thuật" }), Some(&teacher)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}

// 5, 6
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn lop_rieng_cua_giao_vien_va_dua_len_chung(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let (teacher_a_id, teacher_a) = app.login_as("giaovien.a", UserRole::Teacher).await;
    let (_, teacher_b) = app.login_as("giaovien.b", UserRole::Teacher).await;
    let (_, student) = app.login_as("hocsinh1", UserRole::Student).await;

    let res = app.post("/api/grades", json!({ "name": "Ôn thi IELTS" }), Some(&teacher_a)).await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["isShared"], false);
    assert_eq!(res.body["ownerId"], teacher_a_id.to_string());
    let id = res.body["id"].as_str().unwrap().to_string();

    // Thêm lại cùng tên (khác dấu, hoa thường) → nhận lại mục cũ, không tạo trùng.
    let res = app.post("/api/grades", json!({ "name": "on thi ielts" }), Some(&teacher_a)).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["id"], id);

    let sees = |token: String| {
        let app = &app;
        async move {
            names(&app.get("/api/master-data", Some(&token)).await.body["grades"]).contains(&"Ôn thi IELTS".to_string())
        }
    };
    assert!(sees(teacher_a.clone()).await);
    assert!(!sees(teacher_b.clone()).await, "giáo viên khác không thấy lớp riêng");

    let pending = app.get("/api/admin/master-data/pending", Some(&admin)).await;
    assert_eq!(pending.body[0]["kind"], "grade");
    assert_eq!(pending.body[0]["ownerUsername"], "giaovien.a");

    let res = app.post(&format!("/api/admin/grades/{id}/promote"), json!({}), Some(&admin)).await;
    assert_eq!(res.body["isShared"], true);
    assert!(sees(teacher_b.clone()).await, "sau khi đưa lên chung thì ai cũng thấy");
    assert!(app.get("/api/admin/master-data/pending", Some(&admin)).await.body.as_array().unwrap().is_empty());

    // Giáo viên B thêm tên trùng → nhận lại mục chung.
    let res = app.post("/api/grades", json!({ "name": "Ôn thi IELTS" }), Some(&teacher_b)).await;
    assert_eq!((res.status, res.body["id"].clone()), (StatusCode::OK, json!(id)));

    // Học sinh không được thêm.
    let res = app.post("/api/grades", json!({ "name": "Lớp của em" }), Some(&student)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}

// 7
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn gop_lop_rieng_vao_lop_chung(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let (_, teacher) = app.login_as("giaovien1", UserRole::Teacher).await;

    let shared = app.post("/api/grades", json!({ "name": "Luyện thi IELTS" }), Some(&admin)).await;
    assert_eq!(shared.status, StatusCode::CREATED);
    assert_eq!(shared.body["isShared"], true);
    let shared_id = shared.body["id"].as_str().unwrap().to_string();
    let own = app.post("/api/grades", json!({ "name": "IELTS 6.5" }), Some(&teacher)).await;
    let own_id = own.body["id"].as_str().unwrap().to_string();

    let res = app.post(&format!("/api/admin/grades/{own_id}/merge"), json!({ "intoId": own_id }), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res =
        app.post(&format!("/api/admin/grades/{own_id}/merge"), json!({ "intoId": shared_id }), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);

    let grades = names(&app.get("/api/master-data", Some(&teacher)).await.body["grades"]);
    assert!(!grades.contains(&"IELTS 6.5".to_string()));
    assert!(grades.contains(&"Luyện thi IELTS".to_string()));
    assert_eq!(app.get("/api/admin/audit-logs?action=grade.merge", Some(&admin)).await.body["total"], 1);
}

// 8
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn danh_muc_thuoc_tung_mon(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, admin) = app.login_as("quantri1", UserRole::Admin).await;
    let (_, teacher) = app.login_as("giaovien1", UserRole::Teacher).await;
    let toan = app.subject_id("toan").await;
    let ly = app.subject_id("vat-ly").await;

    let res = app.post(&format!("/api/subjects/{toan}/tags"), json!({ "name": "Đại số" }), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::CREATED);
    assert_eq!(res.body["subjectId"], toan.to_string());
    let dai_so = res.body["id"].as_str().unwrap().to_string();

    let res = app.post(&format!("/api/subjects/{toan}/tags"), json!({ "name": "đại số" }), Some(&admin)).await;
    assert_eq!((res.status, res.code()), (StatusCode::CONFLICT, "DUPLICATE_NAME"));
    let res = app.post(&format!("/api/subjects/{ly}/tags"), json!({ "name": "Đại số" }), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::CREATED, "môn khác được trùng tên");

    let res =
        app.post(&format!("/api/subjects/{toan}/tags"), json!({ "name": "Hình học không gian" }), Some(&teacher)).await;
    assert_eq!(res.body["isShared"], false);
    let own_tag = res.body["id"].as_str().unwrap().to_string();

    let tags = names(&app.get(&format!("/api/subjects/{toan}/tags"), Some(&teacher)).await.body);
    assert_eq!(tags, vec!["Đại số", "Hình học không gian"]);
    let public_tags = names(&app.get(&format!("/api/subjects/{toan}/tags"), None).await.body);
    assert_eq!(public_tags, vec!["Đại số"]);

    // Gộp danh mục sang môn khác → không cho phép.
    let ly_tag = app.get(&format!("/api/subjects/{ly}/tags"), None).await.body[0]["id"].as_str().unwrap().to_string();
    let res = app.post(&format!("/api/admin/tags/{own_tag}/merge"), json!({ "intoId": ly_tag }), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = app.post(&format!("/api/admin/tags/{own_tag}/merge"), json!({ "intoId": dai_so }), Some(&admin)).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    let res = app.get(&format!("/api/subjects/{}/tags", uuid::Uuid::now_v7()), None).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}
