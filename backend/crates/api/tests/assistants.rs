//! Đợt 7 (M32): trợ giảng — chủ lớp mời giáo viên khác cùng lo lớp: xem lớp / học sinh / tiến độ / báo cáo,
//! duyệt ghi danh, chấm bài, trả lời hỏi đáp; không sửa được lớp, lộ trình, không mời ra học sinh.

mod common;

use axum::http::StatusCode;
use common::{TestApp, ok, teacher};
use serde_json::json;
use sls_core::modules::users::model::UserRole;
use sqlx::PgPool;

async fn latest_kind(app: &TestApp, token: &str) -> String {
    app.get("/api/notifications", Some(token)).await.body["items"][0]["kind"].as_str().unwrap_or_default().to_string()
}

#[sqlx::test(migrator = "sls_core::db::MIGRATOR")]
async fn tro_giang_quyen_trong_lop(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_, t, toan) = teacher(&app, "gv", "toan").await;
    let (tg_id, tg) = app.login_as("gv_tro_giang", UserRole::Teacher).await;
    let (hs_id, hs) = app.login_as("hs_an", UserRole::Student).await;
    let (_, other_teacher) = app.login_as("gv_khac", UserRole::Teacher).await;

    let res = app
        .post(
            "/api/teaching/classes",
            json!({ "subjectId": toan, "title": "Toán 9A", "mode": "self_paced", "visibility": "public", "enrollment": "approval" }),
            Some(&t),
        )
        .await;
    ok(&res, StatusCode::CREATED);
    let c = res.body["id"].as_str().unwrap().to_string();
    ok(&app.post(&format!("/api/classes/{c}/enroll"), json!({}), Some(&hs)).await, StatusCode::OK);

    // Chỉ chủ lớp mời được; chỉ tài khoản giáo viên làm trợ giảng được
    let url = format!("/api/teaching/classes/{c}/assistants");
    let res = app.post(&url, json!({ "login": "hs_an" }), Some(&t)).await;
    assert_eq!(res.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(res.body["error"]["fields"]["login"][0].as_str().unwrap().contains("giáo viên"));
    assert_eq!(
        app.post(&url, json!({ "login": "gv_tro_giang" }), Some(&other_teacher)).await.status,
        StatusCode::NOT_FOUND
    );
    let res = app.post(&url, json!({ "login": "gv_tro_giang" }), Some(&t)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["userId"], tg_id.to_string());
    assert_eq!(latest_kind(&app, &tg).await, "assistant_added");
    assert_eq!(
        app.post(&url, json!({ "login": "gv_tro_giang" }), Some(&t)).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );

    // Trợ giảng thấy lớp trong "lớp trợ giảng" (không thấy mã mời), không lẫn với lớp của mình
    let mine = app.get("/api/teaching/classes", Some(&tg)).await;
    assert_eq!(mine.body["total"], 0);
    let assisting = app.get("/api/teaching/classes?assisting=true", Some(&tg)).await;
    ok(&assisting, StatusCode::OK);
    assert_eq!(assisting.body["total"], 1);
    assert_eq!(assisting.body["items"][0]["myRole"], "assistant");
    assert_eq!(assisting.body["items"][0]["inviteCode"], "");
    assert!(assisting.body["items"][0]["ownerName"].as_str().is_some());
    let detail = app.get(&format!("/api/teaching/classes/{c}"), Some(&tg)).await;
    ok(&detail, StatusCode::OK);
    assert_eq!(detail.body["myRole"], "assistant");
    assert_eq!(app.get(&format!("/api/teaching/classes/{c}"), Some(&t)).await.body["myRole"], "owner");
    // Học sinh của lớp không tính trợ giảng
    assert_eq!(detail.body["studentCount"], 0);
    assert_eq!(detail.body["pendingCount"], 1);

    // Được: xem học sinh, duyệt ghi danh, tiến độ, báo cáo, bài tập
    let members = app.get(&format!("/api/teaching/classes/{c}/members"), Some(&tg)).await;
    ok(&members, StatusCode::OK);
    assert_eq!(members.body.as_array().unwrap().len(), 1);
    ok(
        &app.post(&format!("/api/teaching/classes/{c}/members/{hs_id}/approve"), json!({}), Some(&tg)).await,
        StatusCode::NO_CONTENT,
    );
    ok(&app.get(&format!("/api/teaching/classes/{c}/progress"), Some(&tg)).await, StatusCode::OK);
    ok(&app.get(&format!("/api/teaching/classes/{c}/report"), Some(&tg)).await, StatusCode::OK);
    ok(&app.get(&format!("/api/teaching/classes/{c}/homework"), Some(&tg)).await, StatusCode::OK);
    ok(&app.get(&url, Some(&tg)).await, StatusCode::OK);

    // Không được: mời ra học sinh, sửa lớp / lộ trình, đổi mã mời, thêm học sinh, mời trợ giảng
    let res = app.post(&format!("/api/teaching/classes/{c}/members/{hs_id}/remove"), json!({}), Some(&tg)).await;
    assert_eq!(res.status, StatusCode::FORBIDDEN);
    assert_eq!(res.code(), "OWNER_ONLY");
    let body = json!({ "subjectId": toan, "title": "Đổi tên", "mode": "self_paced", "visibility": "public", "enrollment": "open" });
    assert_eq!(app.put(&format!("/api/teaching/classes/{c}"), body, Some(&tg)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get(&format!("/api/teaching/classes/{c}/path"), Some(&tg)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        app.post(&format!("/api/teaching/classes/{c}/invite-code"), json!({}), Some(&tg)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.post(&format!("/api/teaching/classes/{c}/members"), json!({ "login": "gv_khac" }), Some(&tg)).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(app.post(&url, json!({ "login": "gv_khac" }), Some(&tg)).await.status, StatusCode::NOT_FOUND);

    // Hỏi đáp: trợ giảng được báo câu hỏi mới, trả lời hiện nhãn giáo viên
    let res =
        app.post(&format!("/api/classes/{c}/threads"), json!({ "title": "Em hỏi bài", "body": "…" }), Some(&hs)).await;
    ok(&res, StatusCode::CREATED);
    let th = res.body["id"].as_str().unwrap().to_string();
    assert_eq!(latest_kind(&app, &tg).await, "question_new");
    assert_eq!(latest_kind(&app, &t).await, "question_new");
    let res = app.post(&format!("/api/threads/{th}/posts"), json!({ "body": "Trợ giảng trả lời." }), Some(&tg)).await;
    ok(&res, StatusCode::CREATED);
    assert_eq!(res.body["posts"][0]["byTeacher"], true);

    // Học sinh không vào lớp bằng cách ghi danh trùng tài khoản trợ giảng; trợ giảng tự rời lớp
    assert_eq!(app.post(&format!("/api/classes/{c}/enroll"), json!({}), Some(&tg)).await.status, StatusCode::CONFLICT);
    let res = app.delete(&format!("/api/teaching/classes/{c}/assistants/{tg_id}"), Some(&tg)).await;
    assert_eq!(res.status, StatusCode::NO_CONTENT);
    assert_eq!(app.get(&format!("/api/teaching/classes/{c}"), Some(&tg)).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get(&url, Some(&t)).await.body.as_array().unwrap().len(), 0);

    // Lớp đã lưu trữ: không mời thêm
    ok(&app.post(&format!("/api/teaching/classes/{c}/archive"), json!({}), Some(&t)).await, StatusCode::OK);
    assert_eq!(app.post(&url, json!({ "login": "gv_tro_giang" }), Some(&t)).await.status, StatusCode::CONFLICT);
}
