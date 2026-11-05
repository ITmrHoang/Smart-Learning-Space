//! Nhà cung cấp OIDC giả chạy trong test (thay Google / Microsoft): cấu hình discovery, JWKS, token endpoint.
//! Khóa RSA ở `tests/fixtures/` chỉ dùng cho test.
//!
//! - `{base}/.well-known/openid-configuration`: kiểu Google (issuer = `{base}`).
//! - `{base}/common/v2.0/.well-known/openid-configuration`: kiểu Microsoft "common"
//!   (issuer = `{base}/{tenantid}/v2.0`, token mang claim `tid`).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    Form, Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use sls_core::{config::SsoProviderConfig, modules::sso::oidc::pkce_challenge};

pub const CLIENT_ID: &str = "test-client";
pub const CLIENT_SECRET: &str = "test-secret";
pub const KID: &str = "test-key";
/// Tenant tài khoản Microsoft cá nhân.
pub const MS_CONSUMER_TENANT: &str = "9188040d-6c67-4c5b-b112-36a304b66dad";

const KEY_DER: &[u8] = include_bytes!("../fixtures/oidc_test_key.der");
const KEY_N: &str = include_str!("../fixtures/oidc_test_key.n");

struct Pending {
    id_token: String,
    challenge: String,
}

#[derive(Clone)]
struct FakeState {
    base: String,
    codes: Arc<Mutex<HashMap<String, Pending>>>,
}

pub struct FakeOidc {
    pub base: String,
    state: FakeState,
    counter: Mutex<u32>,
}

/// Các tham số trên URL đăng nhập mà backend tạo ra.
#[derive(Debug, Clone)]
pub struct AuthorizeRequest {
    pub params: HashMap<String, String>,
}

impl AuthorizeRequest {
    pub fn parse(url: &str) -> Self {
        let query = url.split_once('?').map(|(_, q)| q).unwrap_or_default();
        let params = query
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .map(|(k, v)| (k.to_string(), percent_decode(v)))
            .collect();
        Self { params }
    }

    pub fn get(&self, key: &str) -> &str {
        self.params.get(key).map(String::as_str).unwrap_or("")
    }
}

fn percent_decode(s: &str) -> String {
    let bytes = s.replace('+', " ").into_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("zz"), 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap()
}

impl FakeOidc {
    pub async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let state = FakeState { base: base.clone(), codes: Arc::default() };
        let app = Router::new()
            .route("/.well-known/openid-configuration", get(google_discovery))
            .route("/common/v2.0/.well-known/openid-configuration", get(microsoft_discovery))
            .route("/jwks", get(jwks))
            .route("/token", post(token))
            .with_state(state.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self { base, state, counter: Mutex::new(0) }
    }

    pub fn google_config(&self) -> SsoProviderConfig {
        SsoProviderConfig {
            client_id: CLIENT_ID.into(),
            client_secret: CLIENT_SECRET.into(),
            issuer: self.base.clone(),
        }
    }

    pub fn microsoft_config(&self) -> SsoProviderConfig {
        SsoProviderConfig {
            client_id: CLIENT_ID.into(),
            client_secret: CLIENT_SECRET.into(),
            issuer: format!("{}/common/v2.0", self.base),
        }
    }

    /// Claim chuẩn của id_token kiểu Google cho một người dùng.
    pub fn google_claims(&self, sub: &str, email: &str) -> Value {
        let now = Utc::now().timestamp();
        json!({
            "iss": self.base, "aud": CLIENT_ID, "sub": sub, "iat": now, "exp": now + 3600,
            "email": email, "email_verified": true, "name": "Người Dùng SSO",
        })
    }

    /// Claim kiểu Microsoft (không có `email_verified`; xác minh dựa vào `tid` / `xms_edov`).
    pub fn microsoft_claims(&self, sub: &str, email: &str, tid: &str) -> Value {
        let now = Utc::now().timestamp();
        json!({
            "iss": format!("{}/{tid}/v2.0", self.base), "aud": CLIENT_ID, "sub": sub, "tid": tid,
            "iat": now, "exp": now + 3600, "email": email, "name": "Microsoft User",
        })
    }

    pub fn sign(claims: &Value) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(KID.into());
        encode(&header, claims, &EncodingKey::from_rsa_der(KEY_DER)).unwrap()
    }

    /// Giả lập người dùng đăng nhập thành công ở nhà cung cấp: tạo `code` gắn với id_token (thêm `nonce`
    /// lấy từ URL đăng nhập, trừ khi `claims` đã có sẵn `nonce`). Trả về `(code, state)`.
    pub fn authorize(&self, authorize_url: &str, mut claims: Value) -> (String, String) {
        let req = AuthorizeRequest::parse(authorize_url);
        if claims.get("nonce").is_none() {
            claims["nonce"] = json!(req.get("nonce"));
        }
        self.authorize_raw(&req, Self::sign(&claims))
    }

    /// Như `authorize` nhưng dùng nguyên id_token cho sẵn (để test chữ ký sai…).
    pub fn authorize_raw(&self, req: &AuthorizeRequest, id_token: String) -> (String, String) {
        let mut n = self.counter.lock().unwrap();
        *n += 1;
        let code = format!("code-{n}");
        self.state
            .codes
            .lock()
            .unwrap()
            .insert(code.clone(), Pending { id_token, challenge: req.get("code_challenge").to_string() });
        (code, req.get("state").to_string())
    }
}

async fn google_discovery(State(s): State<FakeState>) -> Json<Value> {
    Json(json!({
        "issuer": s.base,
        "authorization_endpoint": format!("{}/authorize", s.base),
        "token_endpoint": format!("{}/token", s.base),
        "jwks_uri": format!("{}/jwks", s.base),
    }))
}

async fn microsoft_discovery(State(s): State<FakeState>) -> Json<Value> {
    Json(json!({
        "issuer": format!("{}/{{tenantid}}/v2.0", s.base),
        "authorization_endpoint": format!("{}/authorize", s.base),
        "token_endpoint": format!("{}/token", s.base),
        "jwks_uri": format!("{}/jwks", s.base),
    }))
}

async fn jwks() -> Json<Value> {
    Json(json!({ "keys": [
        // Khóa mã hóa (không phải khóa ký) — backend phải bỏ qua chứ không lỗi cả bộ khóa.
        { "kty": "RSA", "use": "enc", "alg": "RSA-OAEP-256", "kid": "enc-key", "n": KEY_N, "e": "AQAB" },
        { "kty": "RSA", "use": "sig", "alg": "RS256", "kid": KID, "n": KEY_N, "e": "AQAB" },
    ]}))
}

/// Token endpoint: kiểm tra client, PKCE (S256) và `code` dùng một lần như nhà cung cấp thật.
async fn token(State(s): State<FakeState>, Form(form): Form<HashMap<String, String>>) -> (StatusCode, Json<Value>) {
    let field = |k: &str| form.get(k).map(String::as_str).unwrap_or("");
    let bad = |e: &str| (StatusCode::BAD_REQUEST, Json(json!({ "error": e })));
    if field("grant_type") != "authorization_code" {
        return bad("unsupported_grant_type");
    }
    if field("client_id") != CLIENT_ID || field("client_secret") != CLIENT_SECRET {
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "invalid_client" })));
    }
    if !field("redirect_uri").ends_with("/callback") {
        return bad("invalid_redirect_uri");
    }
    let Some(pending) = s.codes.lock().unwrap().remove(field("code")) else {
        return bad("invalid_grant");
    };
    if pkce_challenge(field("code_verifier")) != pending.challenge {
        return bad("invalid_grant");
    }
    (StatusCode::OK, Json(json!({ "id_token": pending.id_token, "access_token": "x", "token_type": "Bearer" })))
}
