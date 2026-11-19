//! Đợt 8 (M34, M37): giáo án theo mẫu — tạo, sửa, sao chép, quyền, khối câu hỏi, tài liệu đính kèm.

mod common;

use axum::http::StatusCode;
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const PDF: &[u8] = b"%PDF-1.7 phieu hoc tap";

async fn question(app: &TestApp, t: &str, subject: Uuid) -> String {
    let body = json!({
        "subjectId": subject, "kind": "single", "level": "nhan_biet", "content": "$1 + 1 = ?$",
        "options": [{ "content": "2", "correct": true }, { "content": "3", "correct": false }]
    });
    let res = app.post("/api/teaching/questions", body, Some(t)).await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().or(res.body["question"]["id"].as_str()).unwrap().to_string()
}

/// Upload tài liệu đính kèm lên kho (bộ nhớ) như trình duyệt.
async fn attachment(app: &TestApp, t: &str) -> String {
    let res = app
        .post(
            "/api/files",
            json!({ "purpose": "attachment", "name": "Phiếu học tập.pdf", "contentType": "application/pdf", "size": PDF.len() }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let id = res.body["id"].as_str().unwrap().to_string();
    let key: String = sqlx::query_scalar("SELECT object_key FROM files WHERE id = $1::uuid")
        .bind(&id)
        .fetch_one(&app.ctx.db)
        .await
        .unwrap();
    app.ctx.storage.memory_put(&key, PDF, "application/pdf");
    ok(&app.post(&format!("/api/files/{id}/complete"), json!({}), Some(t)).await, StatusCode::OK);
    id
}

fn plan(subject: Uuid, blocks: Value) -> Value {
    json!({
        "subjectId": subject, "title": "Bài 3. Phương trình bậc hai một ẩn", "template": "cv5512",
        "info": { "school": "THCS Nguyễn Du", "teacherName": "Nguyễn Thị Lan", "periods": "2 tiết" },
        "blocks": blocks
    })
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn giao_an_tao_sua_sao_chep_quyen(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (_, other, _) = teacher(&app, "gv2", "toan").await;
    let q_mine = question(&app, &t, toan).await;
    let q_other = question(&app, &other, toan).await;

    let blocks = json!([
        { "id": "b1", "kind": "section", "title": "I. Mục tiêu", "body": "**1. Kiến thức:** nhận biết $ax^2 + bx + c = 0$." },
        { "id": "b2", "kind": "activity", "title": "Hoạt động 1: Mở đầu", "minutes": 5,
          "goal": "Tạo hứng thú", "steps": { "assign": "GV chiếu bài toán", "conclude": "GV chốt" } },
        { "id": "b3", "kind": "questions", "title": "Phiếu học tập số 1", "questionIds": [q_mine], "showAnswers": true }
    ]);
    let res = app.post("/api/teaching/lesson-plans", plan(toan, blocks.clone()), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    let id = res.body["id"].as_str().unwrap().to_string();
    assert_eq!(res.body["template"], "cv5512");
    assert_eq!(res.body["info"]["teacherName"], "Nguyễn Thị Lan");
    assert_eq!(res.body["blocks"][1]["steps"]["assign"], "GV chiếu bài toán");
    assert_eq!(res.body["blocks"][2]["questionIds"][0], q_mine.as_str());

    // Câu hỏi của người khác, mẫu lạ, quá nhiều mục → lỗi
    let bad = json!([{ "kind": "questions", "questionIds": [q_other] }]);
    let res = app.post("/api/teaching/lesson-plans", plan(toan, bad), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let mut wrong = plan(toan, json!([]));
    wrong["template"] = json!("cv9999");
    assert!(app.post("/api/teaching/lesson-plans", wrong, Some(&t)).await.status.is_client_error());
    let many: Vec<Value> = (0..81).map(|_| json!({ "kind": "section" })).collect();
    let res = app.post("/api/teaching/lesson-plans", plan(toan, json!(many)), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Giáo viên khác không thấy; học sinh không vào được trang dạy học
    assert_eq!(app.get(&format!("/api/teaching/lesson-plans/{id}"), Some(&other)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get("/api/teaching/lesson-plans", Some(&other)).await.body.as_array().unwrap().len(), 0);

    // Sửa (tự lưu) + gắn tài liệu đính kèm
    let file = attachment(&app, &t).await;
    let mut body = plan(toan, blocks);
    body["title"] = json!("Bài 3. Phương trình bậc hai (2 tiết)");
    body["attachmentIds"] = json!([file, file]);
    let res = app.put(&format!("/api/teaching/lesson-plans/{id}"), body.clone(), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["title"], "Bài 3. Phương trình bậc hai (2 tiết)");
    let files = res.body["attachments"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["name"], "Phiếu học tập.pdf");
    assert!(files[0]["url"].as_str().unwrap().contains("&sig="), "link tài liệu có chữ ký");
    // Tài liệu của người khác
    let foreign = attachment(&app, &other).await;
    body["attachmentIds"] = json!([foreign]);
    let res = app.put(&format!("/api/teaching/lesson-plans/{id}"), body, Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Tìm theo tên không dấu, sao chép, lưu trữ
    let list = app.get("/api/teaching/lesson-plans?q=phuong%20trinh", Some(&t)).await;
    ok(&list, StatusCode::OK);
    assert_eq!(list.body[0]["blockCount"], 3);
    let res = app.post(&format!("/api/teaching/lesson-plans/{id}/copy"), json!({}), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert!(res.body["title"].as_str().unwrap().ends_with("(bản sao)"));
    assert_eq!(res.body["copiedFrom"], id.as_str());
    assert_eq!(res.body["attachments"].as_array().unwrap().len(), 1);
    assert_eq!(app.get("/api/teaching/lesson-plans", Some(&t)).await.body.as_array().unwrap().len(), 2);
    assert_eq!(app.delete(&format!("/api/teaching/lesson-plans/{id}"), Some(&t)).await.status, StatusCode::NO_CONTENT);
    assert_eq!(app.get(&format!("/api/teaching/lesson-plans/{id}"), Some(&t)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get("/api/teaching/lesson-plans", Some(&t)).await.body.as_array().unwrap().len(), 1);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tai_lieu_bai_hoc_theo_ban_xuat_ban_va_bai_cua_buoi_hoc(pool: PgPool) {
    use chrono::{Duration, Utc};
    use sls_core::modules::users::model::UserRole;

    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (hs_id, hs) = app.login_as("hs_an", UserRole::Student).await;
    let (_, outsider) = app.login_as("hs_ngoai", UserRole::Student).await;

    // Bài có tài liệu đính kèm → xuất bản
    let file1 = attachment(&app, &t).await;
    let body = json!({ "subjectId": toan, "title": "Bài 3", "content": "Slide 1\n\n---\n\nSlide 2", "attachmentIds": [file1] });
    let res = app.post("/api/teaching/lessons", body.clone(), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    let lesson = res.body["id"].as_str().unwrap().to_string();
    assert_eq!(res.body["attachments"][0]["name"], "Phiếu học tập.pdf");
    let draft_only = format!("/api/teaching/lessons/{lesson}/publish");

    // Buổi học có gắn bài; học sinh của lớp được mời
    let res = app
        .post(
            "/api/teaching/classes",
            json!({ "subjectId": toan, "title": "Toán 9", "mode": "live", "visibility": "public", "enrollment": "open" }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let class_id = res.body["id"].as_str().unwrap().to_string();
    ok(&app.post(&format!("/api/classes/{class_id}/enroll"), json!({}), Some(&hs)).await, StatusCode::OK);
    let res = app
        .post(
            "/api/teaching/sessions",
            json!({
                "title": "Chữa bài", "classId": class_id, "startsAt": Utc::now() + Duration::hours(2), "durationMinutes": 60,
                "invitees": { "userIds": [hs_id] }, "access": "invited", "meetingUrl": "https://meet.example.com/abc",
                "lessonIds": [lesson]
            }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let s = res.body["sessions"][0]["id"].as_str().unwrap().to_string();
    let url = format!("/api/sessions/{s}/lessons/{lesson}");
    // Chưa xuất bản → chưa xem được
    assert_eq!(app.get(&url, Some(&hs)).await.status, StatusCode::NOT_FOUND);
    ok(&app.post(&draft_only, json!({}), Some(&t)).await, StatusCode::OK);
    let res = app.get(&url, Some(&hs)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["content"], "Slide 1\n\n---\n\nSlide 2");
    assert_eq!(res.body["attachments"].as_array().unwrap().len(), 1);
    assert!(res.body["attachments"][0]["url"].as_str().unwrap().contains("&sig="));
    // Người ngoài / bài không gắn vào buổi
    assert_eq!(app.get(&url, Some(&outsider)).await.status, StatusCode::NOT_FOUND);
    let other = app.post("/api/teaching/lessons", json!({ "subjectId": toan, "title": "Bài khác" }), Some(&t)).await;
    let other_id = other.body["id"].as_str().unwrap();
    assert_eq!(
        app.get(&format!("/api/sessions/{s}/lessons/{other_id}"), Some(&hs)).await.status,
        StatusCode::NOT_FOUND
    );

    // Thêm tài liệu vào bản nháp: học sinh vẫn thấy tài liệu của bản đã xuất bản
    let file2 = attachment(&app, &t).await;
    let mut body = body;
    body["attachmentIds"] = json!([file1, file2]);
    let res = app.put(&format!("/api/teaching/lessons/{lesson}"), body, Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["attachments"].as_array().unwrap().len(), 2);
    assert_eq!(app.get(&url, Some(&hs)).await.body["attachments"].as_array().unwrap().len(), 1);
    ok(&app.post(&draft_only, json!({}), Some(&t)).await, StatusCode::OK);
    assert_eq!(app.get(&url, Some(&hs)).await.body["attachments"].as_array().unwrap().len(), 2);
    // Ảnh không phải tài liệu đính kèm
    let res = app
        .post(
            "/api/files",
            json!({ "purpose": "attachment", "name": "a.png", "contentType": "image/png", "size": 10 }),
            Some(&t),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
}
