//! Đợt 9 (M38, M39): câu tự luận + hướng dẫn chấm, trả lời ngắn sai số / biểu thức, điểm riêng từng câu, học sinh
//! nộp ảnh bài làm, giáo viên / trợ giảng chấm theo bước, nhận xét mẫu.

mod common;

use axum::http::StatusCode;
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;
use uuid::Uuid;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nbai-lam";

async fn create_question(app: &TestApp, t: &str, body: Value) -> Value {
    let res = app.post("/api/teaching/questions", body, Some(t)).await;
    ok(&res, StatusCode::CREATED);
    res.body
}

/// Upload ảnh (mục đích `homework`) lên kho trong bộ nhớ như trình duyệt.
async fn photo(app: &TestApp, t: &str, name: &str) -> String {
    let res = app
        .post(
            "/api/files",
            json!({ "purpose": "homework", "name": name, "contentType": "image/png", "size": PNG.len() }),
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
    app.ctx.storage.memory_put(&key, PNG, "image/png");
    ok(&app.post(&format!("/api/files/{id}/complete"), json!({}), Some(t)).await, StatusCode::OK);
    id
}

async fn notification_count(app: &TestApp, token: &str) -> usize {
    app.get("/api/notifications", Some(token)).await.body["items"].as_array().unwrap().len()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn cau_tu_luan_va_tra_loi_ngan_nang_cao(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;

    // Tự luận có hướng dẫn chấm 3 bước (tổng 1 điểm)
    let essay = create_question(
        &app,
        &t,
        json!({
            "subjectId": toan, "kind": "essay", "level": "van_dung", "content": "Giải $x^2 - 5x + 6 = 0$.",
            "explanation": "$\\Delta = 1$, $x = 2$ hoặc $x = 3$.",
            "rubric": [
                { "content": "Tính $\\Delta = 1$", "points": 0.25 },
                { "content": "Tìm đúng hai nghiệm", "points": 0.5 },
                { "content": "Kết luận", "points": 0.25 }
            ]
        }),
    )
    .await;
    assert_eq!(essay["kind"], "essay");
    assert_eq!(essay["rubric"].as_array().unwrap().len(), 3);
    let step_id = essay["rubric"][1]["id"].clone();

    // Sửa câu: giữ ID bước cũ
    let mut edit = json!({
        "subjectId": toan, "kind": "essay", "level": "van_dung", "content": "Giải $x^2 - 5x + 6 = 0$ (sửa).",
        "rubric": [
            { "id": essay["rubric"][0]["id"], "content": "Tính Δ", "points": 0.25 },
            { "id": step_id, "content": "Tìm đúng hai nghiệm", "points": 0.75 }
        ]
    });
    let res =
        app.put(&format!("/api/teaching/questions/{}", essay["id"].as_str().unwrap()), edit.clone(), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["question"]["rubric"][1]["id"], step_id);
    assert_eq!(res.body["answerChanged"], false);

    // Lỗi: tự luận có phương án; bước chấm ở câu không phải tự luận; biểu thức không đọc được
    edit["options"] = json!([{ "content": "A", "correct": true }]);
    let res = app.post("/api/teaching/questions", edit, Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = app
        .post(
            "/api/teaching/questions",
            json!({ "subjectId": toan, "kind": "short", "level": "nhan_biet", "content": "x?", "accepted": ["1"],
                    "rubric": [{ "content": "b", "points": 1 }] }),
            Some(&t),
        )
        .await;
    assert!(res.body["error"]["fields"]["rubric"][0].is_string(), "{}", res.body);
    let res = app
        .post(
            "/api/teaching/questions",
            json!({ "subjectId": toan, "kind": "short", "level": "nhan_biet", "content": "Rút gọn",
                    "accepted": ["2x +"], "matchMode": "expression" }),
            Some(&t),
        )
        .await;
    assert!(res.body["error"]["fields"]["accepted"][0].as_str().unwrap().contains("2x +"), "{}", res.body);

    // Trả lời ngắn: biểu thức tương đương + sai số
    let expr_q = create_question(
        &app,
        &t,
        json!({ "subjectId": toan, "kind": "short", "level": "thong_hieu", "content": "Khai triển $2(x+1)$",
                "accepted": ["2(x+1)"], "matchMode": "expression" }),
    )
    .await;
    assert_eq!(expr_q["matchMode"], "expression");
    let tol_q = create_question(
        &app,
        &t,
        json!({ "subjectId": toan, "kind": "short", "level": "nhan_biet", "content": "$\\pi$ ≈ ?",
                "accepted": ["3,14"], "tolerance": 0.01 }),
    )
    .await;
    assert_eq!(tol_q["tolerance"], 0.01);

    // Đề: điểm riêng của câu tự luận (2 điểm) > tổng bước (1 điểm); câu tự luận không đặt điểm = tổng bước
    let essay2 = create_question(
        &app,
        &t,
        json!({ "subjectId": toan, "kind": "essay", "level": "van_dung_cao", "content": "Chứng minh …" }),
    )
    .await;
    let ids = [&expr_q["id"], &tol_q["id"], &essay["id"], &essay2["id"]];
    let res = app
        .post(
            "/api/teaching/exams",
            json!({
                "subjectId": toan, "title": "Kiểm tra 45 phút", "kind": "test", "showAnswers": "after_submit",
                "shuffleQuestions": false,
                "sections": [
                    { "title": "I. Trả lời ngắn", "scoring": "per_question", "pointsPerQuestion": 0.5,
                      "questionIds": [ids[0], ids[1]] },
                    { "title": "II. Tự luận", "scoring": "per_question", "pointsPerQuestion": 3,
                      "questionIds": [ids[2], ids[3]], "questionPoints": { essay["id"].as_str().unwrap(): 2 } }
                ]
            }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    // 0,5 + 0,5 + 2 (điểm riêng) + 3 (tự luận không có bước chấm → điểm của phần)
    assert_eq!(res.body["maxScore"], 6.0);
    assert_eq!(res.body["sections"][1]["questionPoints"][essay["id"].as_str().unwrap()], 2.0);
    let exam_id = res.body["id"].as_str().unwrap().to_string();

    // Điểm riêng sai: câu không có trong phần
    let res = app
        .put(
            &format!("/api/teaching/exams/{exam_id}"),
            json!({
                "subjectId": toan, "title": "Kiểm tra", "kind": "test", "showAnswers": "after_submit",
                "sections": [{ "title": "I", "scoring": "per_question", "pointsPerQuestion": 1,
                               "questionIds": [ids[0]], "questionPoints": { Uuid::now_v7().to_string(): 1 } }]
            }),
            Some(&t),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Giáo viên làm thử: biểu thức tương đương + sai số chấm tự động, tự luận chờ chấm
    let view = app.post(&format!("/api/teaching/exams/{exam_id}/preview"), json!({}), Some(&t)).await;
    ok(&view, StatusCode::OK);
    let attempt = view.body["id"].as_str().unwrap().to_string();
    assert_eq!(view.body["maxScore"], 6.0);
    assert_eq!(view.body["items"][2]["manual"], true);
    assert_eq!(view.body["items"][2]["points"], 2.0);
    let put = |pos: i32, body: Value| {
        let app = &app;
        let t = t.clone();
        let attempt = attempt.clone();
        async move { app.put(&format!("/api/attempts/{attempt}/items/{pos}"), body, Some(&t)).await }
    };
    ok(&put(0, json!({ "text": "2 + 2x" })).await, StatusCode::NO_CONTENT);
    ok(&put(1, json!({ "text": "3,145" })).await, StatusCode::NO_CONTENT);
    ok(&put(2, json!({ "text": "Δ = 25 − 24 = 1 nên x = 2, x = 3" })).await, StatusCode::NO_CONTENT);
    // Câu trắc nghiệm / trả lời ngắn không nhận ảnh; tự luận chỉ nhận file của chính mình
    assert_eq!(put(0, json!({ "files": [Uuid::now_v7()] })).await.status, StatusCode::BAD_REQUEST);
    assert_eq!(put(3, json!({ "files": [Uuid::now_v7()] })).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    // Ôn tập mới kiểm tra từng câu; tự luận càng không
    let res = app.post(&format!("/api/attempts/{attempt}/submit"), json!({}), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["score"], 1.0);
    assert_eq!(res.body["pending"], 2);
    assert_eq!(res.body["items"][0]["result"]["isCorrect"], true);
    assert_eq!(res.body["items"][1]["result"]["isCorrect"], true);
    assert_eq!(res.body["items"][2]["grading"]["pending"], true);
    assert_eq!(res.body["items"][2]["result"]["rubric"].as_array().unwrap().len(), 2);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn hoc_sinh_nop_anh_giao_vien_cham_theo_buoc(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (_, s) = app.login_as("hs", UserRole::Student).await;
    let essay = create_question(
        &app,
        &t,
        json!({
            "subjectId": toan, "kind": "essay", "level": "van_dung", "content": "Giải phương trình.",
            "rubric": [{ "content": "Tính Δ", "points": 0.5 }, { "content": "Kết luận", "points": 0.5 }]
        }),
    )
    .await;
    let steps: Vec<String> =
        essay["rubric"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect();
    let free = create_question(
        &app,
        &t,
        json!({ "subjectId": toan, "kind": "essay", "level": "van_dung", "content": "Nêu nhận xét." }),
    )
    .await;
    let res = app
        .post(
            "/api/teaching/exams",
            json!({
                "subjectId": toan, "title": "Tự luận tuần 3", "kind": "test", "showAnswers": "never",
                "shuffleQuestions": false,
                "sections": [{ "title": "Tự luận", "scoring": "per_question", "pointsPerQuestion": 1,
                               "questionIds": [essay["id"], free["id"]] }]
            }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let exam_id = res.body["id"].as_str().unwrap().to_string();

    // Lớp + lộ trình có bước đề; học sinh ghi danh
    let class = app
        .post(
            "/api/teaching/classes",
            json!({ "subjectId": toan, "title": "Toán 9", "mode": "self_paced", "visibility": "public",
                    "enrollment": "open", "gradeIds": [], "tagIds": [] }),
            Some(&t),
        )
        .await;
    ok(&class, StatusCode::CREATED);
    let cid = class.body["id"].as_str().unwrap().to_string();
    let path = app
        .put(
            &format!("/api/teaching/classes/{cid}/path"),
            json!({ "steps": [{ "kind": "exam", "examId": exam_id }] }),
            Some(&t),
        )
        .await;
    ok(&path, StatusCode::OK);
    let step = path.body["steps"][0]["id"].as_str().unwrap().to_string();
    ok(&app.post(&format!("/api/teaching/classes/{cid}/path/publish"), json!({}), Some(&t)).await, StatusCode::OK);
    ok(&app.post(&format!("/api/classes/{cid}/enroll"), json!({}), Some(&s)).await, StatusCode::OK);

    // Học sinh làm bài: chụp 2 ảnh cho câu 1, gõ câu 2
    let res = app.post(&format!("/api/classes/{cid}/steps/{step}/attempts"), json!({}), Some(&s)).await;
    ok(&res, StatusCode::CREATED);
    let attempt = res.body["attemptId"].as_str().unwrap().to_string();
    let p1 = photo(&app, &s, "trang1.png").await;
    let p2 = photo(&app, &s, "trang2.png").await;
    // Ảnh của người khác không nộp được
    let other = photo(&app, &t, "cua-gv.png").await;
    let res = app.put(&format!("/api/attempts/{attempt}/items/0"), json!({ "files": [other] }), Some(&s)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    ok(
        &app.put(&format!("/api/attempts/{attempt}/items/0"), json!({ "files": [p1, p2] }), Some(&s)).await,
        StatusCode::NO_CONTENT,
    );
    ok(
        &app.put(&format!("/api/attempts/{attempt}/items/1"), json!({ "text": "Hàm số đồng biến." }), Some(&s)).await,
        StatusCode::NO_CONTENT,
    );
    let view = app.get(&format!("/api/attempts/{attempt}"), Some(&s)).await;
    assert_eq!(view.body["items"][0]["files"].as_array().unwrap().len(), 2);
    assert!(view.body["items"][0]["files"][0]["file"]["url"].as_str().unwrap().contains("&sig="));
    // Chưa nộp thì chưa chấm được
    let grade = |pos: i32, body: Value, tok: String| {
        let app = &app;
        let attempt = attempt.clone();
        async move { app.put(&format!("/api/teaching/attempts/{attempt}/items/{pos}/grade"), body, Some(&tok)).await }
    };
    assert_eq!(grade(0, json!({ "marks": {} }), t.clone()).await.code(), "ATTEMPT_NOT_SUBMITTED");
    let res = app.post(&format!("/api/attempts/{attempt}/submit"), json!({}), Some(&s)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["pending"], 2);
    assert_eq!(res.body["score"], 0.0);
    // Đề "không cho xem đáp án": học sinh không thấy hướng dẫn chấm
    assert!(res.body["items"][0]["result"].is_null());

    // Danh sách bài chờ chấm của giáo viên
    let queue = app.get("/api/teaching/grading", Some(&t)).await;
    ok(&queue, StatusCode::OK);
    assert_eq!(queue.body.as_array().unwrap().len(), 1);
    assert_eq!(queue.body[0]["pending"], 2);
    assert_eq!(queue.body[0]["classTitle"], "Toán 9");
    let attempts = app.get(&format!("/api/teaching/exams/{exam_id}/attempts?classId={cid}"), Some(&t)).await;
    assert_eq!(attempts.body[0]["pending"], 2);

    // Học sinh / giáo viên khác không chấm được; bước lạ, điểm quá bước, câu không bước chấm thiếu điểm → lỗi
    assert_eq!(grade(0, json!({ "marks": {} }), s.clone()).await.status, StatusCode::FORBIDDEN);
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    assert_eq!(grade(0, json!({ "marks": {} }), t2.clone()).await.status, StatusCode::NOT_FOUND);
    let res = grade(0, json!({ "marks": { Uuid::now_v7().to_string(): 0.5 } }), t.clone()).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = grade(0, json!({ "marks": { steps[0].clone(): 0.75 } }), t.clone()).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(grade(1, json!({}), t.clone()).await.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Chấm câu 1: bước 1 đủ, bước 2 nửa điểm, vẽ nhận xét lên ảnh trang 1
    let drawn = photo(&app, &t, "trang1-da-cham.png").await;
    let res = grade(
        0,
        json!({
            "marks": { steps[0].clone(): 0.5, steps[1].clone(): 0.25 },
            "feedback": "Thiếu kết luận tập nghiệm.",
            "annotations": [{ "fileId": p1, "annotatedFileId": drawn }]
        }),
        t.clone(),
    )
    .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["items"][0]["grading"]["score"], 0.75);
    assert_eq!(res.body["items"][0]["files"][0]["annotated"]["id"], drawn.as_str());
    assert_eq!(res.body["score"], 0.75);
    assert_eq!(res.body["pending"], 1);
    // Ảnh vẽ không thuộc bài làm → lỗi
    let res =
        grade(0, json!({ "marks": {}, "annotations": [{ "fileId": drawn, "annotatedFileId": drawn }] }), t.clone())
            .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let before = notification_count(&app, &s).await;

    // Trợ giảng của lớp chấm câu 2 (không có bước chấm: nhập điểm cả câu) → học sinh nhận thông báo
    let (ta_id, ta, _) = teacher(&app, "trogiang", "toan").await;
    sqlx::query(
        "INSERT INTO class_members (class_id, user_id, role, status) VALUES ($1::uuid, $2, 'assistant', 'active')",
    )
    .bind(&cid)
    .bind(ta_id)
    .execute(&app.ctx.db)
    .await
    .unwrap();
    let res = grade(1, json!({ "score": 0.5, "feedback": "Cần giải thích vì sao." }), ta.clone()).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["pending"], 0);
    assert_eq!(res.body["score"], 1.25);
    assert_eq!(notification_count(&app, &s).await, before + 1);
    assert_eq!(app.get("/api/teaching/grading", Some(&t)).await.body.as_array().unwrap().len(), 0);

    // Học sinh xem: điểm từng câu, nhận xét, ảnh đã vẽ (dù đề không cho xem đáp án)
    let view = app.get(&format!("/api/attempts/{attempt}"), Some(&s)).await;
    assert_eq!(view.body["score"], 1.25);
    assert_eq!(view.body["canGrade"], false);
    assert_eq!(view.body["items"][0]["grading"]["feedback"], "Thiếu kết luận tập nghiệm.");
    assert_eq!(view.body["items"][0]["grading"]["marks"][steps[1].as_str()], 0.25);
    assert!(view.body["items"][0]["files"][0]["annotated"]["url"].is_string());
    assert!(view.body["items"][0]["result"].is_null());

    // Chấm lại câu 1: điểm cập nhật, không báo lần nữa
    ok(
        &grade(0, json!({ "marks": { steps[0].clone(): 0.5, steps[1].clone(): 0.5 } }), t.clone()).await,
        StatusCode::OK,
    );
    assert_eq!(app.get(&format!("/api/attempts/{attempt}"), Some(&s)).await.body["score"], 1.5);
    assert_eq!(notification_count(&app, &s).await, before + 1);
    // "Chấm lại" câu tự luận từ ngân hàng không xóa điểm đã chấm
    let res = app
        .post(&format!("/api/teaching/questions/{}/regrade", essay["id"].as_str().unwrap()), json!({}), Some(&t))
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(app.get(&format!("/api/attempts/{attempt}"), Some(&s)).await.body["score"], 1.5);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn nhan_xet_mau(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, _) = teacher(&app, "gv", "toan").await;
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    let (_, s) = app.login_as("hs", UserRole::Student).await;
    let res = app.post("/api/teaching/comment-snippets", json!({ "body": "  Trình bày rõ ràng.  " }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["body"], "Trình bày rõ ràng.");
    let id = res.body["id"].as_str().unwrap().to_string();
    // Trùng, rỗng → lỗi; học sinh không dùng được
    let dup = app.post("/api/teaching/comment-snippets", json!({ "body": "Trình bày rõ ràng." }), Some(&t)).await;
    assert_eq!(dup.status, StatusCode::UNPROCESSABLE_ENTITY);
    let empty = app.post("/api/teaching/comment-snippets", json!({ "body": "   " }), Some(&t)).await;
    assert_eq!(empty.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.get("/api/teaching/comment-snippets", Some(&s)).await.status, StatusCode::FORBIDDEN);
    app.post("/api/teaching/comment-snippets", json!({ "body": "Thiếu điều kiện xác định." }), Some(&t)).await;
    assert_eq!(app.get("/api/teaching/comment-snippets", Some(&t)).await.body.as_array().unwrap().len(), 2);
    // Của ai người nấy thấy / xóa
    assert_eq!(app.get("/api/teaching/comment-snippets", Some(&t2)).await.body.as_array().unwrap().len(), 0);
    assert_eq!(
        app.delete(&format!("/api/teaching/comment-snippets/{id}"), Some(&t2)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.delete(&format!("/api/teaching/comment-snippets/{id}"), Some(&t)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(app.get("/api/teaching/comment-snippets", Some(&t)).await.body.as_array().unwrap().len(), 1);
}
