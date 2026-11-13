//! Đợt 5: lưu file (M21), bài tập về nhà (M22), lộ trình riêng (M24).

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sls_core::modules::{classes::homework, users::model::UserRole};
use sqlx::PgPool;
use uuid::Uuid;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-anh-bai-lam";
const PDF: &[u8] = b"%PDF-1.7 bai lam";

async fn class(app: &TestApp, t: &str, subject: Uuid) -> String {
    let res = app
        .post(
            "/api/teaching/classes",
            json!({ "subjectId": subject, "title": "Toán 9", "mode": "self_paced", "visibility": "public", "enrollment": "open" }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

async fn student(app: &TestApp, name: &str, class_id: &str) -> (Uuid, String) {
    let (id, token) = app.login_as(name, UserRole::Student).await;
    ok(&app.post(&format!("/api/classes/{class_id}/enroll"), json!({}), Some(&token)).await, StatusCode::OK);
    (id, token)
}

async fn lesson(app: &TestApp, t: &str, subject: Uuid, title: &str) -> String {
    let res = app
        .post("/api/teaching/lessons", json!({ "subjectId": subject, "title": title, "content": "Nội dung" }), Some(t))
        .await;
    ok(&res, StatusCode::CREATED);
    let id = res.body["id"].as_str().unwrap().to_string();
    ok(&app.post(&format!("/api/teaching/lessons/{id}/publish"), json!({}), Some(t)).await, StatusCode::OK);
    id
}

async fn publish_path(app: &TestApp, t: &str, class_id: &str, steps: Value) -> Vec<String> {
    let res = app.put(&format!("/api/teaching/classes/{class_id}/path"), json!({ "steps": steps }), Some(t)).await;
    ok(&res, StatusCode::OK);
    let res = app.post(&format!("/api/teaching/classes/{class_id}/path/publish"), json!({}), Some(t)).await;
    ok(&res, StatusCode::OK);
    res.body["steps"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect()
}

async fn object_key(app: &TestApp, id: &str) -> String {
    sqlx::query_scalar("SELECT object_key FROM files WHERE id = $1::uuid")
        .bind(id)
        .fetch_one(&app.ctx.db)
        .await
        .unwrap()
}

/// Xin link upload → "trình duyệt" PUT lên kho (bộ nhớ) → báo xong. Trả về FileView.
async fn upload(app: &TestApp, token: &str, purpose: &str, name: &str, ct: &str, bytes: &[u8]) -> Value {
    let res = app
        .post(
            "/api/files",
            json!({ "purpose": purpose, "name": name, "contentType": ct, "size": bytes.len() }),
            Some(token),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let id = res.body["id"].as_str().unwrap().to_string();
    assert_eq!(res.body["headers"]["Content-Type"], ct);
    app.ctx.storage.memory_put(&object_key(app, &id).await, bytes, ct);
    let res = app.post(&format!("/api/files/{id}/complete"), json!({}), Some(token)).await;
    ok(&res, StatusCode::OK);
    res.body
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn upload_file_va_link_xem(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, token) = app.login_as("hs_upload", UserRole::Student).await;
    let (_, other) = app.login_as("hs_khac", UserRole::Student).await;

    // Ảnh nội dung: link không cần đăng nhập, chuyển hướng sang presigned URL.
    let img = upload(&app, &token, "content", "Hình 1.png", "image/png", PNG).await;
    assert_eq!(img["isImage"], true);
    let url = img["url"].as_str().unwrap().to_string();
    assert_eq!(url, format!("/api/files/{}", img["id"].as_str().unwrap()));
    let res = app.get(&url, None).await;
    assert_eq!(res.status, StatusCode::FOUND);
    assert!(res.location().starts_with("memory://files/content/"));

    // Sai loại / quá lớn → từ chối ngay khi xin link.
    let bad = app
        .post(
            "/api/files",
            json!({ "purpose": "content", "name": "a.pdf", "contentType": "application/pdf", "size": 10 }),
            Some(&token),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    let big = app
        .post(
            "/api/files",
            json!({ "purpose": "content", "name": "a.png", "contentType": "image/png", "size": 6 * 1024 * 1024 }),
            Some(&token),
        )
        .await;
    assert_eq!(big.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.post("/api/files", json!({}), None).await.status, StatusCode::UNAUTHORIZED);

    // Chưa upload mà báo xong → 409; nội dung không phải PNG → xóa, báo lỗi.
    let res = app
        .post(
            "/api/files",
            json!({ "purpose": "content", "name": "x.png", "contentType": "image/png", "size": 20 }),
            Some(&token),
        )
        .await;
    let id = res.body["id"].as_str().unwrap().to_string();
    let res = app.post(&format!("/api/files/{id}/complete"), json!({}), Some(&token)).await;
    assert_eq!(res.code(), "FILE_NOT_UPLOADED");
    // Người khác không báo xong file của mình được.
    assert_eq!(
        app.post(&format!("/api/files/{id}/complete"), json!({}), Some(&other)).await.status,
        StatusCode::NOT_FOUND
    );
    app.ctx.storage.memory_put(&object_key(&app, &id).await, b"<html><script>alert(1)</script>", "image/png");
    let res = app.post(&format!("/api/files/{id}/complete"), json!({}), Some(&token)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.get(&format!("/api/files/{id}"), None).await.status, StatusCode::NOT_FOUND);

    // File bài làm: link phải có chữ ký đúng, còn hạn.
    let doc = upload(&app, &token, "homework", "Bài làm.pdf", "application/pdf", PDF).await;
    let id = doc["id"].as_str().unwrap();
    let signed = doc["url"].as_str().unwrap();
    assert!(signed.contains("?exp=") && signed.contains("&sig="));
    assert_eq!(app.get(signed, None).await.status, StatusCode::FOUND);
    assert_eq!(app.get(&format!("/api/files/{id}"), None).await.status, StatusCode::NOT_FOUND);
    let tampered = format!("{}0", &signed[..signed.len() - 1]);
    let tampered = if tampered == signed { format!("{}1", &signed[..signed.len() - 1]) } else { tampered };
    assert_eq!(app.get(&tampered, None).await.status, StatusCode::NOT_FOUND);
}

async fn notif_kinds(app: &TestApp, token: &str) -> Vec<String> {
    app.get("/api/notifications", Some(token)).await.body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["kind"].as_str().unwrap().to_string())
        .collect()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn bai_tap_nop_cham_tra_lai_va_nhac_han(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    let c = class(&app, &t, toan).await;

    // Thư viện bài tập.
    let res = app
        .post(
            "/api/teaching/assignments",
            json!({ "subjectId": toan, "title": "Bài tập về nhà số 1", "instructions": "Giải $x^2 - 4 = 0$, chụp bài làm.", "maxScore": 10 }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let a = res.body["id"].as_str().unwrap().to_string();
    assert_eq!(app.get(&format!("/api/teaching/assignments/{a}"), Some(&t2)).await.status, StatusCode::NOT_FOUND);
    let list = app.get("/api/teaching/assignments?q=s%E1%BB%91%201", Some(&t)).await;
    assert_eq!(list.body.as_array().unwrap().len(), 1);

    // Bước bài tập: phải chọn bài tập của mình.
    let res = app
        .put(&format!("/api/teaching/classes/{c}/path"), json!({ "steps": [{ "kind": "homework" }] }), Some(&t))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let due = Utc::now() + Duration::hours(3);
    let l = lesson(&app, &t, toan, "Bài 1").await;
    let steps = publish_path(
        &app,
        &t,
        &c,
        json!([
            { "kind": "homework", "assignmentId": a, "dueAt": due },
            { "kind": "lesson", "lessonId": l, "unlock": "previous" }
        ]),
    )
    .await;
    let hw = &steps[0];
    let (an_id, an) = student(&app, "hs_an", &c).await;
    let (_, binh) = student(&app, "hs_binh", &c).await;
    let (_, chi) = student(&app, "hs_chi", &c).await;

    // Học sinh xem đề; bước sau bị khóa tới khi nộp.
    let view = app.get(&format!("/api/classes/{c}/steps/{hw}/homework"), Some(&an)).await;
    ok(&view, StatusCode::OK);
    assert_eq!(view.body["title"], "Bài tập về nhà số 1");
    assert_eq!(view.body["maxScore"], 10.0);
    assert_eq!(view.body["canSubmit"], true);
    assert!(view.body["submission"].is_null());
    let learn = app.get(&format!("/api/classes/{c}/learn"), Some(&an)).await.body;
    assert_eq!(learn["steps"][1]["status"], "locked");

    // Nộp: phải có file, file phải là của mình + đã upload xong.
    let res = app.put(&format!("/api/classes/{c}/steps/{hw}/homework"), json!({ "fileIds": [] }), Some(&an)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let f1 = upload(&app, &an, "homework", "trang 1.jpg", "image/png", PNG).await;
    let f2 = upload(&app, &an, "homework", "trang 2.pdf", "application/pdf", PDF).await;
    let fb = upload(&app, &binh, "homework", "binh.png", "image/png", PNG).await;
    let res =
        app.put(&format!("/api/classes/{c}/steps/{hw}/homework"), json!({ "fileIds": [fb["id"]] }), Some(&an)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = app
        .put(
            &format!("/api/classes/{c}/steps/{hw}/homework"),
            json!({ "note": "Em làm 2 cách", "fileIds": [f1["id"], f2["id"]] }),
            Some(&an),
        )
        .await;
    ok(&res, StatusCode::OK);
    let sub = &res.body["submission"];
    assert_eq!(sub["status"], "submitted");
    assert_eq!(sub["late"], false);
    assert_eq!(sub["files"].as_array().unwrap().len(), 2);
    assert!(sub["files"][0]["file"]["url"].as_str().unwrap().contains("&sig="));
    let submission = sub["id"].as_str().unwrap().to_string();
    let learn = app.get(&format!("/api/classes/{c}/learn"), Some(&an)).await.body;
    assert_eq!(learn["steps"][0]["status"], "done");
    assert_eq!(learn["steps"][0]["homeworkStatus"], "submitted");
    assert_eq!(learn["steps"][1]["status"], "available");
    assert_eq!(notif_kinds(&app, &t).await[0], "homework_submitted");

    // Giáo viên: tổng quan + danh sách (chưa chấm lên đầu, chưa nộp cuối).
    let overview = app.get(&format!("/api/teaching/classes/{c}/homework"), Some(&t)).await;
    ok(&overview, StatusCode::OK);
    assert_eq!(overview.body[0]["assigned"], 3);
    assert_eq!(overview.body[0]["submitted"], 1);
    let subs = app.get(&format!("/api/teaching/classes/{c}/steps/{hw}/submissions"), Some(&t)).await;
    ok(&subs, StatusCode::OK);
    assert_eq!(subs.body["students"][0]["username"], "hs_an");
    assert_eq!(subs.body["students"][0]["submission"]["fileCount"], 2);
    assert!(subs.body["students"][2]["submission"].is_null());
    assert_eq!(
        app.get(&format!("/api/teaching/submissions/{submission}"), Some(&t2)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.get(&format!("/api/teaching/submissions/{submission}"), Some(&an)).await.status,
        StatusCode::NOT_FOUND
    );

    // Chấm: điểm ngoài thang → lỗi; ảnh đã vẽ phải là file của giáo viên.
    let grade_url = format!("/api/teaching/submissions/{submission}/grade");
    let res = app.post(&grade_url, json!({ "action": "grade", "score": 11 }), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = app
        .post(
            &grade_url,
            json!({ "action": "grade", "score": 8, "annotations": [{ "position": 0, "fileId": f1["id"] }] }),
            Some(&t),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let drawn = upload(&app, &t, "homework", "nhan-xet.png", "image/png", PNG).await;
    let res = app
        .post(
            &grade_url,
            json!({ "action": "grade", "score": 8, "feedback": "Cách 2 **gọn hơn**.", "annotations": [{ "position": 0, "fileId": drawn["id"] }] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["status"], "graded");
    assert_eq!(res.body["score"], 8.0);
    assert_eq!(res.body["files"][0]["annotated"]["id"], drawn["id"]);
    assert!(res.body["files"][1]["annotated"].is_null());

    // Học sinh thấy điểm + ảnh đã vẽ; không nộp lại được nữa.
    let view = app.get(&format!("/api/classes/{c}/steps/{hw}/homework"), Some(&an)).await.body;
    assert_eq!(view["submission"]["status"], "graded");
    assert_eq!(view["submission"]["feedback"], "Cách 2 **gọn hơn**.");
    assert_eq!(view["canSubmit"], false);
    assert!(view["submission"]["files"][0]["annotated"]["url"].as_str().unwrap().contains("&sig="));
    assert_eq!(notif_kinds(&app, &an).await[0], "homework_graded");
    let res =
        app.put(&format!("/api/classes/{c}/steps/{hw}/homework"), json!({ "fileIds": [f1["id"]] }), Some(&an)).await;
    assert_eq!(res.code(), "HOMEWORK_GRADED");

    // Trả lại để làm lại → bước chưa xong; nộp lại được.
    let fb2 = upload(&app, &binh, "homework", "binh.png", "image/png", PNG).await;
    let res =
        app.put(&format!("/api/classes/{c}/steps/{hw}/homework"), json!({ "fileIds": [fb2["id"]] }), Some(&binh)).await;
    let binh_sub = res.body["submission"]["id"].as_str().unwrap().to_string();
    let res = app
        .post(
            &format!("/api/teaching/submissions/{binh_sub}/grade"),
            json!({ "action": "return", "feedback": "Thiếu bước 2" }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["status"], "returned");
    let learn = app.get(&format!("/api/classes/{c}/learn"), Some(&binh)).await.body;
    assert_eq!(learn["steps"][0]["status"], "available");
    assert_eq!(learn["steps"][0]["homeworkStatus"], "returned");
    assert_eq!(notif_kinds(&app, &binh).await[0], "homework_returned");
    let res =
        app.put(&format!("/api/classes/{c}/steps/{hw}/homework"), json!({ "fileIds": [fb2["id"]] }), Some(&binh)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["submission"]["submitCount"], 2);

    // Tiến độ cả lớp có ô bài tập + điểm.
    let progress = app.get(&format!("/api/teaching/classes/{c}/progress"), Some(&t)).await.body;
    let an_row = progress["students"].as_array().unwrap().iter().find(|s| s["userId"] == json!(an_id)).unwrap().clone();
    assert_eq!(an_row["cells"][0]["homeworkStatus"], "graded");
    assert_eq!(an_row["cells"][0]["score"], 8.0);
    assert_eq!(an_row["cells"][0]["maxScore"], 10.0);

    // Nhắc hạn nộp: chỉ Chi (chưa nộp) được nhắc, một lần.
    app.emails();
    assert_eq!(homework::send_due_reminders(&app.ctx, Utc::now()).await.unwrap(), 1);
    assert_eq!(homework::send_due_reminders(&app.ctx, Utc::now()).await.unwrap(), 0);
    let emails = app.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].to, "hs_chi@example.com");
    assert!(emails[0].text.contains("Bài tập về nhà số 1"));
    assert_eq!(notif_kinds(&app, &chi).await[0], "homework_due");
    // Còn hơn 24 giờ thì chưa nhắc.
    assert_eq!(homework::send_due_reminders(&app.ctx, Utc::now() - Duration::hours(30)).await.unwrap(), 0);

    // Quá hạn vẫn nộp được, đánh dấu muộn.
    publish_path(
        &app,
        &t,
        &c,
        json!([
            { "id": hw, "kind": "homework", "assignmentId": a, "dueAt": Utc::now() - Duration::hours(1) },
            { "id": steps[1], "kind": "lesson", "lessonId": l, "unlock": "previous" }
        ]),
    )
    .await;
    let fc = upload(&app, &chi, "homework", "chi.png", "image/png", PNG).await;
    let res =
        app.put(&format!("/api/classes/{c}/steps/{hw}/homework"), json!({ "fileIds": [fc["id"]] }), Some(&chi)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["submission"]["late"], true);
    assert_eq!(res.body["overdue"], true);

    // Giáo viên xem thử: không nộp được.
    let res = app.get(&format!("/api/classes/{c}/steps/{hw}/homework"), Some(&t)).await;
    assert_eq!(res.body["isPreview"], true);
    let res =
        app.put(&format!("/api/classes/{c}/steps/{hw}/homework"), json!({ "fileIds": [drawn["id"]] }), Some(&t)).await;
    assert_eq!(res.code(), "PREVIEW_ONLY");

    // Ẩn bài tập → không xuất bản lại được.
    ok(&app.delete(&format!("/api/teaching/assignments/{a}"), Some(&t)).await, StatusCode::NO_CONTENT);
    let res = app.post(&format!("/api/teaching/classes/{c}/path/publish"), json!({}), Some(&t)).await;
    assert_eq!(res.code(), "ASSIGNMENT_ARCHIVED");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn lo_trinh_rieng_cho_nhom_va_hoc_sinh(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan).await;
    let (an_id, an) = student(&app, "hs_an", &c).await;
    let (binh_id, binh) = student(&app, "hs_binh", &c).await;
    let (_, outsider) = app.login_as("hs_ngoai", UserRole::Student).await;
    let outsider_id: Uuid =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'hs_ngoai'").fetch_one(&app.ctx.db).await.unwrap();
    let _ = outsider;

    let res =
        app.post("/api/teaching/groups", json!({ "name": "Nhóm nâng cao", "memberIds": [an_id] }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    let nhom = res.body["id"].as_str().unwrap().to_string();
    let l1 = lesson(&app, &t, toan, "Bài chung").await;
    let l2 = lesson(&app, &t, toan, "Bài nâng cao").await;
    let l3 = lesson(&app, &t, toan, "Bài phụ đạo").await;

    // Học sinh ngoài lớp / nhóm của người khác → lỗi.
    let bad = app
        .put(
            &format!("/api/teaching/classes/{c}/path"),
            json!({ "steps": [{ "kind": "lesson", "lessonId": l1, "audienceUserIds": [outsider_id] }] }),
            Some(&t),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    let bad = app
        .put(
            &format!("/api/teaching/classes/{c}/path"),
            json!({ "steps": [{ "kind": "lesson", "lessonId": l1, "audienceGroupIds": [Uuid::now_v7()] }] }),
            Some(&t),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);

    let steps = publish_path(
        &app,
        &t,
        &c,
        json!([
            { "kind": "milestone", "title": "Chương 1" },
            { "kind": "lesson", "lessonId": l1 },
            { "kind": "milestone", "title": "Nâng cao" },
            { "kind": "lesson", "lessonId": l2, "audienceGroupIds": [nhom] },
            { "kind": "lesson", "lessonId": l3, "audienceUserIds": [binh_id] }
        ]),
    )
    .await;
    let titles = |v: &Value| -> Vec<String> {
        v["steps"].as_array().unwrap().iter().map(|s| s["title"].as_str().unwrap().to_string()).collect()
    };

    let an_view = app.get(&format!("/api/classes/{c}/learn"), Some(&an)).await.body;
    assert_eq!(titles(&an_view), ["Chương 1", "Bài chung", "Nâng cao", "Bài nâng cao"]);
    assert_eq!(an_view["progress"]["total"], 2);
    let binh_view = app.get(&format!("/api/classes/{c}/learn"), Some(&binh)).await.body;
    assert_eq!(titles(&binh_view), ["Chương 1", "Bài chung", "Nâng cao", "Bài phụ đạo"]);
    // Bước không giao cho mình → không mở được.
    let res = app.get(&format!("/api/classes/{c}/steps/{}/lesson", steps[3]), Some(&binh)).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    ok(&app.get(&format!("/api/classes/{c}/steps/{}/lesson", steps[4]), Some(&binh)).await, StatusCode::OK);

    // Giáo viên xem thử thấy mọi bước, có đánh dấu bước riêng.
    let preview = app.get(&format!("/api/classes/{c}/learn"), Some(&t)).await.body;
    assert_eq!(titles(&preview).len(), 5);
    assert_eq!(preview["steps"][3]["restricted"], true);
    assert_eq!(preview["steps"][1]["restricted"], false);

    // Tiến độ cả lớp: ô không giao → assigned = false, không tính vào tổng.
    ok(&app.post(&format!("/api/classes/{c}/steps/{}/complete", steps[3]), json!({}), Some(&an)).await, StatusCode::OK);
    let progress = app.get(&format!("/api/teaching/classes/{c}/progress"), Some(&t)).await.body;
    let row =
        |id: Uuid| progress["students"].as_array().unwrap().iter().find(|s| s["userId"] == json!(id)).unwrap().clone();
    let an_row = row(an_id);
    assert_eq!(an_row["progress"]["total"], 2);
    assert_eq!(an_row["progress"]["done"], 1);
    assert_eq!(an_row["cells"][1]["assigned"], true);
    assert_eq!(an_row["cells"][2]["assigned"], false);
    let my = app.get("/api/my/classes", Some(&an)).await.body;
    assert_eq!(my[0]["progress"]["percent"], 50);
    let _ = row(binh_id);
}

async fn offer(app: &TestApp, t: &str, audience: &str) -> String {
    let all_week: Vec<Value> = (1..=7).map(|d| json!({ "weekday": d, "start": "00:00", "end": "23:00" })).collect();
    let res = app
        .post(
            "/api/teaching/tutoring/offers",
            json!({
                "title": "Học kèm Toán 9", "durationMinutes": 60, "windows": all_week, "audience": audience,
                "minNoticeHours": 1, "horizonDays": 3, "meetingUrl": "https://meet.google.com/abc-defg-hij"
            }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

async fn slots(app: &TestApp, token: &str, offer: &str) -> Vec<String> {
    let res = app.get(&format!("/api/tutoring/offers/{offer}/slots"), Some(token)).await;
    ok(&res, StatusCode::OK);
    res.body.as_array().unwrap().iter().map(|s| s["startsAt"].as_str().unwrap().to_string()).collect()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn hoc_kem_dat_lich_duyet_tu_choi_huy(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan).await;

    // Khung giờ sai → lỗi.
    let bad = app
        .post(
            "/api/teaching/tutoring/offers",
            json!({ "title": "x", "durationMinutes": 60, "windows": [{ "weekday": 1, "start": "20:00", "end": "19:00" }],
                    "audience": "everyone", "minNoticeHours": 0, "horizonDays": 7 }),
            Some(&t),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);

    let o = offer(&app, &t, "my_students").await;
    let mine = app.get("/api/teaching/tutoring/offers", Some(&t)).await.body;
    assert_eq!(mine[0]["meetingUrl"], "https://meet.google.com/abc-defg-hij");

    // Chỉ học sinh của giáo viên đặt được; học sinh không thấy link họp trước khi được duyệt.
    let (_, outsider) = app.login_as("hs_ngoai", UserRole::Student).await;
    assert_eq!(app.get(&format!("/api/tutoring/offers/{o}"), Some(&outsider)).await.status, StatusCode::NOT_FOUND);
    let (_, an) = student(&app, "hs_an", &c).await;
    let (_, binh) = student(&app, "hs_binh", &c).await;
    let seen = app.get("/api/tutoring/offers", Some(&an)).await.body;
    assert_eq!(seen.as_array().unwrap().len(), 1);
    assert!(seen[0]["meetingUrl"].is_null());
    let free = slots(&app, &an, &o).await;
    assert!(free.len() > 20, "{}", free.len());

    // Đặt khung đầu; người khác đặt trùng → SLOT_TAKEN.
    let book_url = format!("/api/tutoring/offers/{o}/bookings");
    let res =
        app.post(&book_url, json!({ "startsAt": free[0], "message": "Em muốn ôn hệ phương trình" }), Some(&an)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["status"], "pending");
    let first = res.body["id"].as_str().unwrap().to_string();
    let res = app.post(&book_url, json!({ "startsAt": free[0] }), Some(&binh)).await;
    assert_eq!(res.code(), "SLOT_TAKEN");
    assert!(!slots(&app, &binh, &o).await.contains(&free[0]));
    let res = app.post(&book_url, json!({ "startsAt": "2020-01-01T00:00:00Z" }), Some(&binh)).await;
    assert_eq!(res.code(), "SLOT_TAKEN");
    assert_eq!(notif_kinds(&app, &t).await[0], "booking_requested");
    let pending = app.get("/api/teaching/tutoring/bookings?status=pending", Some(&t)).await.body;
    assert_eq!(pending[0]["studentUsername"], "hs_an");
    assert_eq!(pending[0]["message"], "Em muốn ôn hệ phương trình");

    // Đồng ý → tạo buổi học, gửi lời mời .ics cho học sinh.
    app.emails();
    let res = app.post(&format!("/api/teaching/tutoring/bookings/{first}/approve"), json!({}), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["status"], "approved");
    let session = res.body["sessionId"].as_str().unwrap().to_string();
    let emails = app.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].to, "hs_an@example.com");
    assert_eq!(emails[0].attachments.len(), 1);
    assert_eq!(notif_kinds(&app, &an).await[0], "booking_approved");
    let s = app.get(&format!("/api/sessions/{session}"), Some(&an)).await;
    ok(&s, StatusCode::OK);
    let res = app.post(&format!("/api/teaching/tutoring/bookings/{first}/approve"), json!({}), Some(&t)).await;
    assert_eq!(res.code(), "BOOKING_NOT_PENDING");

    // Từ chối kèm lý do; khung vừa bị từ chối lại trống.
    let res = app.post(&book_url, json!({ "startsAt": free[2] }), Some(&binh)).await;
    let second = res.body["id"].as_str().unwrap().to_string();
    let res = app
        .post(&format!("/api/teaching/tutoring/bookings/{second}/decline"), json!({ "reason": "Cô bận họp" }), Some(&t))
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["status"], "declined");
    assert_eq!(notif_kinds(&app, &binh).await[0], "booking_declined");
    assert!(slots(&app, &binh, &o).await.contains(&free[2]));

    // Học sinh hủy lịch đã duyệt → hủy cả buổi học, báo giáo viên.
    let res = app.post(&format!("/api/my/bookings/{first}/cancel"), json!({ "reason": "Em bị ốm" }), Some(&binh)).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let res = app.post(&format!("/api/my/bookings/{first}/cancel"), json!({ "reason": "Em bị ốm" }), Some(&an)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["status"], "cancelled");
    let s = app.get(&format!("/api/sessions/{session}"), Some(&t)).await;
    assert_eq!(s.body["status"], "cancelled");
    assert_eq!(notif_kinds(&app, &t).await[0], "booking_cancelled");
    let my = app.get("/api/my/bookings?status=all", Some(&an)).await.body;
    assert_eq!(my[0]["status"], "cancelled");

    // Tối đa 3 yêu cầu đang chờ với một giáo viên.
    for slot in &free[3..6] {
        ok(&app.post(&book_url, json!({ "startsAt": slot }), Some(&an)).await, StatusCode::CREATED);
    }
    assert_eq!(app.post(&book_url, json!({ "startsAt": free[7] }), Some(&an)).await.code(), "TOO_MANY_PENDING");

    // Mở cho mọi tài khoản → người ngoài xem / đặt được.
    let o2 = offer(&app, &t, "everyone").await;
    ok(&app.get(&format!("/api/tutoring/offers/{o2}"), Some(&outsider)).await, StatusCode::OK);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn ho_so_giao_vien_va_trang_cong_khai(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "co_lan", "toan").await;
    let c = class(&app, &t, toan).await;
    let l1 = lesson(&app, &t, toan, "Căn bậc hai").await;
    let l2 = lesson(&app, &t, toan, "Bài riêng nhóm giỏi").await;
    let (an_id, _) = student(&app, "hs_an", &c).await;
    publish_path(
        &app,
        &t,
        &c,
        json!([
            { "kind": "milestone", "title": "Chương 1" },
            { "kind": "lesson", "lessonId": l1 },
            { "kind": "lesson", "lessonId": l2, "audienceUserIds": [an_id] }
        ]),
    )
    .await;
    offer(&app, &t, "everyone").await;
    offer(&app, &t, "my_students").await;

    let res = app
        .put(
            "/api/teaching/profile",
            json!({ "headline": "Giáo viên Toán THCS, 10 năm ôn thi vào 10", "bio": "Tôi dạy **Toán** 9.\n\nHọc sinh <đạt> 9+.", "isPublic": true }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["publicPath"], "/p/giao-vien/co_lan");

    // Dữ liệu công khai (không đăng nhập): chỉ khung học kèm mở cho mọi tài khoản.
    let p = app.get("/api/public/teachers/co_lan", None).await;
    ok(&p, StatusCode::OK);
    assert_eq!(p.body["headline"], "Giáo viên Toán THCS, 10 năm ôn thi vào 10");
    assert_eq!(p.body["classes"][0]["title"], "Toán 9");
    assert_eq!(p.body["subjects"].as_array().unwrap().len(), 1);
    assert_eq!(p.body["offers"].as_array().unwrap().len(), 1);
    let by_id = app.get(&format!("/api/public/teachers/{}", p.body["id"].as_str().unwrap()), None).await;
    ok(&by_id, StatusCode::OK);
    assert_eq!(app.get("/api/public/teachers/hs_an", None).await.status, StatusCode::NOT_FOUND);

    // Trang HTML cho máy tìm kiếm.
    let html = app.get("/p/giao-vien/co_lan", None).await;
    ok(&html, StatusCode::OK);
    let text = html.body.as_str().unwrap();
    assert!(text.contains("<title>User co_lan — giáo viên — Smart Learning Space</title>"), "{text}");
    assert!(text.contains("og:title"));
    assert!(text.contains("\"@type\":\"Person\""));
    assert!(text.contains(&format!("href=\"/p/lop-hoc/{c}\"")));
    assert!(text.contains("Học sinh &lt;đạt&gt; 9+."));
    assert!(!text.contains("<đạt>"));
    let html = app.get(&format!("/p/lop-hoc/{c}"), None).await;
    ok(&html, StatusCode::OK);
    let text = html.body.as_str().unwrap();
    assert!(text.contains("\"@type\":\"Course\""));
    assert!(text.contains("Căn bậc hai"));
    assert!(!text.contains("Bài riêng nhóm giỏi"), "bước của lộ trình riêng không hiện công khai");
    assert_eq!(app.get("/p/lop-hoc/khong-phai-id", None).await.status, StatusCode::NOT_FOUND);

    let sitemap = app.get("/sitemap.xml", None).await.body;
    let sitemap = sitemap.as_str().unwrap();
    assert!(sitemap.contains("/p/giao-vien/co_lan</loc>"));
    assert!(sitemap.contains(&format!("/p/lop-hoc/{c}</loc>")));
    assert!(app.get("/robots.txt", None).await.body.as_str().unwrap().contains("Sitemap: "));

    // Tắt công khai → người khác không xem được, chính mình vẫn xem được; không còn trong sitemap.
    ok(&app.put("/api/teaching/profile", json!({ "isPublic": false }), Some(&t)).await, StatusCode::OK);
    assert_eq!(app.get("/api/public/teachers/co_lan", None).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get("/p/giao-vien/co_lan", None).await.status, StatusCode::NOT_FOUND);
    ok(&app.get("/api/public/teachers/co_lan", Some(&t)).await, StatusCode::OK);
    assert!(!app.get("/sitemap.xml", None).await.body.as_str().unwrap().contains("co_lan"));

    // Lớp học riêng tư không có trang công khai.
    ok(
        &app.put(
            &format!("/api/teaching/classes/{c}"),
            json!({ "subjectId": toan, "title": "Toán 9", "mode": "self_paced", "visibility": "private", "enrollment": "open" }),
            Some(&t),
        )
        .await,
        StatusCode::OK,
    );
    assert_eq!(app.get(&format!("/p/lop-hoc/{c}"), None).await.status, StatusCode::NOT_FOUND);
}
