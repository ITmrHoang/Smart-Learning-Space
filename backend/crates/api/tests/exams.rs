//! M11: đề, ma trận, trộn câu / phương án, chấm THPT, ôn tập kiểm tra từng câu, chấm lại.

mod common;

use axum::http::StatusCode;
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

fn single(subject: Uuid, content: &str, correct: usize, level: &str, tag: Option<&str>) -> Value {
    json!({
        "subjectId": subject, "kind": "single", "level": level, "content": content,
        "tagIds": tag.map(|t| vec![t]).unwrap_or_default(),
        "options": (0..4).map(|i| json!({ "content": format!("{content} — PA {i}"), "correct": i == correct })).collect::<Vec<_>>(),
        "explanation": format!("Lời giải {content}")
    })
}

async fn create_q(app: &TestApp, t: &str, body: Value) -> Value {
    let res = app.post("/api/teaching/questions", body, Some(t)).await;
    ok(&res, StatusCode::CREATED);
    res.body
}

/// Đề THPT thu nhỏ: phần I 2 câu một đáp án (0,25), phần II 1 câu đúng/sai (1 điểm, thang THPT),
/// phần III 1 câu trả lời ngắn (0,5). Trả về (exam, [q1, q2, tf, short]).
async fn thpt_exam(app: &TestApp, t: &str, subject: Uuid, kind: &str) -> (Value, Vec<Value>) {
    let q1 = create_q(app, t, single(subject, "Câu 1", 0, "nhan_biet", None)).await;
    let q2 = create_q(app, t, single(subject, "Câu 2", 3, "nhan_biet", None)).await;
    let tf = create_q(
        app,
        t,
        json!({
            "subjectId": subject, "kind": "true_false", "level": "van_dung", "content": "Đúng sai",
            "options": [
                { "content": "a", "correct": true }, { "content": "b", "correct": false },
                { "content": "c", "correct": true }, { "content": "d", "correct": false }
            ]
        }),
    )
    .await;
    let short = create_q(
        app,
        t,
        json!({ "subjectId": subject, "kind": "short", "level": "thong_hieu", "content": "Ngắn", "accepted": ["0,5"] }),
    )
    .await;
    let exam = json!({
        "subjectId": subject, "title": "Đề THPT", "kind": kind, "showAnswers": "after_submit",
        "sections": [
            { "title": "Phần I", "scoring": "per_question", "pointsPerQuestion": 0.25, "questionIds": [q1["id"], q2["id"]] },
            { "title": "Phần II", "scoring": "thpt_true_false", "pointsPerQuestion": 1, "questionIds": [tf["id"]] },
            { "title": "Phần III", "scoring": "per_question", "pointsPerQuestion": 0.5, "questionIds": [short["id"]] }
        ]
    });
    let res = app.post("/api/teaching/exams", exam, Some(t)).await;
    ok(&res, StatusCode::CREATED);
    (res.body, vec![q1, q2, tf, short])
}

fn item_by_content<'a>(view: &'a Value, content: &str) -> &'a Value {
    view["items"].as_array().unwrap().iter().find(|i| i["content"] == content).expect("có câu")
}

/// ID phương án có nội dung kết thúc bằng `PA {n}` (không phụ thuộc thứ tự đã trộn).
fn option_id(item: &Value, n: usize) -> Value {
    item["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["content"].as_str().unwrap().ends_with(&format!("PA {n}")))
        .unwrap()["id"]
        .clone()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tao_de_tinh_diem_toi_da_va_kiem_tra(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (exam, _) = thpt_exam(&app, &t, toan, "test").await;
    assert_eq!(exam["questionCount"], 4);
    assert_eq!(exam["maxScore"], 2.0);
    assert_eq!(exam["sections"][1]["scoring"], "thpt_true_false");
    assert_eq!(exam["warnings"], json!([]));

    // Câu của giáo viên khác / phần rỗng / câu trùng → 422.
    let (_, other, _) = teacher(&app, "gv2", "toan").await;
    let foreign = create_q(&app, &other, single(toan, "Của người khác", 0, "nhan_biet", None)).await;
    let bad = json!({
        "subjectId": toan, "title": "Đề", "kind": "test", "showAnswers": "after_submit",
        "sections": [{ "title": "I", "scoring": "per_question", "pointsPerQuestion": 1, "questionIds": [foreign["id"]] }]
    });
    assert_eq!(app.post("/api/teaching/exams", bad, Some(&t)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let empty = json!({
        "subjectId": toan, "title": "Đề", "kind": "test", "showAnswers": "after_submit",
        "sections": [{ "title": "I", "scoring": "per_question", "pointsPerQuestion": 1 }]
    });
    assert_eq!(app.post("/api/teaching/exams", empty, Some(&t)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let q = exam["sections"][0]["questions"][0]["id"].clone();
    let dup = json!({
        "subjectId": toan, "title": "Đề", "kind": "test", "showAnswers": "after_submit",
        "sections": [
            { "title": "I", "scoring": "per_question", "pointsPerQuestion": 1, "questionIds": [q] },
            { "title": "II", "scoring": "per_question", "pointsPerQuestion": 1, "questionIds": [q] }
        ]
    });
    assert_eq!(app.post("/api/teaching/exams", dup, Some(&t)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn lam_bai_khong_lo_dap_an_cham_diem_thpt(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (exam, _) = thpt_exam(&app, &t, toan, "test").await;
    let exam_id = exam["id"].as_str().unwrap();

    let view = app.post(&format!("/api/teaching/exams/{exam_id}/preview"), json!({}), Some(&t)).await;
    ok(&view, StatusCode::OK);
    let attempt = view.body["id"].as_str().unwrap().to_string();
    assert_eq!(view.body["status"], "in_progress");
    assert_eq!(view.body["items"].as_array().unwrap().len(), 4);
    // Trước khi nộp: không có đáp án, lời giải, cờ đúng.
    let text = view.body.to_string();
    assert!(!text.contains("Lời giải") && !text.contains("\"correct\"") && !text.contains("accepted"), "{text}");
    assert!(view.body["items"].as_array().unwrap().iter().all(|i| i["result"].is_null()));
    // Phần giữ đúng thứ tự I → II → III dù trộn câu trong phần.
    let sections: Vec<i64> =
        view.body["items"].as_array().unwrap().iter().map(|i| i["sectionPosition"].as_i64().unwrap()).collect();
    assert_eq!(sections, vec![0, 0, 1, 2]);

    // Trả lời: câu 1 đúng, câu 2 sai, đúng/sai đúng 3 ý, trả lời ngắn "1/2".
    let b = &view.body;
    let c1 = item_by_content(b, "Câu 1");
    let c2 = item_by_content(b, "Câu 2");
    let tf = item_by_content(b, "Đúng sai");
    let short = item_by_content(b, "Ngắn");
    let put = |pos: &Value, body: Value| {
        let path = format!("/api/attempts/{attempt}/items/{pos}");
        let app = &app;
        let t = t.clone();
        async move { app.put(&path, body, Some(&t)).await }
    };
    ok(&put(&c1["position"], json!({ "selected": [option_id(c1, 0)] })).await, StatusCode::NO_CONTENT);
    ok(&put(&c2["position"], json!({ "selected": [option_id(c2, 0)] })).await, StatusCode::NO_CONTENT);
    let opts = tf["options"].as_array().unwrap();
    let truth: serde_json::Map<String, Value> = opts
        .iter()
        .map(|o| {
            (o["id"].as_str().unwrap().to_string(), json!(matches!(o["content"].as_str().unwrap(), "a" | "c" | "d")))
        })
        .collect();
    ok(&put(&tf["position"], json!({ "truth": truth })).await, StatusCode::NO_CONTENT);
    ok(&put(&short["position"], json!({ "text": "1/2" })).await, StatusCode::NO_CONTENT);
    // Chọn 2 phương án cho câu một đáp án / ID lạ → 400.
    let both = json!({ "selected": [option_id(c1, 0), option_id(c1, 1)] });
    assert_eq!(put(&c1["position"], both).await.code(), "INVALID_ANSWER");
    assert_eq!(put(&c1["position"], json!({ "selected": [Uuid::now_v7()] })).await.code(), "INVALID_ANSWER");
    // Câu ôn tập "Kiểm tra" chỉ dùng cho đề ôn tập.
    let res = app.post(&format!("/api/attempts/{attempt}/items/0/check"), json!({}), Some(&t)).await;
    assert_eq!(res.code(), "NOT_PRACTICE");

    let res = app.post(&format!("/api/attempts/{attempt}/submit"), json!({}), Some(&t)).await;
    ok(&res, StatusCode::OK);
    // 0,25 + 0 + 0,5 (3/4 ý) + 0,5 = 1,25 / 2.
    assert_eq!(res.body["score"], 1.25);
    assert_eq!(res.body["maxScore"], 2.0);
    assert_eq!(res.body["reveal"], true);
    let tf_after = item_by_content(&res.body, "Đúng sai");
    assert_eq!(tf_after["result"]["score"], 0.5);
    assert_eq!(item_by_content(&res.body, "Câu 1")["result"]["explanation"], "Lời giải Câu 1");
    // Nộp rồi thì không sửa / nộp lại được.
    assert_eq!(put(&c1["position"], json!({ "selected": [] })).await.code(), "ATTEMPT_CLOSED");
    assert_eq!(
        app.post(&format!("/api/attempts/{attempt}/submit"), json!({}), Some(&t)).await.code(),
        "ATTEMPT_CLOSED"
    );
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tron_cau_va_phuong_an_moi_luot_khac_nhau(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let mut ids = Vec::new();
    for i in 0..8 {
        ids.push(create_q(&app, &t, single(toan, &format!("Q{i}"), 0, "nhan_biet", None)).await["id"].clone());
    }
    let exam = json!({
        "subjectId": toan, "title": "Trộn", "kind": "practice", "showAnswers": "after_submit",
        "sections": [{ "title": "I", "scoring": "per_question", "pointsPerQuestion": 1, "questionIds": ids }]
    });
    let exam_id = app.post("/api/teaching/exams", exam, Some(&t)).await.body["id"].as_str().unwrap().to_string();

    let order = |v: &Value| -> Vec<String> {
        v["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                format!(
                    "{}|{}",
                    i["content"],
                    i["options"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|o| o["content"].as_str().unwrap().to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect()
    };
    let first = app.post(&format!("/api/teaching/exams/{exam_id}/preview"), json!({}), Some(&t)).await.body;
    // Đang làm dở → bấm lại vẫn ra đúng lượt đó.
    let again = app.post(&format!("/api/teaching/exams/{exam_id}/preview"), json!({}), Some(&t)).await.body;
    assert_eq!(first["id"], again["id"]);
    app.post(&format!("/api/attempts/{}/submit", first["id"].as_str().unwrap()), json!({}), Some(&t)).await;
    let second = app.post(&format!("/api/teaching/exams/{exam_id}/preview"), json!({}), Some(&t)).await.body;
    assert_ne!(first["id"], second["id"]);
    // 8 câu × 4 phương án: xác suất hai lượt trùng hoàn toàn gần như bằng 0.
    assert_ne!(order(&first), order(&second));
    let mut a = order(&first);
    let mut b = order(&second);
    a.sort();
    b.sort();
    assert_eq!(a.len(), b.len());
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn ma_tran_rut_theo_danh_muc_va_muc_do(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let tag = app.post(&format!("/api/subjects/{toan}/tags"), json!({ "name": "Hàm số" }), Some(&t)).await;
    let tag = tag.body["id"].as_str().unwrap().to_string();
    for i in 0..3 {
        create_q(&app, &t, single(toan, &format!("HS-NB{i}"), 0, "nhan_biet", Some(&tag))).await;
    }
    for i in 0..2 {
        create_q(&app, &t, single(toan, &format!("HS-TH{i}"), 0, "thong_hieu", Some(&tag))).await;
    }
    create_q(&app, &t, single(toan, "Khác-NB", 0, "nhan_biet", None)).await;

    let exam = |nb: i32, th: i32| {
        json!({
            "subjectId": toan, "title": "Ma trận", "kind": "practice", "showAnswers": "after_submit",
            "sections": [{ "title": "I", "scoring": "per_question", "pointsPerQuestion": 0.5, "rules": [
                { "tagId": tag, "level": "nhan_biet", "count": nb },
                { "tagId": tag, "level": "thong_hieu", "count": th }
            ]}]
        })
    };
    // Ô thông hiểu cần 3 câu nhưng chỉ có 2 → vẫn lưu, kèm cảnh báo.
    let res = app.post("/api/teaching/exams", exam(2, 3), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["sections"][0]["rules"][1]["available"], 2);
    assert_eq!(res.body["warnings"].as_array().unwrap().len(), 1);

    let res = app.post("/api/teaching/exams", exam(2, 1), Some(&t)).await;
    assert_eq!(res.body["warnings"], json!([]));
    assert_eq!(res.body["questionCount"], 3);
    assert_eq!(res.body["maxScore"], 1.5);
    let exam_id = res.body["id"].as_str().unwrap();
    let view = app.post(&format!("/api/teaching/exams/{exam_id}/preview"), json!({}), Some(&t)).await.body;
    let contents: Vec<&str> =
        view["items"].as_array().unwrap().iter().map(|i| i["content"].as_str().unwrap()).collect();
    assert_eq!(contents.len(), 3);
    assert_eq!(contents.iter().filter(|c| c.starts_with("HS-NB")).count(), 2);
    assert_eq!(contents.iter().filter(|c| c.starts_with("HS-TH")).count(), 1);
    assert!(!contents.contains(&"Khác-NB"));
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn on_tap_kiem_tra_tung_cau(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (exam, _) = thpt_exam(&app, &t, toan, "practice").await;
    let view = app
        .post(&format!("/api/teaching/exams/{}/preview", exam["id"].as_str().unwrap()), json!({}), Some(&t))
        .await
        .body;
    assert_eq!(view["canCheck"], true);
    let attempt = view["id"].as_str().unwrap();
    let c1 = item_by_content(&view, "Câu 1");
    let pos = c1["position"].clone();
    app.put(&format!("/api/attempts/{attempt}/items/{pos}"), json!({ "selected": [option_id(c1, 2)] }), Some(&t)).await;
    let res = app.post(&format!("/api/attempts/{attempt}/items/{pos}/check"), json!({}), Some(&t)).await;
    ok(&res, StatusCode::OK);
    let checked = item_by_content(&res.body, "Câu 1");
    assert_eq!(checked["checked"], true);
    assert_eq!(checked["result"]["isCorrect"], false);
    assert_eq!(checked["result"]["correct"], json!([option_id(c1, 0)]));
    // Câu chưa kiểm tra vẫn ẩn đáp án.
    assert!(item_by_content(&res.body, "Câu 2")["result"].is_null());
    // Đã kiểm tra thì khóa câu trả lời.
    let res = app
        .put(&format!("/api/attempts/{attempt}/items/{pos}"), json!({ "selected": [option_id(c1, 0)] }), Some(&t))
        .await;
    assert_eq!(res.code(), "ITEM_LOCKED");
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn sua_dap_an_roi_cham_lai(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (exam, qs) = thpt_exam(&app, &t, toan, "test").await;
    let view = app
        .post(&format!("/api/teaching/exams/{}/preview", exam["id"].as_str().unwrap()), json!({}), Some(&t))
        .await
        .body;
    let attempt = view["id"].as_str().unwrap();
    // Chọn PA 1 cho câu 1 (đáp án lưu sai là PA 0).
    let c1 = item_by_content(&view, "Câu 1");
    app.put(
        &format!("/api/attempts/{attempt}/items/{}", c1["position"]),
        json!({ "selected": [option_id(c1, 1)] }),
        Some(&t),
    )
    .await;
    let res = app.post(&format!("/api/attempts/{attempt}/submit"), json!({}), Some(&t)).await;
    assert_eq!(res.body["score"], 0.0);

    // Giáo viên sửa đáp án đúng của câu 1 thành PA 1 (giữ ID phương án).
    let q1 = &qs[0];
    let mut body = single(toan, "Câu 1", 1, "nhan_biet", None);
    for (i, o) in q1["options"].as_array().unwrap().iter().enumerate() {
        body["options"][i]["id"] = o["id"].clone();
    }
    let res = app.put(&format!("/api/teaching/questions/{}", q1["id"].as_str().unwrap()), body, Some(&t)).await;
    assert_eq!(res.body["answerChanged"], true);
    assert_eq!(res.body["affectedAttempts"], 1);
    let res =
        app.post(&format!("/api/teaching/questions/{}/regrade", q1["id"].as_str().unwrap()), json!({}), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body, json!({ "regradedAttempts": 1, "changedAttempts": 1 }));
    let after = app.get(&format!("/api/attempts/{attempt}"), Some(&t)).await;
    assert_eq!(after.body["score"], 0.25);
}
