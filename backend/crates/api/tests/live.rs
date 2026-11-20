//! Đợt 6: phòng học trực tuyến (LiveKit) — token theo vai trò, giáo viên đổi quyền / mời ra / kết thúc,
//! khách vào bằng link + phòng chờ, lưu bình chọn / bảng / lượt lên bảng.
//! LiveKit giả: một server HTTP ghi lại các lần api gọi RoomService.

mod common;

use std::sync::{Arc, Mutex};

use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use chrono::{Duration, Utc};
use common::{TestApp, ok, teacher};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde_json::{Value, json};
use sls_core::{
    config::{Config, LiveConfig},
    modules::users::model::UserRole,
    rate_limit::RateLimiter,
};
use sqlx::PgPool;
use uuid::Uuid;

const SECRET: &str = "live-test-secret-live-test-secret-0123";

type Calls = Arc<Mutex<Vec<(String, Value)>>>;

/// LiveKit giả: nhận `POST /twirp/livekit.RoomService/{method}`, kiểm tra token quản trị, ghi lại body.
async fn fake_livekit() -> (String, Calls) {
    async fn handle(
        State(calls): State<Calls>,
        axum::extract::Path(method): axum::extract::Path<String>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        let token = headers["authorization"].to_str().unwrap().trim_start_matches("Bearer ");
        let claims =
            decode::<Value>(token, &DecodingKey::from_secret(SECRET.as_bytes()), &validation()).unwrap().claims;
        assert_eq!(claims["video"]["roomAdmin"], true);
        assert_eq!(claims["video"]["room"], body["room"]);
        calls.lock().unwrap().push((method, body));
        Json(json!({}))
    }
    let calls: Calls = Arc::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = Router::new().route("/twirp/livekit.RoomService/{method}", post(handle)).with_state(calls.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, calls)
}

fn validation() -> Validation {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_required_spec_claims(&["exp", "sub", "iss"]);
    v
}

fn claims(token: &str) -> Value {
    decode::<Value>(token, &DecodingKey::from_secret(SECRET.as_bytes()), &validation()).unwrap().claims
}

async fn live_app(pool: PgPool) -> (TestApp, Calls) {
    let (url, calls) = fake_livekit().await;
    let mut config = Config::for_tests("postgres://test");
    config.live = Some(LiveConfig {
        public_url: "ws://live.test".into(),
        url,
        api_key: "sls-test".into(),
        api_secret: SECRET.into(),
    });
    (TestApp::with_config(pool, config, RateLimiter::Disabled), calls)
}

async fn class(app: &TestApp, t: &str, subject: Uuid) -> String {
    let res = app
        .post(
            "/api/teaching/classes",
            json!({ "subjectId": subject, "title": "Toán 9", "mode": "live", "visibility": "public", "enrollment": "open" }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

use axum::http::StatusCode;

/// Buổi học bắt đầu sau `minutes` phút, phòng của hệ thống.
async fn live_session(app: &TestApp, t: &str, class_id: &str, minutes: i64, access: &str) -> String {
    let res = app
        .post(
            "/api/teaching/sessions",
            json!({
                "title": "Chữa đề chương 2", "classId": class_id, "startsAt": Utc::now() + Duration::minutes(minutes),
                "durationMinutes": 60, "invitees": { "classIds": [class_id] }, "access": access, "room": "live"
            }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["sessions"][0]["room"], "live");
    res.body["sessions"][0]["id"].as_str().unwrap().to_string()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn phong_live_token_quyen_va_luu_lai(pool: PgPool) {
    let (app, calls) = live_app(pool).await;
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan).await;
    let (an_id, an) = app.login_as("hs_an", UserRole::Student).await;
    ok(&app.post(&format!("/api/classes/{c}/enroll"), json!({}), Some(&an)).await, StatusCode::OK);
    let (_, outsider) = app.login_as("hs_ngoai", UserRole::Student).await;
    assert_eq!(app.get("/api/live/status", None).await.body["enabled"], true);

    // Chưa tới giờ: học sinh chưa vào được; giáo viên mở phòng sớm được.
    let later = live_session(&app, &t, &c, 55, "invited").await;
    let res = app.post(&format!("/api/sessions/{later}/live/join"), json!({}), Some(&an)).await;
    assert_eq!(res.code(), "SESSION_NOT_OPEN");
    ok(&app.post(&format!("/api/sessions/{later}/live/join"), json!({}), Some(&t)).await, StatusCode::OK);

    let s = live_session(&app, &t, &c, -10, "invited").await;
    // "Vào học" của buổi live báo mở phòng trong ứng dụng.
    let res = app.post(&format!("/api/sessions/{s}/join"), json!({}), Some(&an)).await;
    assert_eq!(res.body["room"], "live");

    // Token giáo viên: phát mọi nguồn + quản trị phòng.
    let res = app.post(&format!("/api/sessions/{s}/live/join"), json!({}), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["url"], "ws://live.test");
    assert_eq!(res.body["role"], "teacher");
    let room = res.body["room"].as_str().unwrap().to_string();
    assert_eq!(room, format!("session-{s}"));
    let tc = claims(res.body["token"].as_str().unwrap());
    assert_eq!(tc["video"]["roomAdmin"], true);
    assert_eq!(tc["video"]["canPublish"], true);
    assert_eq!(tc["metadata"], r#"{"role":"teacher"}"#);

    // Token học sinh: chỉ xem / nghe / gửi dữ liệu; điểm danh luôn.
    let res = app.post(&format!("/api/sessions/{s}/live/join"), json!({}), Some(&an)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["identity"], format!("u:{an_id}"));
    let sc = claims(res.body["token"].as_str().unwrap());
    assert_eq!(sc["video"]["canPublish"], false);
    assert_eq!(sc["video"]["canPublishData"], true);
    assert_eq!(sc["video"]["roomAdmin"], false);
    assert_eq!(sc["metadata"], r#"{"role":"student"}"#);
    let detail = app.get(&format!("/api/sessions/{s}"), Some(&an)).await.body;
    assert_eq!(detail["attended"], true);
    assert_eq!(
        app.post(&format!("/api/sessions/{s}/live/join"), json!({}), Some(&outsider)).await.status,
        StatusCode::NOT_FOUND
    );

    // Giáo viên mời bật mic → api đổi quyền ở LiveKit; thu lại; học sinh không tự gọi được.
    let media = format!("/api/sessions/{s}/live/media");
    let who = format!("u:{an_id}");
    ok(&app.post(&media, json!({ "identity": who, "microphone": true }), Some(&t)).await, StatusCode::NO_CONTENT);
    ok(&app.post(&media, json!({ "identity": who }), Some(&t)).await, StatusCode::NO_CONTENT);
    assert_eq!(
        app.post(&media, json!({ "identity": who, "microphone": true }), Some(&an)).await.status,
        StatusCode::FORBIDDEN
    );
    {
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].0, "UpdateParticipant");
        assert_eq!(calls[0].1["identity"], who);
        assert_eq!(calls[0].1["permission"]["canPublish"], true);
        assert_eq!(calls[0].1["permission"]["canPublishSources"], json!(["MICROPHONE"]));
        assert_eq!(calls[1].1["permission"]["canPublish"], false);
    }

    // Lượt lên bảng, bình chọn, ảnh bảng.
    ok(
        &app.post(&format!("/api/sessions/{s}/live/board-turns/start"), json!({ "identity": who }), Some(&t)).await,
        StatusCode::NO_CONTENT,
    );
    ok(
        &app.post(&format!("/api/sessions/{s}/live/board-turns/end"), json!({ "identity": who }), Some(&t)).await,
        StatusCode::NO_CONTENT,
    );
    let bad = app
        .post(
            &format!("/api/sessions/{s}/live/polls"),
            json!({ "kind": "quiz", "question": "1 + 1 = ?", "options": ["1", "2"], "counts": [0, 1], "voters": 1 }),
            Some(&t),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = app
        .post(
            &format!("/api/sessions/{s}/live/polls"),
            json!({ "kind": "quiz", "question": "1 + 1 = ?", "options": ["1", "2", "3"], "correct": [1], "counts": [0, 3, 1], "voters": 4 }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let png = b"\x89PNG\r\n\x1a\n-bang";
    let ticket = app
        .post(
            "/api/files",
            json!({ "purpose": "content", "name": "bang-1.png", "contentType": "image/png", "size": png.len() }),
            Some(&t),
        )
        .await;
    let file = ticket.body["id"].as_str().unwrap().to_string();
    let key: String = sqlx::query_scalar("SELECT object_key FROM files WHERE id = $1::uuid")
        .bind(&file)
        .fetch_one(&app.ctx.db)
        .await
        .unwrap();
    app.ctx.storage.memory_put(&key, png, "image/png");
    ok(&app.post(&format!("/api/files/{file}/complete"), json!({}), Some(&t)).await, StatusCode::OK);
    let board = format!("/api/sessions/{s}/live/board");
    ok(
        &app.put(&board, json!({ "page": 0, "fileId": file, "data": [{ "id": "a", "type": "pen" }] }), Some(&t)).await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(app.put(&board, json!({ "page": 0, "fileId": file }), Some(&an)).await.status, StatusCode::FORBIDDEN);
    let saved = app.get(&board, Some(&t)).await.body;
    assert_eq!(saved["pages"][0][0]["type"], "pen");
    assert_eq!(saved["backgrounds"], json!(["none"]));
    assert_eq!(saved["draft"], false);

    let rec = app.get(&format!("/api/sessions/{s}/live/records"), Some(&an)).await;
    ok(&rec, StatusCode::OK);
    assert_eq!(rec.body["polls"][0]["counts"], json!([0, 3, 1]));
    assert_eq!(rec.body["polls"][0]["options"], json!(["1", "2", "3"]));
    assert_eq!(rec.body["board"][0]["imageUrl"], format!("/api/files/{file}"));
    assert!(rec.body["boardTurns"].as_array().unwrap().is_empty(), "học sinh không xem lượt lên bảng");
    let rec = app.get(&format!("/api/sessions/{s}/live/records"), Some(&t)).await.body;
    assert_eq!(rec["boardTurns"][0]["turns"], 1);

    // Mời ra + kết thúc buổi.
    ok(
        &app.post(&format!("/api/sessions/{s}/live/remove"), json!({ "identity": who }), Some(&t)).await,
        StatusCode::NO_CONTENT,
    );
    ok(&app.post(&format!("/api/sessions/{s}/live/end"), json!({}), Some(&t)).await, StatusCode::NO_CONTENT);
    let methods: Vec<String> = calls.lock().unwrap().iter().map(|(m, _)| m.clone()).collect();
    assert_eq!(&methods[2..], ["RemoveParticipant", "DeleteRoom"]);
}

/// Đợt 10: nháp bảng tự lưu (từng trang + nền), câu hỏi ngân hàng hỏi nhanh trong phòng, chấm trả lời ngắn.
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn bang_nhap_nen_va_cau_hoi_ngan_hang(pool: PgPool) {
    let (app, _) = live_app(pool).await;
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan).await;
    let (_, an) = app.login_as("hs_an", UserRole::Student).await;
    ok(&app.post(&format!("/api/classes/{c}/enroll"), json!({}), Some(&an)).await, StatusCode::OK);
    let s = live_session(&app, &t, &c, -5, "invited").await;
    let board = format!("/api/sessions/{s}/live/board");
    let draft = |page: i32| format!("/api/sessions/{s}/live/board/draft/{page}");

    // Chưa có gì: bảng trống
    let state = app.get(&board, Some(&t)).await;
    ok(&state, StatusCode::OK);
    assert_eq!(state.body, json!({ "pages": [], "backgrounds": [], "draft": false }));

    // Tự lưu trang 0 (nền ô ly) và trang 2 (hệ trục) — trang 1 chưa vẽ gì
    let pen = json!([{ "id": "a", "type": "pen", "points": [[1, 1], [5, 5]] }]);
    ok(&app.put(&draft(0), json!({ "data": pen, "background": "oly" }), Some(&t)).await, StatusCode::NO_CONTENT);
    ok(&app.put(&draft(2), json!({ "data": [], "background": "axes" }), Some(&t)).await, StatusCode::NO_CONTENT);
    let state = app.get(&board, Some(&t)).await.body;
    assert_eq!(state["draft"], true);
    assert_eq!(state["pages"].as_array().unwrap().len(), 3);
    assert_eq!(state["pages"][0][0]["id"], "a");
    assert_eq!(state["backgrounds"], json!(["oly", "none", "axes"]));
    // Ghi đè trang 0, báo còn 1 trang → trang nháp thừa bị xóa
    let res = app.put(&draft(0), json!({ "data": [], "background": "grid", "pageCount": 1 }), Some(&t)).await;
    ok(&res, StatusCode::NO_CONTENT);
    assert_eq!(app.get(&board, Some(&t)).await.body["backgrounds"], json!(["grid"]));

    // Lỗi: học sinh không lưu / xem nháp; nền lạ; dữ liệu không phải mảng; trang ngoài phạm vi
    assert_eq!(app.put(&draft(0), json!({ "data": [] }), Some(&an)).await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get(&board, Some(&an)).await.status, StatusCode::FORBIDDEN);
    let bad_bg = app.put(&draft(0), json!({ "data": [], "background": "tranh" }), Some(&t)).await;
    assert_eq!(bad_bg.status, StatusCode::UNPROCESSABLE_ENTITY);
    let bad_data = app.put(&draft(0), json!({ "data": { "a": 1 } }), Some(&t)).await;
    assert_eq!(bad_data.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(app.put(&draft(99), json!({ "data": [] }), Some(&t)).await.status, StatusCode::UNPROCESSABLE_ENTITY);

    // Câu trả lời ngắn trong ngân hàng → chấm nhanh câu trả lời của cả lớp ở server
    let q = app
        .post(
            "/api/teaching/questions",
            json!({ "subjectId": toan, "kind": "short", "level": "thong_hieu", "content": "Khai triển $2(x+1)$",
                    "accepted": ["2(x+1)"], "matchMode": "expression" }),
            Some(&t),
        )
        .await;
    ok(&q, StatusCode::CREATED);
    let qid = q.body["id"].as_str().unwrap().to_string();
    let res = app
        .post(
            &format!("/api/teaching/questions/{qid}/check"),
            json!({ "answers": ["2x + 2", "2x + 1", "2 + 2x", ""] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["results"], json!([true, false, true, false]));
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    let res = app.post(&format!("/api/teaching/questions/{qid}/check"), json!({ "answers": ["1"] }), Some(&t2)).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);

    // Lưu kết quả câu hỏi ngân hàng (trả lời ngắn: không có phương án)
    let polls = format!("/api/sessions/{s}/live/polls");
    let res = app
        .post(
            &polls,
            json!({ "kind": "question", "questionId": qid, "questionKind": "short", "question": "Khai triển $2(x+1)$",
                    "options": [], "counts": [], "voters": 4, "rightCount": 2 }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["rightCount"], 2);
    assert_eq!(res.body["questionKind"], "short");
    // Câu của người khác / số đúng vượt số người trả lời → lỗi
    let res = app
        .post(
            &polls,
            json!({ "kind": "question", "questionId": Uuid::now_v7(), "questionKind": "short", "question": "x",
                    "options": [], "counts": [], "voters": 1, "rightCount": 0 }),
            Some(&t),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = app
        .post(
            &polls,
            json!({ "kind": "question", "questionId": qid, "questionKind": "short", "question": "x",
                    "options": [], "counts": [], "voters": 1, "rightCount": 3 }),
            Some(&t),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let rec = app.get(&format!("/api/sessions/{s}/live/records"), Some(&an)).await.body;
    assert_eq!(rec["polls"][0]["kind"], "question");
    assert_eq!(rec["polls"][0]["rightCount"], 2);
}

/// Upload file tạm (mục đích `import`) vào kho trong bộ nhớ, trả id.
async fn upload_import(app: &TestApp, token: &str, name: &str, content_type: &str, bytes: Vec<u8>) -> String {
    let res = app
        .post(
            "/api/files",
            json!({ "purpose": "import", "name": name, "contentType": content_type, "size": bytes.len() }),
            Some(token),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let id = res.body["id"].as_str().unwrap().to_string();
    let key: String = sqlx::query_scalar("SELECT object_key FROM files WHERE id = $1::uuid")
        .bind(&id)
        .fetch_one(&app.ctx.db)
        .await
        .unwrap();
    app.ctx.storage.memory_put(&key, &bytes, content_type);
    ok(&app.post(&format!("/api/files/{id}/complete"), json!({}), Some(token)).await, StatusCode::OK);
    id
}

/// Nhập PDF đề thành trang bảng — đường dự phòng ở máy chủ (khi trình duyệt không tự đọc được).
#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn nhap_pdf_thanh_trang_bang_o_may_chu(pool: PgPool) {
    use sls_core::modules::live::pdf::sample_pdf;

    let (app, _) = live_app(pool).await;
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan).await;
    let (_, an) = app.login_as("hs_an", UserRole::Student).await;
    ok(&app.post(&format!("/api/classes/{c}/enroll"), json!({}), Some(&an)).await, StatusCode::OK);
    let s = live_session(&app, &t, &c, -5, "invited").await;
    let url = format!("/api/sessions/{s}/live/board/pdf");

    // 3 trang, chỉ lấy 2 (bảng còn chỗ 2 trang) → ảnh PNG nội dung, file PDF tạm bị xóa
    let pdf = upload_import(&app, &t, "Đề kiểm tra.pdf", "application/pdf", sample_pdf(3)).await;
    let res = app.post(&url, json!({ "fileId": pdf, "maxPages": 2 }), Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["total"], 3);
    let pages = res.body["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0]["height"], 1800);
    assert!(pages[0]["url"].as_str().unwrap().starts_with("/api/files/"));
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM files WHERE purpose = 'content' AND content_type = 'image/png' ORDER BY name",
    )
    .fetch_all(&app.ctx.db)
    .await
    .unwrap();
    assert_eq!(names, ["Đề kiểm tra - trang 1.png", "Đề kiểm tra - trang 2.png"]);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM files WHERE id = $1::uuid")
        .bind(&pdf)
        .fetch_one(&app.ctx.db)
        .await
        .unwrap();
    assert_eq!(left, 0, "file PDF tạm đã xóa");
    // Gọi lại với file đã dùng → không còn
    assert_eq!(app.post(&url, json!({ "fileId": pdf }), Some(&t)).await.status, StatusCode::NOT_FOUND);

    // Học sinh không được nhập; file không đọc được → lỗi trường `file`
    let pdf = upload_import(&app, &t, "de.pdf", "application/pdf", sample_pdf(1)).await;
    assert_eq!(app.post(&url, json!({ "fileId": pdf }), Some(&an)).await.status, StatusCode::FORBIDDEN);
    let broken = upload_import(&app, &t, "hong.pdf", "application/pdf", b"%PDF-1.4\nrac".to_vec()).await;
    let res = app.post(&url, json!({ "fileId": broken }), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res.body["error"]["fields"]["file"][0].as_str().unwrap().contains("PDF"));
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn khach_vao_bang_link_qua_phong_cho(pool: PgPool) {
    let (app, calls) = live_app(pool).await;
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan).await;

    // Mở cho khách chỉ với phòng của hệ thống.
    let bad = app
        .post(
            "/api/teaching/sessions",
            json!({ "title": "x", "startsAt": Utc::now(), "durationMinutes": 60, "access": "public_link",
                    "meetingUrl": "https://meet.google.com/a" }),
            Some(&t),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
    let invited = live_session(&app, &t, &c, 70, "invited").await;
    assert_eq!(
        app.post(&format!("/api/sessions/{invited}/live/guest-link"), json!({}), Some(&t)).await.code(),
        "NOT_PUBLIC"
    );

    let s = live_session(&app, &t, &c, 5, "public_link").await;
    let link = app.post(&format!("/api/sessions/{s}/live/guest-link"), json!({}), Some(&t)).await;
    ok(&link, StatusCode::OK);
    let url = link.body["url"].as_str().unwrap();
    assert!(url.contains(&format!("/vao-hoc/{s}?k=")));
    let key = url.rsplit("k=").next().unwrap().to_string();

    // Sai khóa → không vào được; đúng khóa → phòng chờ.
    let join = format!("/api/live/guest/{s}/join");
    assert_eq!(
        app.post(&join, json!({ "key": "sai", "name": "Phụ huynh A" }), None).await.status,
        StatusCode::NOT_FOUND
    );
    let res = app.post(&join, json!({ "key": key, "name": "Phụ huynh A" }), None).await;
    ok(&res, StatusCode::CREATED);
    let guest = res.body["guestId"].as_str().unwrap().to_string();
    let token = res.body["token"].as_str().unwrap().to_string();
    let state = |g: String, tk: String| {
        let app = &app;
        async move { app.get(&format!("/api/live/guest/state/{g}?token={tk}"), None).await }
    };
    let st = state(guest.clone(), token.clone()).await;
    assert_eq!(st.body["status"], "waiting");
    assert!(st.body["live"].is_null());
    assert_eq!(state(guest.clone(), "sai".into()).await.status, StatusCode::NOT_FOUND);

    // Giáo viên thấy phòng chờ, duyệt → khách nhận token chỉ xem / nghe.
    let list = app.get(&format!("/api/sessions/{s}/live/guests"), Some(&t)).await.body;
    assert_eq!(list[0]["name"], "Phụ huynh A");
    ok(
        &app.post(&format!("/api/sessions/{s}/live/guests/{guest}/decide"), json!({ "admit": true }), Some(&t)).await,
        StatusCode::OK,
    );
    let st = state(guest.clone(), token.clone()).await.body;
    assert_eq!(st["status"], "admitted");
    assert_eq!(st["live"]["role"], "guest");
    assert_eq!(st["live"]["name"], "Phụ huynh A (Khách)");
    let gc = claims(st["live"]["token"].as_str().unwrap());
    assert_eq!(gc["video"]["canPublish"], false);
    assert_eq!(gc["video"]["canPublishData"], false);
    assert_eq!(gc["video"]["canSubscribe"], true);

    // Khách thứ hai bị từ chối; "duyệt tất cả" không đụng tới người đã bị từ chối.
    let res = app.post(&join, json!({ "key": key, "name": "Khách B" }), None).await;
    let b = res.body["guestId"].as_str().unwrap().to_string();
    let tb = res.body["token"].as_str().unwrap().to_string();
    ok(
        &app.post(&format!("/api/sessions/{s}/live/guests/{b}/decide"), json!({ "admit": false }), Some(&t)).await,
        StatusCode::OK,
    );
    ok(
        &app.post(&format!("/api/sessions/{s}/live/guests/decide-all"), json!({ "admit": true }), Some(&t)).await,
        StatusCode::OK,
    );
    assert_eq!(state(b, tb).await.body["status"], "denied");

    // Mời khách ra → không lấy lại token được.
    ok(
        &app.post(&format!("/api/sessions/{s}/live/remove"), json!({ "identity": format!("g:{guest}") }), Some(&t))
            .await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(state(guest, token).await.body["status"], "removed");
    assert_eq!(calls.lock().unwrap().last().unwrap().0, "RemoveParticipant");

    // Đổi link → khóa cũ hết hiệu lực.
    ok(&app.post(&format!("/api/sessions/{s}/live/guest-link"), json!({}), Some(&t)).await, StatusCode::OK);
    assert_eq!(app.post(&join, json!({ "key": key, "name": "Khách C" }), None).await.status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn chua_cau_hinh_livekit(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan).await;
    assert_eq!(app.get("/api/live/status", None).await.body["enabled"], false);
    let res = app
        .post(
            "/api/teaching/sessions",
            json!({ "title": "x", "classId": c, "startsAt": Utc::now(), "durationMinutes": 60,
                    "invitees": { "classIds": [c] }, "access": "invited", "room": "live" }),
            Some(&t),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tro_giang_dieu_khien_phong_nhu_giao_vien(pool: PgPool) {
    let (app, calls) = live_app(pool).await;
    let (gv_id, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan).await;
    let (_, tg) = app.login_as("gv_tro_giang", UserRole::Teacher).await;
    let (an_id, an) = app.login_as("hs_an", UserRole::Student).await;
    ok(&app.post(&format!("/api/classes/{c}/enroll"), json!({}), Some(&an)).await, StatusCode::OK);
    ok(
        &app.post(&format!("/api/teaching/classes/{c}/assistants"), json!({ "login": "gv_tro_giang" }), Some(&t)).await,
        StatusCode::CREATED,
    );
    // Phòng mở sớm cho trợ giảng như giáo viên; buổi của lớp hiện trong lịch của trợ giảng
    let s = live_session(&app, &t, &c, 30, "invited").await;
    let res = app.post(&format!("/api/sessions/{s}/live/join"), json!({}), Some(&tg)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["role"], "assistant");
    let tc = claims(res.body["token"].as_str().unwrap());
    assert_eq!(tc["metadata"], r#"{"role":"assistant"}"#);
    assert_eq!(tc["video"]["canPublish"], true);
    assert_ne!(tc["video"]["roomAdmin"], true);
    let from = (Utc::now() - Duration::days(1)).to_rfc3339();
    let to = (Utc::now() + Duration::days(1)).to_rfc3339();
    let res = app.get(&format!("/api/my/sessions?from={}&to={}", urlencode(&from), urlencode(&to)), Some(&tg)).await;
    ok(&res, StatusCode::OK);
    assert!(res.body.as_array().unwrap().iter().any(|x| x["id"] == s.as_str()), "{}", res.body);

    // Cho học sinh nói, mời lên bảng, mời ra; không đụng được giáo viên chủ buổi
    let media =
        |identity: String| json!({ "identity": identity, "microphone": true, "camera": false, "screen": false });
    ok(
        &app.post(&format!("/api/sessions/{s}/live/media"), media(format!("u:{an_id}")), Some(&tg)).await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(calls.lock().unwrap().last().unwrap().0, "UpdateParticipant");
    let res = app.post(&format!("/api/sessions/{s}/live/media"), media(format!("u:{gv_id}")), Some(&tg)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let res = app
        .post(&format!("/api/sessions/{s}/live/remove"), json!({ "identity": format!("u:{gv_id}") }), Some(&tg))
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    ok(
        &app.post(
            &format!("/api/sessions/{s}/live/board-turns/start"),
            json!({ "identity": format!("u:{an_id}") }),
            Some(&tg),
        )
        .await,
        StatusCode::NO_CONTENT,
    );
    // Kết thúc buổi, link khách: chỉ giáo viên chủ buổi
    assert_eq!(
        app.post(&format!("/api/sessions/{s}/live/end"), json!({}), Some(&tg)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.post(&format!("/api/sessions/{s}/live/guest-link"), json!({}), Some(&tg)).await.status,
        StatusCode::NOT_FOUND
    );
    // Học sinh vẫn không điều khiển được
    let res = app.post(&format!("/api/sessions/{s}/live/media"), media(format!("u:{an_id}")), Some(&an)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
}

fn urlencode(s: &str) -> String {
    s.replace(':', "%3A").replace('+', "%2B")
}
