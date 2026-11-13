//! Đợt 4: báo cáo lớp học + Excel (M17), hỏi đáp + báo vi phạm (M18), thẻ ghi nhớ (M19), phụ huynh + báo cáo (M20).

mod common;

use axum::http::{StatusCode, header};
use chrono::{DateTime, Utc};
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sls_core::modules::{guardians, users::model::UserRole};
use sqlx::PgPool;
use uuid::Uuid;

/// Lớp học tự học, công khai, tự vào; trả về id.
async fn class(app: &TestApp, t: &str, subject: Uuid, title: &str) -> String {
    let res = app
        .post(
            "/api/teaching/classes",
            json!({ "subjectId": subject, "title": title, "mode": "self_paced", "visibility": "public", "enrollment": "open" }),
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

async fn lesson(app: &TestApp, t: &str, subject: Uuid) -> String {
    let res = app
        .post(
            "/api/teaching/lessons",
            json!({ "subjectId": subject, "title": "Bài 1", "content": "Nội dung" }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let id = res.body["id"].as_str().unwrap().to_string();
    ok(&app.post(&format!("/api/teaching/lessons/{id}/publish"), json!({}), Some(t)).await, StatusCode::OK);
    id
}

/// Câu một đáp án (PA 0 đúng), có thể gắn danh mục. Trả về id câu.
async fn question(app: &TestApp, t: &str, subject: Uuid, tag: Option<&str>) -> String {
    let res = app
        .post(
            "/api/teaching/questions",
            json!({
                "subjectId": subject, "kind": "single", "level": "thong_hieu", "content": "Căn bậc hai của 16 là?",
                "tagIds": tag.map(|t| vec![t]).unwrap_or_default(),
                "options": [{ "content": "PA 0", "correct": true }, { "content": "PA 1" }],
                "explanation": "Vì $4^2 = 16$."
            }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

async fn exam(app: &TestApp, t: &str, subject: Uuid, question_id: &str) -> String {
    let res = app
        .post(
            "/api/teaching/exams",
            json!({
                "subjectId": subject, "title": "Kiểm tra căn bậc hai", "kind": "test", "showAnswers": "after_submit",
                "sections": [{ "title": "I", "scoring": "per_question", "pointsPerQuestion": 10, "questionIds": [question_id] }]
            }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

/// Lưu + xuất bản lộ trình; trả về id các bước.
async fn publish_path(app: &TestApp, t: &str, class_id: &str, steps: Value) -> Vec<String> {
    let res = app.put(&format!("/api/teaching/classes/{class_id}/path"), json!({ "steps": steps }), Some(t)).await;
    ok(&res, StatusCode::OK);
    let res = app.post(&format!("/api/teaching/classes/{class_id}/path/publish"), json!({}), Some(t)).await;
    ok(&res, StatusCode::OK);
    res.body["steps"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect()
}

/// Học sinh làm đề của bước, chọn PA `n`, nộp.
async fn take_exam(app: &TestApp, s: &str, class: &str, step: &str, n: usize) {
    let res = app.post(&format!("/api/classes/{class}/steps/{step}/attempts"), json!({}), Some(s)).await;
    ok(&res, StatusCode::CREATED);
    let attempt = res.body["attemptId"].as_str().unwrap().to_string();
    let view = app.get(&format!("/api/attempts/{attempt}"), Some(s)).await.body;
    let opt = view["items"][0]["options"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["content"] == format!("PA {n}"))
        .unwrap()["id"]
        .clone();
    ok(
        &app.put(&format!("/api/attempts/{attempt}/items/0"), json!({ "selected": [opt] }), Some(s)).await,
        StatusCode::NO_CONTENT,
    );
    ok(&app.post(&format!("/api/attempts/{attempt}/submit"), json!({}), Some(s)).await, StatusCode::OK);
}

async fn unread(app: &TestApp, token: &str) -> i64 {
    app.get("/api/notifications/unread-count", Some(token)).await.body["unread"].as_i64().unwrap()
}

async fn latest_kind(app: &TestApp, token: &str) -> String {
    app.get("/api/notifications", Some(token)).await.body["items"][0]["kind"].as_str().unwrap().to_string()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn bao_cao_lop_hoc_va_excel(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (_, other, _) = teacher(&app, "gv2", "toan").await;
    let tag = app.post(&format!("/api/subjects/{toan}/tags"), json!({ "name": "Căn bậc hai" }), Some(&t)).await;
    let tag = tag.body["id"].as_str().unwrap().to_string();
    let c = class(&app, &t, toan, "Toán 9 — báo cáo").await;
    let q = question(&app, &t, toan, Some(&tag)).await;
    let e = exam(&app, &t, toan, &q).await;
    let steps = publish_path(&app, &t, &c, json!([{ "kind": "exam", "examId": e }])).await;
    let (_, s1) = student(&app, "hs_gioi", &c).await;
    let (_, s2) = student(&app, "hs_yeu", &c).await;
    take_exam(&app, &s1, &c, &steps[0], 0).await;
    take_exam(&app, &s2, &c, &steps[0], 1).await;

    let r = app.get(&format!("/api/teaching/classes/{c}/report"), Some(&t)).await;
    ok(&r, StatusCode::OK);
    let b = &r.body;
    assert_eq!(b["exams"][0]["submitted"], 2);
    assert_eq!(b["exams"][0]["avgScore"], 5.0);
    assert_eq!(b["exams"][0]["avgPercent"], 50.0);
    let by_name =
        |name: &str| b["students"].as_array().unwrap().iter().find(|s| s["username"] == name).unwrap().clone();
    assert_eq!(by_name("hs_gioi")["scores"], json!([10.0]));
    assert_eq!(by_name("hs_gioi")["avgPercent"], 100.0);
    assert_eq!(by_name("hs_yeu")["scores"], json!([0.0]));
    assert!(by_name("hs_yeu")["flags"].as_array().unwrap().contains(&json!("low_score")));
    assert!(by_name("hs_yeu")["lastActiveAt"].is_string());
    assert_eq!(b["topics"][0]["name"], "Căn bậc hai");
    assert_eq!(b["topics"][0]["percent"], 50.0);
    assert_eq!(b["levels"][0]["name"], "thong_hieu");
    assert_eq!(b["hardQuestions"][0]["questionId"], q);
    assert_eq!(
        (b["hardQuestions"][0]["answers"].as_i64(), b["hardQuestions"][0]["wrong"].as_i64()),
        (Some(2), Some(1))
    );

    // Giáo viên khác / học sinh không xem được.
    assert_eq!(app.get(&format!("/api/teaching/classes/{c}/report"), Some(&other)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get(&format!("/api/teaching/classes/{c}/report"), Some(&s1)).await.status, StatusCode::FORBIDDEN);

    // Excel
    let x = app.get(&format!("/api/teaching/classes/{c}/report.xlsx"), Some(&t)).await;
    ok(&x, StatusCode::OK);
    assert!(x.headers[header::CONTENT_TYPE].to_str().unwrap().contains("spreadsheetml"));
    assert!(x.headers[header::CONTENT_DISPOSITION].to_str().unwrap().contains("bang-diem-toan-9-bao-cao-"));
    assert!(x.body.as_str().unwrap().starts_with("PK"));
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn hoi_dap_va_bao_vi_pham(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan, "Toán 9").await;
    let l = lesson(&app, &t, toan).await;
    let steps = publish_path(&app, &t, &c, json!([{ "kind": "lesson", "lessonId": l }])).await;
    let (_, s1) = student(&app, "hs_hoi", &c).await;
    let (_, s2) = student(&app, "hs_tra_loi", &c).await;
    let (_, outsider) = app.login_as("hs_ngoai", UserRole::Student).await;

    // Hỏi gắn với bước bài học → giáo viên được báo.
    let res = app
        .post(
            &format!("/api/classes/{c}/threads"),
            json!({ "stepId": steps[0], "title": "Vì sao $\\sqrt{16} = 4$?", "body": "Em chưa hiểu." }),
            Some(&s1),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let th = res.body["id"].as_str().unwrap().to_string();
    assert_eq!(res.body["stepTitle"], "Bài 1");
    assert_eq!(unread(&app, &t).await, 1);
    assert_eq!(latest_kind(&app, &t).await, "question_new");
    // Bước không có trong lộ trình; người ngoài lớp.
    let bad = app
        .post(
            &format!("/api/classes/{c}/threads"),
            json!({ "stepId": Uuid::now_v7(), "title": "x", "body": "y" }),
            Some(&s1),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.get(&format!("/api/classes/{c}/threads"), Some(&outsider)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get(&format!("/api/threads/{th}"), Some(&outsider)).await.status, StatusCode::FORBIDDEN);

    // Bạn cùng lớp trả lời → người hỏi được báo; giáo viên trả lời → cả hai được báo.
    ok(
        &app.post(&format!("/api/threads/{th}/posts"), json!({ "body": "Vì $4^2 = 16$." }), Some(&s2)).await,
        StatusCode::CREATED,
    );
    assert_eq!(latest_kind(&app, &s1).await, "question_reply");
    let res = app.post(&format!("/api/threads/{th}/posts"), json!({ "body": "Đúng rồi." }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    let teacher_post = res.body["posts"][1]["id"].as_str().unwrap().to_string();
    assert_eq!(res.body["posts"][1]["byTeacher"], true);
    assert_eq!(unread(&app, &s1).await, 2);
    assert_eq!(unread(&app, &s2).await, 1);

    // Người hỏi chọn câu trả lời đúng → không còn trong "chưa giải đáp".
    let res = app.post(&format!("/api/posts/{teacher_post}/moderate"), json!({ "action": "accept" }), Some(&s1)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["answered"], true);
    assert_eq!(app.get(&format!("/api/classes/{c}/threads?status=open"), Some(&s1)).await.body["total"], 0);
    assert_eq!(app.get(&format!("/api/classes/{c}/threads?status=answered"), Some(&s2)).await.body["total"], 1);
    // Học sinh khác không chọn được, không ghim được.
    let res =
        app.post(&format!("/api/posts/{teacher_post}/moderate"), json!({ "action": "unaccept" }), Some(&s2)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(
        app.post(&format!("/api/threads/{th}/moderate"), json!({ "action": "pin" }), Some(&s2)).await.status,
        StatusCode::FORBIDDEN
    );

    // Báo vi phạm: không báo nội dung của mình; báo 2 lần chỉ tính 1; giáo viên được báo.
    let rep = json!({ "targetType": "thread", "targetId": th, "reason": "Spam" });
    assert_eq!(app.post("/api/content-reports", rep.clone(), Some(&s1)).await.status, StatusCode::BAD_REQUEST);
    ok(&app.post("/api/content-reports", rep.clone(), Some(&s2)).await, StatusCode::NO_CONTENT);
    ok(&app.post("/api/content-reports", rep, Some(&s2)).await, StatusCode::NO_CONTENT);
    assert_eq!(latest_kind(&app, &t).await, "content_reported");
    let reports = app.get(&format!("/api/teaching/classes/{c}/content-reports"), Some(&t)).await;
    assert_eq!(reports.body.as_array().unwrap().len(), 1);
    assert_eq!(reports.body[0]["threadId"], th);
    assert_eq!(app.get(&format!("/api/classes/{c}/qa-counts"), Some(&t)).await.body["openReports"], 1);
    let rid = reports.body[0]["id"].as_str().unwrap().to_string();
    ok(
        &app.post(&format!("/api/teaching/content-reports/{rid}/resolve"), json!({ "action": "hide" }), Some(&t)).await,
        StatusCode::NO_CONTENT,
    );
    // Câu hỏi bị ẩn: học sinh không thấy, giáo viên vẫn thấy.
    assert_eq!(app.get(&format!("/api/threads/{th}"), Some(&s2)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get(&format!("/api/classes/{c}/threads"), Some(&s2)).await.body["total"], 0);
    let seen = app.get(&format!("/api/threads/{th}"), Some(&t)).await;
    assert_eq!(seen.body["hidden"], true);
    assert_eq!(app.get(&format!("/api/teaching/classes/{c}/content-reports"), Some(&t)).await.body, json!([]));

    // Ẩn một trả lời: học sinh thấy "đã ẩn", không thấy nội dung.
    ok(
        &app.post(&format!("/api/threads/{th}/moderate"), json!({ "action": "unhide" }), Some(&t)).await,
        StatusCode::OK,
    );
    let s2_post = seen.body["posts"][0]["id"].as_str().unwrap().to_string();
    ok(
        &app.post(&format!("/api/posts/{s2_post}/moderate"), json!({ "action": "hide" }), Some(&t)).await,
        StatusCode::OK,
    );
    let view = app.get(&format!("/api/threads/{th}"), Some(&s1)).await.body;
    assert_eq!((view["posts"][0]["hidden"].clone(), view["posts"][0]["body"].clone()), (json!(true), json!("")));

    // Xóa: câu hỏi đã có trả lời thì người hỏi không xóa được; câu hỏi chưa có trả lời thì xóa được.
    assert_eq!(app.delete(&format!("/api/threads/{th}"), Some(&s1)).await.status, StatusCode::CONFLICT);
    let res =
        app.post(&format!("/api/classes/{c}/threads"), json!({ "title": "Hỏi nhầm", "body": "..." }), Some(&s2)).await;
    let th2 = res.body["id"].as_str().unwrap().to_string();
    assert_eq!(app.delete(&format!("/api/threads/{th2}"), Some(&s1)).await.status, StatusCode::FORBIDDEN);
    ok(&app.delete(&format!("/api/threads/{th2}"), Some(&s2)).await, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn the_ghi_nho_trong_lo_trinh(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan, "Toán 9").await;

    let res = app
        .post(
            "/api/teaching/decks",
            json!({ "subjectId": toan, "title": "Công thức lũy thừa", "cards": [
                { "front": "$a^m \\cdot a^n$", "back": "$a^{m+n}$" },
                { "front": "$(a^m)^n$", "back": "$a^{mn}$" }
            ] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let deck = res.body["id"].as_str().unwrap().to_string();
    let first_card = res.body["cards"][0]["id"].clone();
    // Thêm thẻ từ câu hỏi: mặt sau có đáp án + lời giải.
    let q = question(&app, &t, toan, None).await;
    let res =
        app.post(&format!("/api/teaching/decks/{deck}/from-questions"), json!({ "questionIds": [q] }), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["cardCount"], 3);
    assert_eq!(res.body["cards"][2]["back"], "Đáp án: **A.** PA 0\n\nVì $4^2 = 16$.");
    // Sửa: giữ ID thẻ cũ.
    let mut cards = res.body["cards"].as_array().unwrap().clone();
    cards[0]["back"] = json!("$a^{m + n}$");
    let res = app
        .put(
            &format!("/api/teaching/decks/{deck}"),
            json!({ "subjectId": toan, "title": "Công thức lũy thừa", "cards": cards }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["cards"][0]["id"], first_card);
    let card_ids: Vec<String> =
        res.body["cards"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap().to_string()).collect();

    // Bộ thẻ trống không xuất bản được.
    let empty = app.post("/api/teaching/decks", json!({ "subjectId": toan, "title": "Trống" }), Some(&t)).await;
    let empty = empty.body["id"].as_str().unwrap().to_string();
    let res = app
        .put(
            &format!("/api/teaching/classes/{c}/path"),
            json!({ "steps": [{ "kind": "deck", "deckId": empty }] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    let res = app.post(&format!("/api/teaching/classes/{c}/path/publish"), json!({}), Some(&t)).await;
    assert_eq!(res.code(), "DECK_EMPTY");

    let steps = publish_path(&app, &t, &c, json!([{ "kind": "deck", "deckId": deck }])).await;
    let (_, s) = student(&app, "hs", &c).await;
    let url = format!("/api/classes/{c}/steps/{}/deck", steps[0]);
    let d = app.get(&url, Some(&s)).await;
    ok(&d, StatusCode::OK);
    assert_eq!((d.body["newCount"].clone(), d.body["dueCount"].clone()), (json!(3), json!(0)));
    assert_eq!(d.body["cards"][0]["nextMinutes"], json!([10, 1440, 1440, 5760]));
    let learn = app.get(&format!("/api/classes/{c}/learn"), Some(&s)).await.body;
    assert_eq!((learn["steps"][0]["kind"].clone(), learn["steps"][0]["dueCards"].clone()), (json!("deck"), json!(3)));

    // Ôn: Được → 1 ngày; Quên → 10 phút; bước chưa xong cho tới khi mọi thẻ được ôn.
    let review = |card: &str, rating: &str| {
        (format!("/api/classes/{c}/steps/{}/reviews", steps[0]), json!({ "cardId": card, "rating": rating }))
    };
    let (u, b) = review(&card_ids[0], "good");
    let r = app.post(&u, b, Some(&s)).await;
    ok(&r, StatusCode::OK);
    assert_eq!((r.body["intervalDays"].clone(), r.body["saved"].clone()), (json!(1), json!(true)));
    let due: DateTime<Utc> = r.body["dueAt"].as_str().unwrap().parse().unwrap();
    assert!((due - Utc::now()).num_hours() >= 23);
    let (u, b) = review(&card_ids[1], "again");
    assert_eq!(app.post(&u, b, Some(&s)).await.body["intervalDays"], 0);
    assert_eq!(app.get(&format!("/api/classes/{c}/learn"), Some(&s)).await.body["steps"][0]["status"], "available");
    let (u, b) = review(&card_ids[2], "easy");
    ok(&app.post(&u, b, Some(&s)).await, StatusCode::OK);
    let learn = app.get(&format!("/api/classes/{c}/learn"), Some(&s)).await.body;
    assert_eq!(learn["steps"][0]["status"], "done");
    assert_eq!(learn["progress"]["percent"], 100);
    // Thẻ của bộ khác / thẻ không có trong bộ.
    let (u, b) = review(&Uuid::now_v7().to_string(), "good");
    assert_eq!(app.post(&u, b, Some(&s)).await.status, StatusCode::NOT_FOUND);

    // "Ôn tập hôm nay": thẻ Quên đến hạn sau 10 phút → giả lập đã tới hạn.
    let mine = app.get("/api/my/reviews", Some(&s)).await.body;
    assert_eq!(
        (mine[0]["deckId"].clone(), mine[0]["due"].clone(), mine[0]["newCount"].clone()),
        (json!(deck), json!(0), json!(0))
    );
    sqlx::query("UPDATE card_reviews SET due_at = now() - interval '1 minute' WHERE card_id = $1::uuid")
        .bind(&card_ids[1])
        .execute(&app.ctx.db)
        .await
        .unwrap();
    assert_eq!(app.get("/api/my/reviews", Some(&s)).await.body[0]["due"], 1);
    assert_eq!(app.get(&url, Some(&s)).await.body["dueCount"], 1);

    // Giáo viên xem thử: không lưu; tiến độ cả lớp có ô bộ thẻ.
    let (u, b) = review(&card_ids[0], "good");
    assert_eq!(app.post(&u, b, Some(&t)).await.body["saved"], false);
    let progress = app.get(&format!("/api/teaching/classes/{c}/progress"), Some(&t)).await.body;
    assert_eq!(progress["students"][0]["cells"][0]["done"], true);
    // Ẩn bộ thẻ: không còn trong danh sách.
    ok(&app.delete(&format!("/api/teaching/decks/{deck}"), Some(&t)).await, StatusCode::NO_CONTENT);
    assert_eq!(app.get("/api/teaching/decks", Some(&t)).await.body.as_array().unwrap().len(), 1);
}

/// Token trong link của email (`…?token=XYZ`).
fn token_in(text: &str, path: &str) -> String {
    let start = text.find(path).unwrap() + path.len() + "?token=".len();
    text[start..].split_whitespace().next().unwrap().to_string()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn phu_huynh_xac_nhan_va_bao_cao(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan, "Toán 9").await;
    let q = question(&app, &t, toan, None).await;
    let e = exam(&app, &t, toan, &q).await;
    let steps = publish_path(&app, &t, &c, json!([{ "kind": "exam", "examId": e }])).await;
    let (sid, s) = student(&app, "hs_thu", &c).await;
    take_exam(&app, &s, &c, &steps[0], 0).await;
    app.emails();

    // Khai phụ huynh → email xác nhận.
    let res = app
        .post(
            "/api/my/guardians",
            json!({ "fullName": "Trần Minh", "relation": "Bố", "email": "Bo@Example.com" }),
            Some(&s),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(
        (res.body["consentStatus"].clone(), res.body["email"].clone()),
        (json!("pending"), json!("bo@example.com"))
    );
    let gid = res.body["id"].as_str().unwrap().to_string();
    let mails = app.emails();
    assert_eq!(mails.len(), 1);
    assert_eq!(mails[0].to, "bo@example.com");
    assert!(mails[0].text.contains("@hs_thu"));
    let consent = token_in(&mails[0].text, "/phu-huynh/xac-nhan");
    // Trùng email; email của chính mình; gửi lại quá sớm.
    let dup = app.post("/api/my/guardians", json!({ "fullName": "X", "email": "bo@example.com" }), Some(&s)).await;
    assert_eq!(dup.status, StatusCode::UNPROCESSABLE_ENTITY);
    let own = app.post("/api/my/guardians", json!({ "fullName": "X", "email": "hs_thu@example.com" }), Some(&s)).await;
    assert_eq!(own.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        app.post(&format!("/api/my/guardians/{gid}/resend"), json!({}), Some(&s)).await.code(),
        "RESEND_TOO_SOON"
    );
    // Học sinh khác không sửa / xóa được.
    let (_, other) = app.login_as("hs_khac", UserRole::Student).await;
    assert_eq!(app.delete(&format!("/api/my/guardians/{gid}"), Some(&other)).await.status, StatusCode::NOT_FOUND);

    // Chưa xác nhận → giáo viên chưa gửi được.
    let manual = format!("/api/teaching/classes/{c}/students/{sid}/guardian-report");
    assert_eq!(app.post(&manual, json!({ "note": "Tốt" }), Some(&t)).await.code(), "NO_GUARDIAN");

    // Phụ huynh bấm link (không cần đăng nhập).
    let info = app.get(&format!("/api/guardians/consent?token={consent}"), None).await;
    ok(&info, StatusCode::OK);
    assert_eq!(info.body["studentName"], "User hs_thu");
    let res = app.post("/api/guardians/consent", json!({ "token": consent, "accept": true }), None).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["consentStatus"], "confirmed");
    assert_eq!(app.get("/api/guardians/consent?token=sai", None).await.code(), "INVALID_LINK");
    assert_eq!(
        app.post(&format!("/api/my/guardians/{gid}/resend"), json!({}), Some(&s)).await.code(),
        "ALREADY_CONFIRMED"
    );

    // Giáo viên gửi ngay kèm nhận xét.
    let res = app.post(&manual, json!({ "note": "Em làm bài rất tốt." }), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["sent"], 1);
    let mails = app.emails();
    assert_eq!(mails.len(), 1);
    assert!(mails[0].text.contains("Nhận xét của giáo viên:\nEm làm bài rất tốt."));
    assert!(mails[0].text.contains("Kiểm tra căn bậc hai: 10/10 điểm (100%)"));
    let unsub = token_in(&mails[0].text, "/phu-huynh/huy-nhan");
    // Giáo viên khác không gửi được.
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    assert_eq!(app.post(&manual, json!({}), Some(&t2)).await.status, StatusCode::NOT_FOUND);

    // Báo cáo tuần: tối Chủ nhật gửi đúng một lần.
    let sunday: DateTime<Utc> = "2026-11-15T13:00:00Z".parse().unwrap(); // 20:00 Chủ nhật giờ VN
    assert_eq!(guardians::send_weekly_reports(&app.ctx, sunday).await.unwrap(), 1);
    assert_eq!(guardians::send_weekly_reports(&app.ctx, sunday).await.unwrap(), 0);
    let monday: DateTime<Utc> = "2026-11-16T13:00:00Z".parse().unwrap();
    assert_eq!(guardians::send_weekly_reports(&app.ctx, monday).await.unwrap(), 0);
    let mails = app.emails();
    assert_eq!(mails.len(), 1);
    assert!(mails[0].subject.starts_with("Báo cáo học tập tuần của User hs_thu"));
    let list = app.get("/api/my/guardians", Some(&s)).await;
    assert!(list.body[0]["lastReportAt"].is_string());

    // Hủy nhận bằng link → tuần sau không gửi; nhận lại được.
    let res = app.post("/api/guardians/unsubscribe", json!({ "token": unsub }), None).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["receiveReports"], false);
    let next_sunday: DateTime<Utc> = "2026-11-22T13:00:00Z".parse().unwrap();
    assert_eq!(guardians::send_weekly_reports(&app.ctx, next_sunday).await.unwrap(), 0);
    assert_eq!(app.post(&manual, json!({}), Some(&t)).await.code(), "NO_GUARDIAN");
    ok(&app.post("/api/guardians/subscribe", json!({ "token": unsub }), None).await, StatusCode::OK);
    assert_eq!(guardians::send_weekly_reports(&app.ctx, next_sunday).await.unwrap(), 1);

    // Phụ huynh thứ hai từ chối; tối đa 3 phụ huynh.
    app.emails();
    let res = app.post("/api/my/guardians", json!({ "fullName": "Mẹ", "email": "me@example.com" }), Some(&s)).await;
    ok(&res, StatusCode::CREATED);
    let consent2 = token_in(&app.emails()[0].text, "/phu-huynh/xac-nhan");
    let res = app.post("/api/guardians/consent", json!({ "token": consent2, "accept": false }), None).await;
    assert_eq!(res.body["consentStatus"], "declined");
    ok(
        &app.post("/api/my/guardians", json!({ "fullName": "Bà", "email": "ba@example.com" }), Some(&s)).await,
        StatusCode::CREATED,
    );
    let res = app.post("/api/my/guardians", json!({ "fullName": "Ông", "email": "ong@example.com" }), Some(&s)).await;
    assert_eq!(res.code(), "TOO_MANY_GUARDIANS");
    let list = app.get("/api/my/guardians", Some(&s)).await.body;
    assert_eq!(list.as_array().unwrap().len(), 3);
    assert_eq!(list[1]["consentStatus"], "declined");
}
