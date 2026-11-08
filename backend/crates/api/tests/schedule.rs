//! M13–M16: nhóm học sinh, buổi học (lặp, mời, .ics, sửa / hủy theo phạm vi, trả lời, vào học, điểm danh),
//! nhắc lịch, thông báo, bước buổi học trong lộ trình.

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use common::{TestApp, ok, teacher};
use serde_json::{Value, json};
use sls_core::modules::{sessions::service as sessions, users::model::UserRole};
use sqlx::PgPool;
use uuid::Uuid;

/// Lớp học công khai, tự vào; trả về id.
async fn class(app: &TestApp, t: &str, subject: Uuid, title: &str) -> String {
    let res = app
        .post(
            "/api/teaching/classes",
            json!({ "subjectId": subject, "title": title, "mode": "live", "visibility": "public", "enrollment": "open" }),
            Some(t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    res.body["id"].as_str().unwrap().to_string()
}

/// Học sinh đăng nhập + vào lớp. Trả về (id, token).
async fn student(app: &TestApp, name: &str, class_id: &str) -> (Uuid, String) {
    let (id, token) = app.login_as(name, UserRole::Student).await;
    ok(&app.post(&format!("/api/classes/{class_id}/enroll"), json!({}), Some(&token)).await, StatusCode::OK);
    (id, token)
}

fn session_body(class_id: &str, start: chrono::DateTime<Utc>, extra: Value) -> Value {
    let mut body = json!({
        "title": "Buổi 1: căn bậc hai",
        "classId": class_id,
        "startsAt": start,
        "durationMinutes": 90,
        "invitees": { "classIds": [class_id] },
        "access": "invited",
        "meetingUrl": "https://meet.google.com/abc-defg-hij",
        "remindMinutes": 60
    });
    for (k, v) in extra.as_object().unwrap() {
        body[k] = v.clone();
    }
    body
}

/// Thứ Ba tuần sau lúc 19:30 giờ Việt Nam (12:30 UTC).
fn next_tuesday() -> chrono::DateTime<Utc> {
    use chrono::{Datelike, NaiveTime, TimeZone};
    let today = (Utc::now() + Duration::hours(7)).date_naive();
    let days = (7 + 1 - today.weekday().num_days_from_monday() as i64) % 7 + 7; // thứ Ba của tuần sau
    let day = today + Duration::days(days);
    Utc.from_utc_datetime(&day.and_time(NaiveTime::from_hms_opt(12, 30, 0).unwrap()))
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn nhom_hoc_sinh(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c1 = class(&app, &t, toan, "Toán 9A").await;
    let c2 = class(&app, &t, toan, "Toán 9B").await;
    let (a, _) = student(&app, "hs_a", &c1).await;
    let (b, _) = student(&app, "hs_b", &c2).await;
    let (outsider, _) = app.login_as("hs_ngoai", UserRole::Student).await;

    let list = app.get("/api/teaching/students", Some(&t)).await;
    assert_eq!(list.body.as_array().unwrap().len(), 2);
    assert_eq!(
        app.get(&format!("/api/teaching/students?classId={c1}"), Some(&t)).await.body.as_array().unwrap().len(),
        1
    );
    assert_eq!(app.get("/api/teaching/students?q=hs_b", Some(&t)).await.body[0]["id"], b.to_string());

    // Nhóm gom học sinh từ 2 lớp học.
    let res = app.post("/api/teaching/groups", json!({ "name": "Nhóm HSG", "memberIds": [a, b] }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["memberCount"], 2);
    let gid = res.body["id"].as_str().unwrap().to_string();
    // Học sinh không học lớp nào của giáo viên → bị từ chối.
    let res = app.post("/api/teaching/groups", json!({ "name": "Sai", "memberIds": [outsider] }), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    // Sửa: bỏ bớt thành viên, gắn vào lớp 9A.
    let res = app
        .put(
            &format!("/api/teaching/groups/{gid}"),
            json!({ "name": "Nhóm HSG 9A", "classId": c1, "memberIds": [a] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["members"].as_array().unwrap().len(), 1);
    assert_eq!(res.body["classTitle"], "Toán 9A");
    // Giáo viên khác không thấy nhóm.
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    assert_eq!(app.get(&format!("/api/teaching/groups/{gid}"), Some(&t2)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get("/api/teaching/groups", Some(&t2)).await.body, json!([]));
    ok(&app.delete(&format!("/api/teaching/groups/{gid}"), Some(&t)).await, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tao_buoi_moi_lop_email_ics_va_quyen_xem(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan, "Toán 9A").await;
    let (_, s1) = student(&app, "hs1", &c).await;
    let (_, s_other) = app.login_as("hs_khac", UserRole::Student).await;
    app.emails();

    let start = next_tuesday();
    let res = app.post("/api/teaching/sessions", session_body(&c, start, json!({})), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    let sid = res.body["sessions"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(res.body["studentConflicts"], 0);

    // Email mời kèm .ics cho học sinh trong lớp.
    let emails = app.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].to, "hs1@example.com");
    assert!(emails[0].subject.starts_with("Lời mời buổi học: Buổi 1"));
    assert!(emails[0].text.contains("19:30 thứ Ba"));
    let ics = &emails[0].attachments[0];
    assert!(ics.content_type.contains("method=REQUEST"));
    assert!(ics.content.contains(&format!("UID:{sid}@smart-learning-space")));
    assert!(ics.content.contains("SEQUENCE:0"));

    // Học sinh thấy trên lịch + thông báo; học sinh khác không thấy.
    let mine = app.get("/api/my/sessions", Some(&s1)).await;
    assert_eq!(mine.body.as_array().unwrap().len(), 1);
    assert_eq!(app.get("/api/notifications/unread-count", Some(&s1)).await.body["unread"], 1);
    assert_eq!(app.get("/api/my/sessions", Some(&s_other)).await.body, json!([]));
    assert_eq!(app.get(&format!("/api/sessions/{sid}"), Some(&s_other)).await.status, StatusCode::NOT_FOUND);
    // Học sinh xem chi tiết: có link họp, không có danh sách người được mời.
    let d = app.get(&format!("/api/sessions/{sid}"), Some(&s1)).await;
    assert_eq!(d.body["meetingUrl"], "https://meet.google.com/abc-defg-hij");
    assert!(d.body["attendees"].is_null());
    assert_eq!(d.body["canJoin"], false);
    // Học sinh vào lớp sau vẫn thấy buổi sắp tới (mời theo lớp học).
    let (_, late) = student(&app, "hs_sau", &c).await;
    assert_eq!(app.get("/api/my/sessions", Some(&late)).await.body.as_array().unwrap().len(), 1);

    // Mời "mọi tài khoản": học sinh ngoài lớp xem được chi tiết (nhưng không có trên lịch của mình).
    let res = app
        .post(
            "/api/teaching/sessions",
            session_body(&c, start + Duration::days(1), json!({ "access": "logged_in", "invitees": {} })),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let open = res.body["sessions"][0]["id"].as_str().unwrap();
    ok(&app.get(&format!("/api/sessions/{open}"), Some(&s_other)).await, StatusCode::OK);

    // Dữ liệu sai: không mời ai với "chỉ người được mời"; lớp học của người khác.
    let res = app
        .post(
            "/api/teaching/sessions",
            session_body(&c, start + Duration::days(2), json!({ "invitees": {} })),
            Some(&t),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    let res =
        app.post("/api/teaching/sessions", session_body(&c, start + Duration::days(3), json!({})), Some(&t2)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    // Học sinh không tạo được buổi.
    assert_eq!(
        app.post("/api/teaching/sessions", session_body(&c, start, json!({})), Some(&s1)).await.status,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn lich_lap_trung_lich_sua_va_huy_theo_pham_vi(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan, "Toán 9A").await;
    let (hs, s1) = student(&app, "hs1", &c).await;
    let start = next_tuesday();
    let until = (start + Duration::days(9)).date_naive(); // thứ Ba, thứ Năm × 2 tuần

    let res = app
        .post(
            "/api/teaching/sessions",
            session_body(&c, start, json!({ "repeat": { "weekdays": [2, 4], "until": until } })),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let ids: Vec<String> =
        res.body["sessions"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect();
    assert_eq!(ids.len(), 4);
    let starts: Vec<String> =
        res.body["sessions"].as_array().unwrap().iter().map(|s| s["startsAt"].as_str().unwrap().to_string()).collect();
    assert_eq!(chrono::DateTime::parse_from_rfc3339(&starts[1]).unwrap(), start + Duration::days(2));
    // Một email có .ics chứa cả 4 buổi.
    let emails = app.emails();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].attachments[0].content.matches("BEGIN:VEVENT").count(), 4);

    // Giáo viên trùng lịch của chính mình → chặn.
    let res =
        app.post("/api/teaching/sessions", session_body(&c, start + Duration::minutes(30), json!({})), Some(&t)).await;
    assert_eq!(res.code(), "SESSION_CONFLICT");
    // Giáo viên khác mời cùng học sinh vào giờ trùng → được tạo, kèm cảnh báo số học sinh trùng lịch.
    let (_, t2, _) = teacher(&app, "gv2", "toan").await;
    let c2 = class(&app, &t2, toan, "Lớp của gv2").await;
    app.post(&format!("/api/classes/{c2}/enroll"), json!({}), Some(&s1)).await;
    let res = app
        .post("/api/teaching/sessions", session_body(&c2, start + Duration::minutes(30), json!({})), Some(&t2))
        .await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["studentConflicts"], 1);
    app.emails();

    // Sửa "buổi này và các buổi sau" từ buổi 2: dời 1 giờ.
    let body = session_body(
        &c,
        start + Duration::days(2) + Duration::hours(1),
        json!({ "scope": "following", "title": "Buổi học (dời)" }),
    );
    let res = app.put(&format!("/api/teaching/sessions/{}", ids[1]), body, Some(&t)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["updated"], 3);
    let list = app
        .get(
            &format!(
                "/api/teaching/sessions?from={}&to={}",
                (start - Duration::days(1)).to_rfc3339(),
                (start + Duration::days(20)).to_rfc3339()
            )
            .replace('+', "%2B"),
            Some(&t),
        )
        .await;
    let mine: Vec<&Value> =
        list.body.as_array().unwrap().iter().filter(|s| ids.contains(&s["id"].as_str().unwrap().to_string())).collect();
    assert_eq!(mine[0]["title"], "Buổi 1: căn bậc hai");
    assert_eq!(chrono::DateTime::parse_from_rfc3339(mine[0]["startsAt"].as_str().unwrap()).unwrap(), start);
    assert_eq!(mine[3]["title"], "Buổi học (dời)");
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(mine[3]["startsAt"].as_str().unwrap()).unwrap(),
        start + Duration::days(9) + Duration::hours(1)
    );
    let emails = app.emails();
    assert!(emails[0].subject.starts_with("Buổi học đã thay đổi"));
    assert!(emails[0].attachments[0].content.contains("SEQUENCE:1"));

    // Hủy "chỉ buổi này" (buổi 1), rồi "buổi này và các buổi sau" từ buổi 3.
    let res =
        app.post(&format!("/api/teaching/sessions/{}/cancel", ids[0]), json!({ "scope": "this" }), Some(&t)).await;
    assert_eq!(res.body["updated"], 1);
    let res =
        app.post(&format!("/api/teaching/sessions/{}/cancel", ids[2]), json!({ "scope": "following" }), Some(&t)).await;
    assert_eq!(res.body["updated"], 2);
    let emails = app.emails();
    assert!(emails.iter().all(|e| e.attachments[0].content.contains("METHOD:CANCEL")));
    let statuses: Vec<String> = app
        .get("/api/my/sessions", Some(&s1))
        .await
        .body
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| ids.contains(&s["id"].as_str().unwrap().to_string()))
        .map(|s| s["status"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(statuses, vec!["cancelled", "scheduled", "cancelled", "cancelled"]);
    // Không trả lời / sửa được buổi đã hủy.
    let res = app.post(&format!("/api/sessions/{}/rsvp", ids[0]), json!({ "response": "yes" }), Some(&s1)).await;
    assert_eq!(res.code(), "SESSION_CLOSED");
    assert_eq!(
        app.put(&format!("/api/teaching/sessions/{}", ids[0]), session_body(&c, start, json!({})), Some(&t))
            .await
            .code(),
        "SESSION_CANCELLED"
    );
    let _ = hs;
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tra_loi_vao_hoc_diem_danh_va_nhac_lich(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan, "Toán 9A").await;
    let (hs1, s1) = student(&app, "hs1", &c).await;
    let (hs2, s2) = student(&app, "hs2", &c).await;
    let (_, outsider) = app.login_as("hs_ngoai", UserRole::Student).await;
    let res = app.post("/api/teaching/sessions", session_body(&c, next_tuesday(), json!({})), Some(&t)).await;
    let sid: Uuid = res.body["sessions"][0]["id"].as_str().unwrap().parse().unwrap();

    ok(
        &app.post(&format!("/api/sessions/{sid}/rsvp"), json!({ "response": "yes" }), Some(&s1)).await,
        StatusCode::NO_CONTENT,
    );
    ok(
        &app.post(&format!("/api/sessions/{sid}/rsvp"), json!({ "response": "no" }), Some(&s2)).await,
        StatusCode::NO_CONTENT,
    );
    assert_eq!(
        app.post(&format!("/api/sessions/{sid}/rsvp"), json!({ "response": "yes" }), Some(&outsider)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(app.get("/api/my/sessions", Some(&s1)).await.body[0]["myRsvp"], "yes");

    // Chưa tới giờ → chưa vào được.
    assert_eq!(app.post(&format!("/api/sessions/{sid}/join"), json!({}), Some(&s1)).await.code(), "SESSION_NOT_OPEN");
    // Còn 5 phút → vào được, ghi điểm danh.
    sqlx::query("UPDATE sessions SET starts_at = now() + interval '5 minutes', ends_at = now() + interval '95 minutes' WHERE id = $1")
        .bind(sid)
        .execute(&app.ctx.db)
        .await
        .unwrap();
    let res = app.post(&format!("/api/sessions/{sid}/join"), json!({}), Some(&s1)).await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["meetingUrl"], "https://meet.google.com/abc-defg-hij");
    // Giáo viên điểm danh tay hs2, xem danh sách.
    ok(
        &app.put(
            &format!("/api/teaching/sessions/{sid}/attendance"),
            json!({ "userId": hs2, "attended": true }),
            Some(&t),
        )
        .await,
        StatusCode::NO_CONTENT,
    );
    let detail = app.get(&format!("/api/sessions/{sid}"), Some(&t)).await;
    let attendees = detail.body["attendees"].as_array().unwrap();
    let find = |id: Uuid| attendees.iter().find(|a| a["userId"] == id.to_string()).unwrap().clone();
    assert_eq!(find(hs1)["rsvp"], "yes");
    assert_eq!(find(hs1)["attended"], true);
    assert!(find(hs1)["joinedAt"].is_string());
    assert_eq!(find(hs2)["rsvp"], "no");
    assert_eq!(find(hs2)["attended"], true);
    assert_eq!(detail.body["invitees"]["classes"][0]["name"], "Toán 9A");

    // Nhắc lịch: buổi bắt đầu sau 5 phút, nhắc trước 60 phút → gửi một lần.
    app.emails();
    assert_eq!(sessions::send_due_reminders(&app.ctx).await.unwrap(), 1);
    assert_eq!(sessions::send_due_reminders(&app.ctx).await.unwrap(), 0);
    let emails = app.emails();
    assert_eq!(emails.len(), 2);
    assert!(emails.iter().all(|e| e.subject.starts_with("Nhắc lịch học") && e.attachments.is_empty()));
    let n = app.get("/api/notifications?unread=true", Some(&s1)).await;
    assert_eq!(n.body["items"][0]["kind"], "session_reminder");
    assert_eq!(n.body["items"][0]["link"], format!("/buoi-hoc/{sid}"));
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn buoi_hoc_trong_lo_trinh_va_thong_bao(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let c = class(&app, &t, toan, "Toán 9A").await;
    let (_, s1) = app.login_as("hs1", UserRole::Student).await;
    // Lớp duyệt ghi danh → được duyệt thì có thông báo.
    app.patch_class_enrollment(&t, &c, "approval").await;
    let enrolled = app.post(&format!("/api/classes/{c}/enroll"), json!({}), Some(&s1)).await;
    assert_eq!(enrolled.body["status"], "pending");
    let me = app.get("/api/auth/me", Some(&s1)).await.body["id"].as_str().unwrap().to_string();
    ok(
        &app.post(&format!("/api/teaching/classes/{c}/members/{me}/approve"), json!({}), Some(&t)).await,
        StatusCode::NO_CONTENT,
    );
    let list = app.get("/api/notifications", Some(&s1)).await;
    assert_eq!(list.body["items"][0]["kind"], "enrollment_approved");

    let res = app.post("/api/teaching/sessions", session_body(&c, next_tuesday(), json!({})), Some(&t)).await;
    let sid = res.body["sessions"][0]["id"].as_str().unwrap().to_string();
    // Buổi của lớp khác không thêm vào lộ trình được.
    let other = class(&app, &t, toan, "Lớp khác").await;
    let res = app
        .put(
            &format!("/api/teaching/classes/{other}/path"),
            json!({ "steps": [{ "kind": "session", "sessionId": sid }] }),
            Some(&t),
        )
        .await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);

    let res = app
        .put(
            &format!("/api/teaching/classes/{c}/path"),
            json!({ "steps": [{ "kind": "session", "sessionId": sid }] }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::OK);
    assert_eq!(res.body["steps"][0]["refTitle"], "Buổi 1: căn bậc hai");
    assert_eq!(res.body["steps"][0]["refStatus"], "scheduled");
    ok(&app.post(&format!("/api/teaching/classes/{c}/path/publish"), json!({}), Some(&t)).await, StatusCode::OK);
    let list = app.get("/api/notifications", Some(&s1)).await;
    assert_eq!(list.body["items"][0]["kind"], "path_published");

    let learn = app.get(&format!("/api/classes/{c}/learn"), Some(&s1)).await;
    assert_eq!(learn.body["steps"][0]["kind"], "session");
    assert_eq!(learn.body["steps"][0]["sessionId"], sid);
    assert_eq!(learn.body["steps"][0]["status"], "available");
    // Có điểm danh → bước xong.
    let sid_uuid: Uuid = sid.parse().unwrap();
    sqlx::query("UPDATE sessions SET starts_at = now(), ends_at = now() + interval '1 hour' WHERE id = $1")
        .bind(sid_uuid)
        .execute(&app.ctx.db)
        .await
        .unwrap();
    ok(&app.post(&format!("/api/sessions/{sid}/join"), json!({}), Some(&s1)).await, StatusCode::OK);
    let learn = app.get(&format!("/api/classes/{c}/learn"), Some(&s1)).await;
    assert_eq!(learn.body["steps"][0]["status"], "done");
    assert_eq!(learn.body["progress"]["percent"], 100);
    let progress = app.get(&format!("/api/teaching/classes/{c}/progress"), Some(&t)).await;
    assert_eq!(progress.body["students"][0]["cells"][0]["done"], true);

    // Đọc thông báo.
    let unread = app.get("/api/notifications/unread-count", Some(&s1)).await.body["unread"].as_i64().unwrap();
    assert!(unread >= 3);
    let first = app.get("/api/notifications", Some(&s1)).await.body["items"][0]["id"].as_str().unwrap().to_string();
    ok(&app.post(&format!("/api/notifications/{first}/read"), json!({}), Some(&s1)).await, StatusCode::NO_CONTENT);
    assert_eq!(
        app.get("/api/notifications/unread-count", Some(&s1)).await.body["unread"].as_i64().unwrap(),
        unread - 1
    );
    ok(&app.post("/api/notifications/read-all", json!({}), Some(&s1)).await, StatusCode::NO_CONTENT);
    assert_eq!(app.get("/api/notifications/unread-count", Some(&s1)).await.body["unread"], 0);
    // Không đọc được thông báo của người khác.
    assert_eq!(
        app.post(&format!("/api/notifications/{first}/read"), json!({}), Some(&t)).await.status,
        StatusCode::NOT_FOUND
    );
}

trait ClassHelpers {
    async fn patch_class_enrollment(&self, t: &str, class_id: &str, enrollment: &str);
}

impl ClassHelpers for TestApp {
    async fn patch_class_enrollment(&self, t: &str, class_id: &str, enrollment: &str) {
        let current = self.get(&format!("/api/teaching/classes/{class_id}"), Some(t)).await.body;
        let body = json!({
            "subjectId": current["subjectId"], "title": current["title"], "mode": current["mode"],
            "visibility": current["visibility"], "enrollment": enrollment
        });
        ok(&self.put(&format!("/api/teaching/classes/{class_id}"), body, Some(t)).await, StatusCode::OK);
    }
}
