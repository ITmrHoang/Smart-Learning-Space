//! M8 + M9: lớp học, ghi danh, lộ trình, học theo lộ trình, làm đề trong lớp, tiến độ.

mod common;

use axum::http::StatusCode;
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;
use uuid::Uuid;

fn class_body(subject: Uuid, title: &str, visibility: &str, enrollment: &str) -> Value {
    json!({
        "subjectId": subject, "title": title, "description": "Giới thiệu", "mode": "self_paced",
        "visibility": visibility, "enrollment": enrollment, "gradeIds": [], "tagIds": []
    })
}

async fn create_class(app: &TestApp, t: &str, body: Value) -> Value {
    let res = app.post("/api/teaching/classes", body, Some(t)).await;
    ok(&res, StatusCode::CREATED);
    res.body
}

async fn lesson(app: &TestApp, t: &str, subject: Uuid, title: &str, publish: bool) -> String {
    let res = app
        .post(
            "/api/teaching/lessons",
            json!({ "subjectId": subject, "title": title, "content": format!("Nội dung {title}") }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let id = res.body["id"].as_str().unwrap().to_string();
    if publish {
        ok(&app.post(&format!("/api/teaching/lessons/{id}/publish"), json!({}), Some(t)).await, StatusCode::OK);
    }
    id
}

/// Đề kiểm tra 1 câu một đáp án (đáp án đúng: PA 0), 10 điểm.
async fn exam(app: &TestApp, t: &str, subject: Uuid, extra: Value) -> String {
    let q = app
        .post(
            "/api/teaching/questions",
            json!({
                "subjectId": subject, "kind": "single", "level": "nhan_biet", "content": "1 + 1 = ?",
                "options": [{ "content": "PA 0", "correct": true }, { "content": "PA 1" }, { "content": "PA 2" }],
                "explanation": "Vì 2"
            }),
            Some(t),
        )
        .await;
    ok(&q, StatusCode::CREATED);
    let mut body = json!({
        "subjectId": subject, "title": "Kiểm tra nhanh", "kind": "test", "showAnswers": "after_submit",
        "sections": [{ "title": "I", "scoring": "per_question", "pointsPerQuestion": 10, "questionIds": [q.body["id"]] }]
    });
    for (k, v) in extra.as_object().unwrap() {
        body[k] = v.clone();
    }
    let res = app.post("/api/teaching/exams", body, Some(t)).await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

/// Học sinh làm đề của bước: chọn PA `n`, nộp. Trả về view sau khi nộp.
async fn take_exam(app: &TestApp, s: &str, class: &str, step: &str, n: usize) -> Value {
    let res = app.post(&format!("/api/classes/{class}/steps/{step}/attempts"), json!({}), Some(s)).await;
    ok(&res, StatusCode::CREATED);
    let attempt = res.body["attemptId"].as_str().unwrap().to_string();
    let view = app.get(&format!("/api/attempts/{attempt}"), Some(s)).await.body;
    let item = &view["items"][0];
    let opt =
        item["options"].as_array().unwrap().iter().find(|o| o["content"] == format!("PA {n}")).unwrap()["id"].clone();
    ok(
        &app.put(&format!("/api/attempts/{attempt}/items/0"), json!({ "selected": [opt] }), Some(s)).await,
        StatusCode::NO_CONTENT,
    );
    let res = app.post(&format!("/api/attempts/{attempt}/submit"), json!({}), Some(s)).await;
    ok(&res, StatusCode::OK);
    res.body
}

fn statuses(learn: &Value) -> Vec<String> {
    learn["steps"].as_array().unwrap().iter().map(|s| s["status"].as_str().unwrap().to_string()).collect()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn hien_thi_va_ghi_danh(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let open = create_class(&app, &t, class_body(toan, "Toán 9 tự học", "public", "open")).await;
    let approval = create_class(&app, &t, class_body(toan, "Toán 9 có duyệt", "public", "approval")).await;
    let invite = create_class(&app, &t, class_body(toan, "Lớp kín", "private", "invite")).await;
    assert_eq!(open["inviteCode"].as_str().unwrap().len(), 8);
    let id = |c: &Value| c["id"].as_str().unwrap().to_string();

    // Danh sách công khai (không cần đăng nhập) chỉ có lớp public; lớp riêng tư không xem được.
    let catalog = app.get("/api/classes", None).await;
    assert_eq!(catalog.body["total"], 2);
    assert_eq!(catalog.body["items"][0]["teacherName"], "User gv");
    assert_eq!(app.get(&format!("/api/classes/{}", id(&invite)), None).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get("/api/classes?q=co%20duyet", None).await.body["total"], 1);

    let (hs_id, s) = app.login_as("hs", UserRole::Student).await;
    // open → vào ngay; ghi danh lại → 409.
    let res = app.post(&format!("/api/classes/{}/enroll", id(&open)), json!({}), Some(&s)).await;
    assert_eq!(res.body["status"], "active");
    assert_eq!(
        app.post(&format!("/api/classes/{}/enroll", id(&open)), json!({}), Some(&s)).await.code(),
        "ALREADY_MEMBER"
    );
    // approval → chờ duyệt; chưa học được.
    let res = app.post(&format!("/api/classes/{}/enroll", id(&approval)), json!({}), Some(&s)).await;
    assert_eq!(res.body["status"], "pending");
    assert_eq!(app.get(&format!("/api/classes/{}/learn", id(&approval)), Some(&s)).await.code(), "NOT_ENROLLED");
    let members = app.get(&format!("/api/teaching/classes/{}/members?status=pending", id(&approval)), Some(&t)).await;
    assert_eq!(members.body.as_array().unwrap().len(), 1);
    ok(
        &app.post(&format!("/api/teaching/classes/{}/members/{hs_id}/approve", id(&approval)), json!({}), Some(&t))
            .await,
        StatusCode::NO_CONTENT,
    );
    ok(&app.get(&format!("/api/classes/{}/learn", id(&approval)), Some(&s)).await, StatusCode::OK);
    // invite → phải có mã; mã sai / đúng.
    assert_eq!(
        app.post(&format!("/api/classes/{}/enroll", id(&invite)), json!({}), Some(&s)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.post("/api/classes/join", json!({ "code": "SAIMA999" }), Some(&s)).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let code = invite["inviteCode"].as_str().unwrap().to_lowercase();
    let res = app.post("/api/classes/join", json!({ "code": code }), Some(&s)).await;
    assert_eq!(res.body["status"], "active");
    assert_eq!(app.get(&format!("/api/classes/{}", id(&invite)), Some(&s)).await.body["myStatus"], "active");

    // Lớp học của tôi.
    assert_eq!(app.get("/api/my/classes", Some(&s)).await.body.as_array().unwrap().len(), 3);
    // Rời lớp / bị mời ra thì không tự vào lại được.
    ok(&app.post(&format!("/api/classes/{}/leave", id(&open)), json!({}), Some(&s)).await, StatusCode::NO_CONTENT);
    assert_eq!(
        app.post(&format!("/api/classes/{}/enroll", id(&open)), json!({}), Some(&s)).await.body["status"],
        "active"
    );
    ok(
        &app.post(&format!("/api/teaching/classes/{}/members/{hs_id}/remove", id(&open)), json!({}), Some(&t)).await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(
        app.post(&format!("/api/classes/{}/enroll", id(&open)), json!({}), Some(&s)).await.code(),
        "REMOVED_FROM_CLASS"
    );
    // Giáo viên thêm lại bằng email.
    let res = app
        .post(&format!("/api/teaching/classes/{}/members", id(&open)), json!({ "login": "HS@example.com" }), Some(&t))
        .await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["status"], "active");

    // Đổi mã mời → mã cũ hết dùng được.
    let (_, s2) = app.login_as("hs2", UserRole::Student).await;
    ok(
        &app.post(&format!("/api/teaching/classes/{}/invite-code", id(&invite)), json!({}), Some(&t)).await,
        StatusCode::OK,
    );
    assert_eq!(
        app.post("/api/classes/join", json!({ "code": code }), Some(&s2)).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    // Học sinh không vào được khu giáo viên của lớp.
    assert_eq!(
        app.get(&format!("/api/teaching/classes/{}/members", id(&open)), Some(&s)).await.status,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn lo_trinh_xuat_ban_va_hoc_theo_lo_trinh(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let class = create_class(&app, &t, class_body(toan, "Toán 9", "public", "open")).await;
    let cid = class["id"].as_str().unwrap().to_string();
    let l1 = lesson(&app, &t, toan, "Bài 1", true).await;
    let l2 = lesson(&app, &t, toan, "Bài 2", false).await;
    let ex = exam(&app, &t, toan, json!({})).await;

    let path = json!({ "steps": [
        { "kind": "milestone", "title": "Tuần 1" },
        { "kind": "lesson", "lessonId": l1 },
        { "kind": "lesson", "lessonId": l2, "unlock": "previous" },
        { "kind": "exam", "examId": ex, "unlock": "previous", "passScore": 5 }
    ]});
    let res = app.put(&format!("/api/teaching/classes/{cid}/path"), path, Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["steps"][2]["refStatus"], "draft");
    assert_eq!(res.body["hasChanges"], true);
    // Bài 2 chưa xuất bản → chưa xuất bản lộ trình được.
    let res = app.post(&format!("/api/teaching/classes/{cid}/path/publish"), json!({}), Some(&t)).await;
    assert_eq!(res.code(), "LESSON_NOT_PUBLISHED");
    ok(&app.post(&format!("/api/teaching/lessons/{l2}/publish"), json!({}), Some(&t)).await, StatusCode::OK);
    let res = app.post(&format!("/api/teaching/classes/{cid}/path/publish"), json!({}), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["publishedVersion"], 1);
    assert_eq!(res.body["hasChanges"], false);
    let steps: Vec<String> =
        res.body["steps"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect();

    // Trang giới thiệu có mục lục (tên bước), không có nội dung.
    let detail = app.get(&format!("/api/classes/{cid}"), None).await;
    assert_eq!(detail.body["outline"][1]["title"], "Bài 1");
    assert_eq!(detail.body["stepCount"], 3);

    let (_, s) = app.login_as("hs", UserRole::Student).await;
    app.post(&format!("/api/classes/{cid}/enroll"), json!({}), Some(&s)).await;
    let learn = app.get(&format!("/api/classes/{cid}/learn"), Some(&s)).await;
    assert_eq!(statuses(&learn.body), vec!["available", "available", "locked", "locked"]);
    assert_eq!(learn.body["progress"], json!({ "done": 0, "total": 3, "percent": 0 }));

    // Bước khóa không mở được.
    assert_eq!(app.get(&format!("/api/classes/{cid}/steps/{}/lesson", steps[2]), Some(&s)).await.code(), "STEP_LOCKED");
    let lesson1 = app.get(&format!("/api/classes/{cid}/steps/{}/lesson", steps[1]), Some(&s)).await;
    assert_eq!(lesson1.body["lesson"]["content"], "Nội dung Bài 1");
    assert_eq!(lesson1.body["nextStepId"], steps[2].as_str());
    let learn = app.post(&format!("/api/classes/{cid}/steps/{}/complete", steps[1]), json!({}), Some(&s)).await;
    assert_eq!(statuses(&learn.body), vec!["available", "done", "available", "locked"]);
    app.post(&format!("/api/classes/{cid}/steps/{}/complete", steps[2]), json!({}), Some(&s)).await;

    // Làm đề: sai (0 điểm < 5) → chưa xong; đúng → xong.
    let wrong = take_exam(&app, &s, &cid, &steps[3], 1).await;
    assert_eq!(wrong["score"], 0.0);
    let learn = app.get(&format!("/api/classes/{cid}/learn"), Some(&s)).await;
    assert_eq!(learn.body["steps"][3]["status"], "available");
    assert_eq!(learn.body["steps"][3]["submittedCount"], 1);
    take_exam(&app, &s, &cid, &steps[3], 0).await;
    let learn = app.get(&format!("/api/classes/{cid}/learn"), Some(&s)).await;
    assert_eq!(learn.body["steps"][3]["status"], "done");
    assert_eq!(learn.body["steps"][3]["bestScore"], 10.0);
    assert_eq!(learn.body["progress"]["percent"], 100);
    assert_eq!(
        app.get(&format!("/api/classes/{cid}/steps/{}/attempts", steps[3]), Some(&s))
            .await
            .body
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Giáo viên xem tiến độ cả lớp + kết quả đề theo lớp.
    let progress = app.get(&format!("/api/teaching/classes/{cid}/progress"), Some(&t)).await;
    assert_eq!(progress.body["steps"].as_array().unwrap().len(), 3);
    assert_eq!(progress.body["students"][0]["progress"]["percent"], 100);
    assert_eq!(progress.body["students"][0]["cells"][2]["score"], 10.0);
    let attempts = app.get(&format!("/api/teaching/exams/{ex}/attempts?classId={cid}"), Some(&t)).await;
    assert_eq!(attempts.body.as_array().unwrap().len(), 2);
    let attempt_id = attempts.body[0]["id"].as_str().unwrap();
    let teacher_view = app.get(&format!("/api/attempts/{attempt_id}"), Some(&t)).await;
    assert_eq!(teacher_view.body["student"]["username"], "hs");
    assert_eq!(teacher_view.body["reveal"], true);
    // Học sinh khác không xem được bài của bạn.
    let (_, s2) = app.login_as("hs2", UserRole::Student).await;
    assert_eq!(app.get(&format!("/api/attempts/{attempt_id}"), Some(&s2)).await.status, StatusCode::NOT_FOUND);

    // Sửa lộ trình: đổi thứ tự + thêm bài mới, giữ ID cũ → không mất tiến độ.
    let l3 = lesson(&app, &t, toan, "Bài 3", true).await;
    let path = json!({ "steps": [
        { "id": steps[1], "kind": "lesson", "lessonId": l1 },
        { "kind": "lesson", "lessonId": l3 },
        { "id": steps[0], "kind": "milestone", "title": "Tuần 1" },
        { "id": steps[2], "kind": "lesson", "lessonId": l2, "unlock": "previous" },
        { "id": steps[3], "kind": "exam", "examId": ex, "unlock": "previous", "passScore": 5 }
    ]});
    ok(&app.put(&format!("/api/teaching/classes/{cid}/path"), path, Some(&t)).await, StatusCode::OK);
    // Học sinh vẫn thấy bản cũ cho tới khi xuất bản.
    assert_eq!(app.get(&format!("/api/classes/{cid}/learn"), Some(&s)).await.body["progress"]["total"], 3);
    ok(&app.post(&format!("/api/teaching/classes/{cid}/path/publish"), json!({}), Some(&t)).await, StatusCode::OK);
    let learn = app.get(&format!("/api/classes/{cid}/learn"), Some(&s)).await;
    assert_eq!(statuses(&learn.body), vec!["done", "available", "available", "done", "done"]);
    assert_eq!(learn.body["progress"], json!({ "done": 3, "total": 4, "percent": 75 }));
    assert_eq!(learn.body["pathVersion"], 2);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn de_kiem_tra_trong_lop_gioi_han_va_het_gio(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let class = create_class(&app, &t, class_body(toan, "Lớp", "public", "open")).await;
    let cid = class["id"].as_str().unwrap().to_string();
    let ex = exam(&app, &t, toan, json!({ "maxAttempts": 1, "showAnswers": "never", "timeLimitMinutes": 10 })).await;
    let closed =
        exam(&app, &t, toan, json!({ "closesAt": "2020-01-01T00:00:00Z", "opensAt": "2019-01-01T00:00:00Z" })).await;
    app.put(
        &format!("/api/teaching/classes/{cid}/path"),
        json!({ "steps": [{ "kind": "exam", "examId": ex }, { "kind": "exam", "examId": closed }] }),
        Some(&t),
    )
    .await;
    let res = app.post(&format!("/api/teaching/classes/{cid}/path/publish"), json!({}), Some(&t)).await;
    let steps: Vec<String> =
        res.body["steps"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect();

    let (_, s) = app.login_as("hs", UserRole::Student).await;
    app.post(&format!("/api/classes/{cid}/enroll"), json!({}), Some(&s)).await;
    // Đề đã đóng.
    let res = app.post(&format!("/api/classes/{cid}/steps/{}/attempts", steps[1]), json!({}), Some(&s)).await;
    assert_eq!(res.code(), "EXAM_CLOSED");

    // Hết giờ → lần gọi kế tiếp tự nộp bài; không hiện đáp án (showAnswers = never) nhưng có điểm.
    let res = app.post(&format!("/api/classes/{cid}/steps/{}/attempts", steps[0]), json!({}), Some(&s)).await;
    let attempt: Uuid = res.body["attemptId"].as_str().unwrap().parse().unwrap();
    let view = app.get(&format!("/api/attempts/{attempt}"), Some(&s)).await.body;
    assert!(view["deadlineAt"].is_string());
    let opt =
        view["items"][0]["options"].as_array().unwrap().iter().find(|o| o["content"] == "PA 0").unwrap()["id"].clone();
    app.put(&format!("/api/attempts/{attempt}/items/0"), json!({ "selected": [opt] }), Some(&s)).await;
    sqlx::query("UPDATE attempts SET deadline_at = now() - interval '1 minute' WHERE id = $1")
        .bind(attempt)
        .execute(&app.ctx.db)
        .await
        .unwrap();
    let res = app.put(&format!("/api/attempts/{attempt}/items/0"), json!({ "selected": [] }), Some(&s)).await;
    assert_eq!(res.code(), "ATTEMPT_CLOSED");
    let view = app.get(&format!("/api/attempts/{attempt}"), Some(&s)).await.body;
    assert_eq!(view["status"], "submitted");
    assert_eq!(view["score"], 10.0);
    assert_eq!(view["reveal"], false);
    assert!(view["items"][0]["result"].is_null());
    // Hết lượt làm.
    let res = app.post(&format!("/api/classes/{cid}/steps/{}/attempts", steps[0]), json!({}), Some(&s)).await;
    assert_eq!(res.code(), "ATTEMPT_LIMIT");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn sao_chep_luu_tru_va_xem_truoc(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let class = create_class(&app, &t, class_body(toan, "Toán 9 khóa 1", "public", "open")).await;
    let cid = class["id"].as_str().unwrap().to_string();
    let l1 = lesson(&app, &t, toan, "Bài 1", true).await;
    app.put(
        &format!("/api/teaching/classes/{cid}/path"),
        json!({ "steps": [{ "kind": "lesson", "lessonId": l1 }, { "kind": "lesson", "lessonId": l1, "unlock": "previous" }] }),
        Some(&t),
    )
    .await;
    app.post(&format!("/api/teaching/classes/{cid}/path/publish"), json!({}), Some(&t)).await;
    let (_, s) = app.login_as("hs", UserRole::Student).await;
    app.post(&format!("/api/classes/{cid}/enroll"), json!({}), Some(&s)).await;

    // Giáo viên xem như học sinh: mọi bước đều mở, không ghi tiến độ.
    let learn = app.get(&format!("/api/classes/{cid}/learn"), Some(&t)).await;
    assert_eq!(learn.body["isPreview"], true);
    assert_eq!(statuses(&learn.body), vec!["available", "available"]);

    // Sao chép: chép thông tin + bản nháp lộ trình, không chép học sinh.
    let res =
        app.post(&format!("/api/teaching/classes/{cid}/copy"), json!({ "title": "Toán 9 khóa 2" }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["studentCount"], 0);
    assert_eq!(res.body["pathVersion"], Value::Null);
    let copy_id = res.body["id"].as_str().unwrap();
    let path = app.get(&format!("/api/teaching/classes/{copy_id}/path"), Some(&t)).await;
    assert_eq!(path.body["steps"].as_array().unwrap().len(), 2);
    assert_ne!(path.body["steps"][0]["id"], learn.body["steps"][0]["id"]);

    // Lưu trữ: học sinh không học được, lớp biến khỏi danh sách công khai.
    ok(&app.post(&format!("/api/teaching/classes/{cid}/archive"), json!({}), Some(&t)).await, StatusCode::OK);
    assert_eq!(app.get(&format!("/api/classes/{cid}/learn"), Some(&s)).await.code(), "NOT_ENROLLED");
    assert_eq!(app.get("/api/classes", None).await.body["total"], 1);
    assert_eq!(app.get("/api/teaching/classes?archived=true", Some(&t)).await.body["total"], 1);
    ok(&app.post(&format!("/api/teaching/classes/{cid}/restore"), json!({}), Some(&t)).await, StatusCode::OK);
    ok(&app.get(&format!("/api/classes/{cid}/learn"), Some(&s)).await, StatusCode::OK);
}
