//! Client OIDC tối giản cho Google / Microsoft (luồng Authorization Code + PKCE):
//! đọc cấu hình `/.well-known/openid-configuration`, tạo URL đăng nhập, đổi `code` lấy token,
//! kiểm tra `id_token` (chữ ký qua JWKS, `iss`, `aud`, `exp`, `nonce`).
//!
//! Cấu hình và khóa công khai (JWKS) được giữ trong bộ nhớ 1 giờ; gặp `kid` lạ (nhà cung cấp vừa đổi khóa)
//! thì tải lại JWKS, tối đa một lần mỗi phút.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header,
    jwk::{Jwk, JwkSet},
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use super::model::{ProviderIdentity, SsoProvider};
use crate::{
    config::{Config, SsoProviderConfig},
    error::{AppError, AppResult},
};

const CACHE_TTL: Duration = Duration::from_secs(3600);
const JWKS_REFRESH_MIN_INTERVAL: Duration = Duration::from_secs(60);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
/// Tenant của tài khoản Microsoft cá nhân (outlook.com, hotmail.com…): email do Microsoft xác minh.
const MS_CONSUMER_TENANT: &str = "9188040d-6c67-4c5b-b112-36a304b66dad";

#[derive(Debug, Deserialize)]
struct Metadata {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

struct Cache {
    meta: Arc<Metadata>,
    jwks: Arc<JwkSet>,
    fetched_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct RawJwks {
    keys: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    sub: String,
    iss: String,
    nonce: Option<String>,
    email: Option<String>,
    email_verified: Option<Value>,
    name: Option<String>,
    /// Microsoft: tenant của tài khoản.
    tid: Option<String>,
    /// Microsoft: email thuộc tên miền mà tenant đã xác minh (optional claim, cần bật trong Entra).
    xms_edov: Option<Value>,
}

/// `true`, `"true"` → true. Google từng trả chuỗi thay vì boolean.
fn truthy(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

/// `code_challenge` của PKCE (phương thức S256).
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub struct OidcClient {
    provider: SsoProvider,
    config: SsoProviderConfig,
    redirect_uri: String,
    http: reqwest::Client,
    cache: RwLock<Option<Cache>>,
}

impl OidcClient {
    fn new(provider: SsoProvider, config: SsoProviderConfig, redirect_base: &str, http: reqwest::Client) -> Self {
        let redirect_uri = format!("{redirect_base}/api/auth/sso/{provider}/callback");
        Self { provider, config, redirect_uri, http, cache: RwLock::new(None) }
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    fn failed(&self, detail: impl std::fmt::Display) -> AppError {
        tracing::warn!(provider = %self.provider, error = %detail, "đăng nhập SSO thất bại");
        AppError::bad_request(
            "SSO_FAILED",
            format!("Không đăng nhập được bằng {}. Vui lòng thử lại.", self.provider.label()),
        )
    }

    fn unavailable(&self, detail: impl std::fmt::Display) -> AppError {
        tracing::error!(provider = %self.provider, error = %detail, "không kết nối được nhà cung cấp SSO");
        AppError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "SSO_UNAVAILABLE",
            format!("Không kết nối được tới {}. Vui lòng thử lại sau.", self.provider.label()),
        )
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> AppResult<T> {
        let res = self.http.get(url).send().await.map_err(|e| self.unavailable(e))?;
        if !res.status().is_success() {
            return Err(self.unavailable(format!("GET {url} → {}", res.status())));
        }
        res.json::<T>().await.map_err(|e| self.unavailable(e))
    }

    /// Bỏ qua khóa không đọc được (ví dụ khóa mã hóa) thay vì làm hỏng cả bộ khóa.
    async fn fetch_jwks(&self, uri: &str) -> AppResult<JwkSet> {
        let raw: RawJwks = self.get_json(uri).await?;
        let keys = raw.keys.into_iter().filter_map(|k| serde_json::from_value::<Jwk>(k).ok()).collect();
        Ok(JwkSet { keys })
    }

    /// Cấu hình + JWKS (từ bộ nhớ nếu còn hạn). `refresh_jwks = true` → tải lại JWKS.
    async fn load(&self, refresh_jwks: bool) -> AppResult<(Arc<Metadata>, Arc<JwkSet>)> {
        {
            let cache = self.cache.read().await;
            if let Some(c) = cache.as_ref() {
                let fresh = c.fetched_at.elapsed() < CACHE_TTL;
                let recently_refetched = c.fetched_at.elapsed() < JWKS_REFRESH_MIN_INTERVAL;
                if fresh && (!refresh_jwks || recently_refetched) {
                    return Ok((c.meta.clone(), c.jwks.clone()));
                }
            }
        }
        let mut cache = self.cache.write().await;
        let meta = match cache.as_ref() {
            Some(c) if c.fetched_at.elapsed() < CACHE_TTL => c.meta.clone(),
            _ => {
                let url = format!("{}/.well-known/openid-configuration", self.config.issuer);
                Arc::new(self.get_json::<Metadata>(&url).await?)
            }
        };
        let jwks = Arc::new(self.fetch_jwks(&meta.jwks_uri).await?);
        *cache = Some(Cache { meta: meta.clone(), jwks: jwks.clone(), fetched_at: Instant::now() });
        Ok((meta, jwks))
    }

    /// URL chuyển người dùng tới trang đăng nhập của nhà cung cấp.
    pub async fn authorize_url(&self, state: &str, nonce: &str, code_challenge: &str) -> AppResult<String> {
        let (meta, _) = self.load(false).await?;
        let url = reqwest::Url::parse_with_params(
            &meta.authorization_endpoint,
            [
                ("client_id", self.config.client_id.as_str()),
                ("response_type", "code"),
                ("scope", "openid email profile"),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("state", state),
                ("nonce", nonce),
                ("code_challenge", code_challenge),
                ("code_challenge_method", "S256"),
                ("prompt", "select_account"),
            ],
        )
        .map_err(|e| self.unavailable(e))?;
        Ok(url.into())
    }

    /// Đổi `code` lấy token rồi kiểm tra `id_token`.
    pub async fn exchange(&self, code: &str, code_verifier: &str, nonce: &str) -> AppResult<ProviderIdentity> {
        let (meta, _) = self.load(false).await?;
        let res = self
            .http
            .post(&meta.token_endpoint)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await
            .map_err(|e| self.unavailable(e))?;
        if !res.status().is_success() {
            let status = res.status();
            let body: String = res.text().await.unwrap_or_default().chars().take(500).collect();
            return Err(self.failed(format!("đổi code thất bại: {status} {body}")));
        }
        let token: TokenResponse = res.json().await.map_err(|e| self.failed(e))?;
        let id_token = token.id_token.ok_or_else(|| self.failed("không có id_token"))?;
        self.verify_id_token(&id_token, nonce).await
    }

    async fn verify_id_token(&self, id_token: &str, nonce: &str) -> AppResult<ProviderIdentity> {
        let header = decode_header(id_token).map_err(|e| self.failed(e))?;
        if header.alg != Algorithm::RS256 {
            return Err(self.failed(format!("thuật toán ký không hỗ trợ: {:?}", header.alg)));
        }
        let kid = header.kid.ok_or_else(|| self.failed("id_token không có kid"))?;
        let (meta, jwks) = self.load(false).await?;
        let jwk = match jwks.find(&kid) {
            Some(jwk) => jwk.clone(),
            None => {
                let (_, jwks) = self.load(true).await?;
                jwks.find(&kid).cloned().ok_or_else(|| self.failed(format!("không có khóa {kid}")))?
            }
        };
        let key = DecodingKey::from_jwk(&jwk).map_err(|e| self.failed(e))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.config.client_id]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.leeway = 60;
        let claims = decode::<IdTokenClaims>(id_token, &key, &validation).map_err(|e| self.failed(e))?.claims;

        // Microsoft "common" / "organizations": issuer có dạng `…/{tenantid}/v2.0`.
        let expected_iss = match &claims.tid {
            Some(tid) => meta.issuer.replace("{tenantid}", tid),
            None => meta.issuer.clone(),
        };
        let iss_ok =
            claims.iss == expected_iss || (self.provider == SsoProvider::Google && claims.iss == "accounts.google.com");
        if !iss_ok {
            return Err(self.failed(format!("iss sai: {}", claims.iss)));
        }
        if claims.nonce.as_deref() != Some(nonce) {
            return Err(self.failed("nonce không khớp"));
        }

        let email_verified = truthy(claims.email_verified.as_ref())
            || (self.provider == SsoProvider::Microsoft
                && (truthy(claims.xms_edov.as_ref()) || claims.tid.as_deref() == Some(MS_CONSUMER_TENANT)));
        Ok(ProviderIdentity {
            provider: self.provider,
            subject: claims.sub,
            email: claims.email,
            email_verified,
            full_name: claims.name,
        })
    }
}

/// Các nhà cung cấp đã cấu hình (giữ trong `AppContext`).
pub struct SsoClients {
    google: Option<OidcClient>,
    microsoft: Option<OidcClient>,
}

impl SsoClients {
    pub fn from_config(config: &Config) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent("smart-learning-space")
            .build()
            .expect("tạo HTTP client");
        let make = |provider, cfg: &Option<SsoProviderConfig>| {
            cfg.clone().map(|c| OidcClient::new(provider, c, &config.sso_redirect_base, http.clone()))
        };
        Self {
            google: make(SsoProvider::Google, &config.sso_google),
            microsoft: make(SsoProvider::Microsoft, &config.sso_microsoft),
        }
    }

    pub fn get(&self, provider: SsoProvider) -> Option<&OidcClient> {
        match provider {
            SsoProvider::Google => self.google.as_ref(),
            SsoProvider::Microsoft => self.microsoft.as_ref(),
        }
    }

    pub fn enabled(&self) -> Vec<SsoProvider> {
        [SsoProvider::Google, SsoProvider::Microsoft].into_iter().filter(|p| self.get(*p).is_some()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_theo_rfc7636() {
        // Ví dụ ở RFC 7636, phụ lục B.
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn doc_co_email_verified() {
        assert!(truthy(Some(&Value::Bool(true))));
        assert!(truthy(Some(&Value::String("true".into()))));
        assert!(!truthy(Some(&Value::String("false".into()))));
        assert!(!truthy(None));
    }
}
