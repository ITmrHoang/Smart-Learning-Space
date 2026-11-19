//! Đợt 9: trộn mã đề (M40), bộ đề + giao nhanh (M41).

mod common;

use axum::http::StatusCode;
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;
use uuid::Uuid;

async fn mcq(app: &TestApp, t: &str, subject: Uuid, n: usize) -> String {
    let res = app
        .post(
            "/api/teaching/questions",
            json!({
                "subjectId": subject, "kind": "single", "level": "nhan_biet", "content": format!("Câu hỏi {n}"),
                "options": [{ "content": "Đúng", "correct": true }, { "content": "Sai 1" }, { "content": "Sai 2" },
                            { "content": "Sai 3" }]
            }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

/// Đề 2 phần: 6 câu một đáp án (trộn) + 1 câu tự luận. Trả về id đề.
async fn exam(app: &TestApp, t: &str, subject: Uuid) -> String {
    let mut ids = Vec::new();
    for i in 0..6 {
        ids.push(mcq(app, t, subject, i).await);
    }
    let essay = app
        .post(
            "/api/teaching/questions",
            json!({ "subjectId": subject, "kind": "essay", "level": "van_dung", "content": "Chứng minh",
                    "rubric": [{ "content": "Lập luận", "points": 1.5 }] }),
            Some(t),
        )
        .await;
    ok(&essay, StatusCode::CREATED);
    let res = app
        .post(
            "/api/teaching/exams",
            json!({
                "subjectId": subject, "title": "Kiểm tra giữa kỳ", "kind": "test", "showAnswers": "after_close",
                "timeLimitMinutes": 45,
                "sections": [
                    { "title": "PHẦN I", "scoring": "per_question", "pointsPerQuestion": 0.25, "questionIds": ids },
                    { "title": "PHẦN II. Tự luận", "scoring": "per_question", "pointsPerQuestion": 1,
                      "questionIds": [essay.body["id"]] }
                ]
            }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

/// Thứ tự câu của một mã đề (theo nội dung) + vị trí đáp án đúng của từng câu một đáp án.
fn layout(paper: &Value) -> (Vec<String>, Vec<usize>) {
    let items = paper["items"].as_array().unwrap();
    let order = items.iter().map(|i| i["content"].as_str().unwrap().to_string()).collect();
    let keys = items
        .iter()
        .filter(|i| i["kind"] == "single")
        .map(|i| i["options"].as_array().unwrap().iter().position(|o| o["correct"] == true).unwrap())
        .collect();
    (order, keys)
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tron_ma_de(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let exam_id = exam(&app, &t, toan).await;
    let url = format!("/api/teaching/exams/{exam_id}/papers");

    // Chưa tạo: danh sách rỗng
    let res = app.get(&url, Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["papers"].as_array().unwrap().len(), 0);

    let res = app.post(&url, json!({ "count": 4 }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    let papers = res.body["papers"].as_array().unwrap().clone();
    let codes: Vec<&str> = papers.iter().map(|p| p["code"].as_str().unwrap()).collect();
    assert_eq!(codes, ["101", "102", "103", "104"]);
    assert_eq!(res.body["maxScore"], 3.0); // 6 × 0,25 + tự luận 1,5 (tổng bước chấm)
    assert_eq!(res.body["timeLimitMinutes"], 45);
    for p in &papers {
        let items = p["items"].as_array().unwrap();
        assert_eq!(items.len(), 7);
        // Đánh số lại từ 1 ở mỗi phần; câu tự luận luôn ở phần II
        assert_eq!(items[5]["number"], 6);
        assert_eq!(items[6]["number"], 1);
        assert_eq!(items[6]["kind"], "essay");
        assert_eq!(items[6]["points"], 1.5);
        assert_eq!(items[6]["rubric"][0]["content"], "Lập luận");
        // Mỗi câu một đáp án có đúng 1 phương án đúng
        for it in &items[..6] {
            assert_eq!(it["options"].as_array().unwrap().iter().filter(|o| o["correct"] == true).count(), 1);
        }
    }
    // Các mã đề trộn khác nhau (thứ tự câu hoặc vị trí đáp án)
    let layouts: Vec<_> = papers.iter().map(layout).collect();
    assert!(layouts.iter().skip(1).any(|l| *l != layouts[0]), "các mã đề phải được trộn khác nhau");
    // Cùng một bộ câu hỏi ở mọi mã
    let mut first: Vec<String> = layouts[0].0.clone();
    first.sort();
    for l in &layouts {
        let mut o = l.0.clone();
        o.sort();
        assert_eq!(o, first);
    }

    // Xem lại: giữ nguyên thứ tự đã trộn
    let again = app.get(&url, Some(&t)).await;
    assert_eq!(again.body["papers"][2]["items"], papers[2]["items"]);

    // Tạo lại 2 mã từ 201: thay bộ cũ
    let res = app.post(&url, json!({ "count": 2, "firstCode": 201 }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    let codes: Vec<&str> = res.body["papers"].as_array().unwrap().iter().map(|p| p["code"].as_str().unwrap()).collect();
    assert_eq!(codes, ["201", "202"]);
    assert_eq!(app.get(&url, Some(&t)).await.body["papers"].as_array().unwrap().len(), 2);

    // Số mã sai; giáo viên khác / học sinh không xem được
    assert_eq!(app.post(&url, json!({ "count": 0 }), Some(&t)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.post(&url, json!({ "count": 25 }), Some(&t)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    assert_eq!(app.get(&url, Some(&t2)).await.status, StatusCode::NOT_FOUND);
    let (_, s) = app.login_as("hs", UserRole::Student).await;
    assert_eq!(app.get(&url, Some(&s)).await.status, StatusCode::FORBIDDEN);
}

/// Đề 1 câu một đáp án.
async fn small_exam(app: &TestApp, t: &str, subject: Uuid, title: &str) -> String {
    let q = mcq(app, t, subject, 99).await;
    let res = app
        .post(
            "/api/teaching/exams",
            json!({ "subjectId": subject, "title": title, "kind": "test", "showAnswers": "after_submit",
                    "sections": [{ "title": "I", "scoring": "per_question", "pointsPerQuestion": 10, "questionIds": [q] }] }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

async fn notification_count(app: &TestApp, token: &str) -> usize {
    app.get("/api/notifications", Some(token)).await.body["items"].as_array().unwrap().len()
}

async fn learn_steps(app: &TestApp, class: &str, token: &str) -> Vec<Value> {
    app.get(&format!("/api/classes/{class}/learn"), Some(token)).await.body["steps"].as_array().unwrap().clone()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn bo_de(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let e1 = small_exam(&app, &t, toan, "Đề số 1").await;
    let e2 = small_exam(&app, &t, toan, "Đề số 2").await;
    let res = app
        .post(
            "/api/teaching/exam-sets",
            json!({ "subjectId": toan, "title": "20 đề ôn thi vào 10", "examIds": [e2, e1, e2] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let titles: Vec<&str> =
        res.body["exams"].as_array().unwrap().iter().map(|e| e["title"].as_str().unwrap()).collect();
    assert_eq!(titles, ["Đề số 2", "Đề số 1"], "bỏ trùng, giữ thứ tự");
    let id = res.body["id"].as_str().unwrap().to_string();
    let list = app.get("/api/teaching/exam-sets", Some(&t)).await;
    assert_eq!(list.body[0]["examCount"], 2);

    // Đổi thứ tự; đề bị ẩn thì không hiện trong bộ
    let res = app
        .put(
            &format!("/api/teaching/exam-sets/{id}"),
            json!({ "subjectId": toan, "title": "20 đề ôn thi vào 10", "examIds": [e1, e2] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["exams"][0]["title"], "Đề số 1");
    ok(&app.delete(&format!("/api/teaching/exams/{e2}"), Some(&t)).await, StatusCode::NO_CONTENT);
    let got = app.get(&format!("/api/teaching/exam-sets/{id}"), Some(&t)).await;
    assert_eq!(got.body["exams"].as_array().unwrap().len(), 1);

    // Đề của người khác → lỗi; người khác không xem được bộ đề
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    let other = small_exam(&app, &t2, toan, "Đề của gv2").await;
    let res = app
        .post("/api/teaching/exam-sets", json!({ "subjectId": toan, "title": "Bộ", "examIds": [other] }), Some(&t))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.get(&format!("/api/teaching/exam-sets/{id}"), Some(&t2)).await.status, StatusCode::NOT_FOUND);
    ok(&app.delete(&format!("/api/teaching/exam-sets/{id}"), Some(&t)).await, StatusCode::NO_CONTENT);
    assert_eq!(app.get("/api/teaching/exam-sets", Some(&t)).await.body.as_array().unwrap().len(), 0);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn giao_nhanh(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (an_id, an) = app.login_as("an", UserRole::Student).await;
    let (_, binh) = app.login_as("binh", UserRole::Student).await;
    let class = app
        .post(
            "/api/teaching/classes",
            json!({ "subjectId": toan, "title": "Toán 9A", "mode": "self_paced", "visibility": "public",
                    "enrollment": "open", "gradeIds": [], "tagIds": [] }),
            Some(&t),
        )
        .await;
    let cid = class.body["id"].as_str().unwrap().to_string();
    for s in [&an, &binh] {
        ok(&app.post(&format!("/api/classes/{cid}/enroll"), json!({}), Some(s)).await, StatusCode::OK);
    }
    let group = app.post("/api/teaching/groups", json!({ "name": "Nhóm khá", "memberIds": [an_id] }), Some(&t)).await;
    ok(&group, StatusCode::CREATED);
    let gid = group.body["id"].as_str().unwrap().to_string();

    // Bản nháp đang sửa dở (chưa xuất bản) không được xuất bản theo khi giao nhanh
    let draft_exam = small_exam(&app, &t, toan, "Đề đang soạn").await;
    let res = app
        .put(
            &format!("/api/teaching/classes/{cid}/path"),
            json!({ "steps": [{ "kind": "exam", "examId": draft_exam }] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);

    let ex = small_exam(&app, &t, toan, "Kiểm tra 15 phút").await;
    let due = (chrono::Utc::now() + chrono::Duration::days(1)).to_rfc3339();
    let url = format!("/api/teaching/classes/{cid}/quick-assign");
    let before = notification_count(&app, &an).await;
    let res = app.post(&url, json!({ "kind": "exam", "examId": ex, "dueAt": due, "groupIds": [gid] }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["version"], 1);
    assert_eq!(res.body["assigned"], 1);
    let step = res.body["stepId"].as_str().unwrap().to_string();
    assert_eq!(notification_count(&app, &an).await, before + 1);
    let notes = app.get("/api/notifications", Some(&an)).await.body["items"].clone();
    assert_eq!(notes[0]["title"], "Bài mới: Kiểm tra 15 phút");

    // An (trong nhóm) thấy đúng bước được giao; Bình không thấy; bước nháp chưa xuất bản không hiện
    let steps = learn_steps(&app, &cid, &an).await;
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["id"], step.as_str());
    assert_eq!(learn_steps(&app, &cid, &binh).await.len(), 0);
    let draft = app.get(&format!("/api/teaching/classes/{cid}/path"), Some(&t)).await;
    assert_eq!(draft.body["steps"].as_array().unwrap().len(), 2, "bản nháp có cả bước đang soạn lẫn bước mới");

    // An làm bài: giờ nộp của lượt làm không quá hạn làm
    let res = app.post(&format!("/api/classes/{cid}/steps/{step}/attempts"), json!({}), Some(&an)).await;
    ok(&res, StatusCode::CREATED);
    let attempt = res.body["attemptId"].as_str().unwrap().to_string();
    let view = app.get(&format!("/api/attempts/{attempt}"), Some(&an)).await;
    let deadline = chrono::DateTime::parse_from_rfc3339(view.body["deadlineAt"].as_str().unwrap()).unwrap();
    assert!(deadline <= chrono::DateTime::parse_from_rfc3339(&due).unwrap());
    ok(&app.post(&format!("/api/attempts/{attempt}/submit"), json!({}), Some(&an)).await, StatusCode::OK);

    // Quá hạn: không bắt đầu lượt mới
    let past = chrono::Utc::now() - chrono::Duration::minutes(5);
    sqlx::query(
        "UPDATE path_versions SET steps = (SELECT jsonb_agg(CASE WHEN s->>'id' = $1 \
             THEN jsonb_set(s, ARRAY['dueAt'], to_jsonb($2::timestamptz)) ELSE s END) \
             FROM jsonb_array_elements(steps) s) \
         WHERE class_id = $3::uuid",
    )
    .bind(&step)
    .bind(past)
    .bind(&cid)
    .execute(&app.ctx.db)
    .await
    .unwrap();
    let res = app.post(&format!("/api/classes/{cid}/steps/{step}/attempts"), json!({}), Some(&an)).await;
    assert_eq!(res.code(), "STEP_OVERDUE");

    // Giao bài tập cho cả lớp (không hạn) → phiên bản 2, cả 2 học sinh
    let hw = app
        .post(
            "/api/teaching/assignments",
            json!({ "subjectId": toan, "title": "Bài về nhà số 3", "instructions": "Làm bài 1, 2", "maxScore": 10 }),
            Some(&t),
        )
        .await;
    ok(&hw, StatusCode::CREATED);
    let res = app.post(&url, json!({ "kind": "homework", "assignmentId": hw.body["id"] }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["version"], 2);
    assert_eq!(res.body["assigned"], 2);
    assert_eq!(learn_steps(&app, &cid, &binh).await.len(), 1);

    // Lỗi: hạn trong quá khứ, loại bước khác, học sinh ngoài lớp, đề của người khác; giáo viên khác 404
    let res = app.post(&url, json!({ "kind": "exam", "examId": ex, "dueAt": past.to_rfc3339() }), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.post(&url, json!({ "kind": "lesson" }), Some(&t)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let (out_id, _) = app.login_as("ngoai", UserRole::Student).await;
    let res = app.post(&url, json!({ "kind": "exam", "examId": ex, "userIds": [out_id] }), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    let other = small_exam(&app, &t2, toan, "Đề gv2").await;
    let res = app.post(&url, json!({ "kind": "exam", "examId": other }), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.post(&url, json!({ "kind": "exam", "examId": ex }), Some(&t2)).await.status, StatusCode::NOT_FOUND);
}
