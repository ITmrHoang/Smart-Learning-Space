//! Nói chuyện với LiveKit: ký token vào phòng (JWT HS256) và gọi RoomService (Twirp, JSON) để đổi quyền phát,
//! mời ra, đóng phòng. Không dùng SDK — chỉ `jsonwebtoken` + `reqwest` có sẵn.

use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    config::LiveConfig,
    error::{AppError, AppResult},
};

/// Quyền trong phòng (claim `video` của token LiveKit).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoGrant {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room: Option<String>,
    pub room_join: bool,
    pub room_admin: bool,
    pub room_create: bool,
    pub can_publish: bool,
    pub can_subscribe: bool,
    pub can_publish_data: bool,
    /// Nguồn được phát (camera, microphone, screen_share, screen_share_audio); rỗng + `can_publish` = mọi nguồn.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub can_publish_sources: Vec<&'static str>,
    pub can_update_own_metadata: bool,
}

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,
    sub: &'a str,
    name: &'a str,
    nbf: i64,
    exp: i64,
    /// Vai trò do server đặt (`{"role":"teacher"}`); người tham gia không tự sửa được.
    metadata: &'a str,
    video: &'a VideoGrant,
}

/// Token vào phòng cho một người (`identity` duy nhất trong phòng).
pub fn token(
    cfg: &LiveConfig,
    identity: &str,
    name: &str,
    metadata: &str,
    grant: &VideoGrant,
    ttl_secs: i64,
) -> String {
    let now = Utc::now().timestamp();
    let claims =
        Claims { iss: &cfg.api_key, sub: identity, name, nbf: now - 10, exp: now + ttl_secs, metadata, video: grant };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(cfg.api_secret.as_bytes()))
        .expect("ký token LiveKit (HS256) không lỗi")
}

/// Token cho api gọi RoomService trên một phòng.
fn admin_token(cfg: &LiveConfig, room: &str) -> String {
    let grant = VideoGrant { room: Some(room.to_string()), room_admin: true, room_create: true, ..Default::default() };
    token(cfg, "sls-api", "sls-api", "", &grant, 120)
}

/// Gọi một hàm của RoomService. Phòng / người không tồn tại (404) coi như xong.
pub async fn call(cfg: &LiveConfig, method: &str, room: &str, body: Value) -> AppResult<()> {
    let url = format!("{}/twirp/livekit.RoomService/{method}", cfg.url);
    let res = reqwest::Client::new()
        .post(&url)
        .bearer_auth(admin_token(cfg, room))
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("không gọi được LiveKit {method}: {e}")))?;
    if res.status().is_success() || res.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(());
    }
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    Err(AppError::internal(format!("LiveKit {method} trả {status}: {text}")))
}

/// Đổi quyền phát của một người đang trong phòng (không cần vào lại phòng).
pub async fn set_publish(cfg: &LiveConfig, room: &str, identity: &str, sources: &[&str], data: bool) -> AppResult<()> {
    call(
        cfg,
        "UpdateParticipant",
        room,
        json!({
            "room": room,
            "identity": identity,
            "permission": {
                "canSubscribe": true,
                "canPublish": !sources.is_empty(),
                "canPublishData": data,
                "canPublishSources": sources,
            }
        }),
    )
    .await
}

pub async fn remove(cfg: &LiveConfig, room: &str, identity: &str) -> AppResult<()> {
    call(cfg, "RemoveParticipant", room, json!({ "room": room, "identity": identity })).await
}

/// Đóng phòng: mọi người bị ngắt kết nối.
pub async fn close_room(cfg: &LiveConfig, room: &str) -> AppResult<()> {
    call(cfg, "DeleteRoom", room, json!({ "room": room })).await
}

#[cfg(test)]
mod tests {
    use jsonwebtoken::{DecodingKey, Validation, decode};

    use super::*;

    #[test]
    fn token_dung_dinh_dang_livekit() {
        let cfg = LiveConfig {
            url: "http://127.0.0.1:7880".into(),
            public_url: "ws://127.0.0.1:7880".into(),
            api_key: "sls-dev".into(),
            api_secret: "0123456789abcdef0123456789abcdef".into(),
        };
        let grant = VideoGrant {
            room: Some("session-1".into()),
            room_join: true,
            can_subscribe: true,
            can_publish_data: true,
            ..Default::default()
        };
        let t = token(&cfg, "u:1", "Học sinh A", r#"{"role":"student"}"#, &grant, 3600);
        let mut v = Validation::new(jsonwebtoken::Algorithm::HS256);
        v.set_required_spec_claims(&["exp", "sub", "iss"]);
        let data = decode::<Value>(&t, &DecodingKey::from_secret(cfg.api_secret.as_bytes()), &v).unwrap().claims;
        assert_eq!(data["iss"], "sls-dev");
        assert_eq!(data["sub"], "u:1");
        assert_eq!(data["name"], "Học sinh A");
        assert_eq!(data["metadata"], r#"{"role":"student"}"#);
        assert_eq!(data["video"]["room"], "session-1");
        assert_eq!(data["video"]["roomJoin"], true);
        assert_eq!(data["video"]["canPublish"], false);
        assert_eq!(data["video"]["canPublishData"], true);
        assert!(data["video"].get("canPublishSources").is_none());
    }
}
