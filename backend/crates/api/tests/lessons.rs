//! M7: thư viện bài học.

mod common;

use axum::http::StatusCode;
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;
use uuid::Uuid;

fn lesson(subject: Uuid, title: &str) -> Value {
    json!({
        "subjectId": subject,
        "title": title,
        "summary": "Tóm tắt",
        "content": "# Căn bậc hai\n\n$\\sqrt{16} = 4$",
        "videoUrl": "https://www.youtube.com/watch?v=abc",
        "durationMinutes": 15,
        "tagIds": []
    })
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tao_sua_xuat_ban_bai_hoc(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, token, toan) = teacher(&app, "gv_toan", "toan").await;

    let res = app.post("/api/teaching/lessons", lesson(toan, "Căn bậc hai"), Some(&token)).await;
    ok(&res, StatusCode::CREATED);
    let id = res.body["id"].as_str().unwrap().to_string();
    assert_eq!(res.body["publishedVersion"], Value::Null);
    assert_eq!(res.body["hasChanges"], true);

    // Xuất bản lần 1.
    let res = app.post(&format!("/api/teaching/lessons/{id}/publish"), json!({}), Some(&token)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["publishedVersion"], 1);
    assert_eq!(res.body["hasChanges"], false);
    // Không có thay đổi → không xuất bản lại.
    let res = app.post(&format!("/api/teaching/lessons/{id}/publish"), json!({}), Some(&token)).await;
    assert_eq!(res.code(), "NOTHING_TO_PUBLISH");

    // Sửa → có thay đổi chưa xuất bản; xuất bản lần 2.
    let mut body = lesson(toan, "Căn bậc hai (sửa)");
    body["content"] = json!("Nội dung mới");
    let res = app.put(&format!("/api/teaching/lessons/{id}"), body, Some(&token)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["hasChanges"], true);
    assert_eq!(res.body["publishedVersion"], 1);
    ok(&app.post(&format!("/api/teaching/lessons/{id}/publish"), json!({}), Some(&token)).await, StatusCode::OK);
    let revs = app.get(&format!("/api/teaching/lessons/{id}/revisions"), Some(&token)).await;
    assert_eq!(revs.body.as_array().unwrap().len(), 2);
    assert_eq!(revs.body[0]["version"], 2);
    assert_eq!(revs.body[0]["title"], "Căn bậc hai (sửa)");

    // Danh sách + lọc trạng thái + tìm không dấu.
    app.post("/api/teaching/lessons", lesson(toan, "Hàm số bậc nhất"), Some(&token)).await;
    let list = app.get("/api/teaching/lessons?q=ham%20so", Some(&token)).await;
    assert_eq!(list.body["total"], 1);
    let drafts = app.get("/api/teaching/lessons?status=draft", Some(&token)).await;
    assert_eq!(drafts.body["total"], 1);
    assert_eq!(drafts.body["items"][0]["title"], "Hàm số bậc nhất");

    // Ẩn bài.
    ok(&app.delete(&format!("/api/teaching/lessons/{id}"), Some(&token)).await, StatusCode::NO_CONTENT);
    assert_eq!(app.get(&format!("/api/teaching/lessons/{id}"), Some(&token)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get("/api/teaching/lessons", Some(&token)).await.body["total"], 1);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn quyen_va_kiem_tra_du_lieu(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, token, toan) = teacher(&app, "gv_toan", "toan").await;
    let van = app.subject_id("ngu-van").await;

    // Học sinh không vào được khu soạn bài.
    let (_, student) = app.login_as("hs", UserRole::Student).await;
    assert_eq!(app.get("/api/teaching/lessons", Some(&student)).await.status, StatusCode::FORBIDDEN);

    // Môn chưa được duyệt dạy.
    let res = app.post("/api/teaching/lessons", lesson(van, "Văn"), Some(&token)).await;
    assert_eq!(res.code(), "SUBJECT_NOT_ALLOWED");

    // Link video phải là http(s); tên bài bắt buộc.
    let mut bad = lesson(toan, "");
    bad["videoUrl"] = json!("javascript:alert(1)");
    let res = app.post("/api/teaching/lessons", bad, Some(&token)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res.body["error"]["fields"]["title"].is_array());
    assert!(res.body["error"]["fields"]["videoUrl"].is_array());

    // Danh mục phải thuộc môn của bài.
    let tag_van = app.post(&format!("/api/subjects/{van}/tags"), json!({ "name": "Nghị luận" }), Some(&token)).await;
    let mut body = lesson(toan, "Bài");
    body["tagIds"] = json!([tag_van.body["id"]]);
    let res = app.post("/api/teaching/lessons", body, Some(&token)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", res.body);

    // Giáo viên khác không xem / sửa được bài.
    let id = app.post("/api/teaching/lessons", lesson(toan, "Bài riêng"), Some(&token)).await.body["id"].clone();
    let (_, other, _) = teacher(&app, "gv_khac", "toan").await;
    let path = format!("/api/teaching/lessons/{}", id.as_str().unwrap());
    assert_eq!(app.get(&path, Some(&other)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.put(&path, lesson(toan, "Chiếm"), Some(&other)).await.status, StatusCode::NOT_FOUND);
    // Admin cũng có quyền soạn (mọi môn).
    let (_, admin) = app.login_as("quantri", UserRole::Admin).await;
    ok(&app.post("/api/teaching/lessons", lesson(van, "Bài của admin"), Some(&admin)).await, StatusCode::CREATED);
}
