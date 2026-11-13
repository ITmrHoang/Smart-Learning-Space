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
    assert_eq!(app.get(&board, Some(&t)).await.body[0][0]["type"], "pen");

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
