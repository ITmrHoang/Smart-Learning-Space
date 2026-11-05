//! M10: ngân hàng câu hỏi, thư mục, import Excel.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub fn single(subject: Uuid, content: &str, correct: usize) -> Value {
    json!({
        "subjectId": subject, "kind": "single", "level": "nhan_biet", "content": content,
        "options": (0..4).map(|i| json!({ "content": format!("Phương án {i}"), "correct": i == correct })).collect::<Vec<_>>(),
        "explanation": "Lời giải"
    })
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tao_cau_hoi_cac_dang_va_kiem_tra_cau_truc(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;

    let res = app.post("/api/teaching/questions", single(toan, "Giá trị $\\sqrt{16}$", 1), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["version"], 1);
    assert_eq!(res.body["options"][1]["correct"], true);

    let tf = json!({
        "subjectId": toan, "kind": "true_false", "level": "van_dung", "content": "Cho $y = 2x + 1$",
        "options": [
            { "content": "a) qua (0;1)", "correct": true }, { "content": "b) nghịch biến", "correct": false },
            { "content": "c) hệ số góc 2", "correct": true }, { "content": "d) cắt Ox tại 1", "correct": false }
        ]
    });
    ok(&app.post("/api/teaching/questions", tf, Some(&t)).await, StatusCode::CREATED);
    let short = json!({ "subjectId": toan, "kind": "short", "level": "thong_hieu", "content": "Nghiệm của $2x-1=0$", "accepted": ["0,5", " "] });
    let res = app.post("/api/teaching/questions", short, Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["accepted"], json!(["0,5"]));

    // Lỗi cấu trúc theo dạng câu.
    let mut two = single(toan, "x", 0);
    two["options"][1]["correct"] = json!(true);
    assert_eq!(app.post("/api/teaching/questions", two, Some(&t)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    let mut one_option = single(toan, "x", 0);
    one_option["options"] = json!([{ "content": "A", "correct": true }]);
    assert_eq!(
        app.post("/api/teaching/questions", one_option, Some(&t)).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let no_answer =
        json!({ "subjectId": toan, "kind": "short", "level": "thong_hieu", "content": "x", "accepted": [] });
    let res = app.post("/api/teaching/questions", no_answer, Some(&t)).await;
    assert!(res.body["error"]["fields"]["accepted"].is_array(), "{}", res.body);

    // Danh sách: lọc theo dạng, tìm không dấu trong nội dung.
    assert_eq!(app.get("/api/teaching/questions?kind=true_false", Some(&t)).await.body["total"], 1);
    assert_eq!(app.get("/api/teaching/questions?q=nghiem", Some(&t)).await.body["total"], 1);
    assert_eq!(app.get("/api/teaching/questions", Some(&t)).await.body["total"], 3);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn sua_cau_hoi_tao_phien_ban_giu_id_phuong_an(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let q = app.post("/api/teaching/questions", single(toan, "Câu 1", 0), Some(&t)).await.body;
    let id = q["id"].as_str().unwrap();
    let opts = q["options"].as_array().unwrap().clone();

    // Sửa chữ (giữ ID) → phiên bản 2, đáp án không đổi.
    let mut body = single(toan, "Câu 1 (sửa)", 0);
    for (i, o) in opts.iter().enumerate() {
        body["options"][i]["id"] = o["id"].clone();
    }
    let res = app.put(&format!("/api/teaching/questions/{id}"), body.clone(), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["question"]["version"], 2);
    assert_eq!(res.body["answerChanged"], false);
    assert_eq!(res.body["question"]["options"][0]["id"], opts[0]["id"]);

    // Đổi đáp án đúng → báo đổi đáp án (chưa có bài nộp nên 0 bài ảnh hưởng).
    body["options"][0]["correct"] = json!(false);
    body["options"][2]["correct"] = json!(true);
    let res = app.put(&format!("/api/teaching/questions/{id}"), body, Some(&t)).await;
    assert_eq!(res.body["answerChanged"], true);
    assert_eq!(res.body["affectedAttempts"], 0);

    // Giáo viên khác không thấy.
    let (_, other, _) = teacher(&app, "gv2", "toan").await;
    assert_eq!(app.get(&format!("/api/teaching/questions/{id}"), Some(&other)).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn thu_muc_cau_hoi(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let ch1 = app.post("/api/teaching/question-folders", json!({ "name": "Chương 1" }), Some(&t)).await;
    ok(&ch1, StatusCode::CREATED);
    let ch1 = ch1.body["id"].as_str().unwrap().to_string();
    let b1 = app.post("/api/teaching/question-folders", json!({ "name": "Bài 1", "parentId": ch1 }), Some(&t)).await;
    let b1 = b1.body["id"].as_str().unwrap().to_string();

    let mut q = single(toan, "Trong thư mục", 0);
    q["folderId"] = json!(b1);
    ok(&app.post("/api/teaching/questions", q, Some(&t)).await, StatusCode::CREATED);
    ok(&app.post("/api/teaching/questions", single(toan, "Ngoài", 0), Some(&t)).await, StatusCode::CREATED);
    assert_eq!(app.get(&format!("/api/teaching/questions?folderId={b1}"), Some(&t)).await.body["total"], 1);
    assert_eq!(app.get("/api/teaching/questions?folderId=none", Some(&t)).await.body["total"], 1);

    // Không chuyển thư mục vào con của nó.
    let res = app
        .put(&format!("/api/teaching/question-folders/{ch1}"), json!({ "name": "Chương 1", "parentId": b1 }), Some(&t))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    // Xóa thư mục → câu hỏi về "chưa xếp thư mục".
    ok(&app.delete(&format!("/api/teaching/question-folders/{ch1}"), Some(&t)).await, StatusCode::NO_CONTENT);
    assert_eq!(app.get("/api/teaching/questions?folderId=none", Some(&t)).await.body["total"], 2);
    // Thư mục của người khác.
    let (_, other, _) = teacher(&app, "gv2", "toan").await;
    let mut q = single(toan, "x", 0);
    q["folderId"] = json!(b1);
    assert_eq!(app.post("/api/teaching/questions", q, Some(&other)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn import_excel_xem_truoc_roi_luu(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;

    // Tải file mẫu.
    let req = Request::builder()
        .uri("/api/teaching/questions/import/template")
        .header(header::AUTHORIZATION, format!("Bearer {t}"))
        .body(Body::empty())
        .unwrap();
    let res = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.headers()[header::CONTENT_DISPOSITION].to_str().unwrap().contains("mau-cau-hoi.xlsx"));
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();

    // Gửi lại chính file mẫu để xem trước.
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/teaching/questions/import/preview?subjectId={toan}"))
        .header(header::AUTHORIZATION, format!("Bearer {t}"))
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from(bytes.clone()))
        .unwrap();
    let res = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let preview: Value =
        serde_json::from_slice(&axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(preview["validCount"], 4, "{preview}");
    let items = preview["items"].as_array().unwrap();
    assert_eq!(items[1]["kind"], "multiple");
    assert_eq!(items[2]["options"].as_array().unwrap().len(), 4);

    // File hỏng → lỗi rõ ràng.
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/teaching/questions/import/preview?subjectId={toan}"))
        .header(header::AUTHORIZATION, format!("Bearer {t}"))
        .body(Body::from("không phải excel"))
        .unwrap();
    let res = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Lưu hàng loạt các dòng hợp lệ.
    let to_input = |i: &Value| {
        json!({
            "subjectId": toan, "kind": i["kind"], "level": i["level"], "content": i["content"],
            "options": i["options"], "accepted": i["accepted"], "explanation": i["explanation"], "tagIds": i["tagIds"]
        })
    };
    let body = json!({ "items": items.iter().map(to_input).collect::<Vec<_>>() });
    let res = app.post("/api/teaching/questions/bulk", body, Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["created"], 4);
    assert_eq!(app.get("/api/teaching/questions", Some(&t)).await.body["total"], 4);

    // Một câu lỗi → không lưu câu nào, báo đúng thứ tự câu.
    let mut bad = items.iter().map(to_input).collect::<Vec<_>>();
    bad[2]["content"] = json!("");
    let res = app.post("/api/teaching/questions/bulk", json!({ "items": bad }), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.get("/api/teaching/questions", Some(&t)).await.body["total"], 4);
}
