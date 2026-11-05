//! Access token: JWT ký HS256, sống ngắn (mặc định 15 phút).

use std::time::Duration;

use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::ErrorKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    modules::users::model::UserRole,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    /// ID người dùng.
    pub sub: Uuid,
    pub role: UserRole,
    /// ID chuỗi phiên (family của refresh token) — dùng để "đăng xuất các thiết bị khác".
    pub sid: Uuid,
    pub iat: i64,
    pub exp: i64,
}

pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    ttl: Duration,
}

impl JwtKeys {
    pub fn new(secret: &[u8], ttl: Duration) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 5;
        validation.set_required_spec_claims(&["exp", "sub"]);
        Self { encoding: EncodingKey::from_secret(secret), decoding: DecodingKey::from_secret(secret), validation, ttl }
    }

    pub fn ttl_secs(&self) -> i64 {
        self.ttl.as_secs() as i64
    }

    pub fn issue(&self, user_id: Uuid, role: UserRole, session_id: Uuid) -> AppResult<String> {
        let now = Utc::now().timestamp();
        self.encode(&Claims { sub: user_id, role, sid: session_id, iat: now, exp: now + self.ttl_secs() })
    }

    pub fn encode(&self, claims: &Claims) -> AppResult<String> {
        encode(&Header::new(Algorithm::HS256), claims, &self.encoding).map_err(AppError::internal)
    }

    pub fn verify(&self, token: &str) -> AppResult<Claims> {
        decode::<Claims>(token, &self.decoding, &self.validation).map(|data| data.claims).map_err(|e| match e.kind() {
            ErrorKind::ExpiredSignature => AppError::unauthorized("TOKEN_EXPIRED", "Phiên đăng nhập đã hết hạn."),
            _ => AppError::unauthorized("INVALID_TOKEN", "Token không hợp lệ."),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys() -> JwtKeys {
        JwtKeys::new(b"0123456789abcdef0123456789abcdef", Duration::from_secs(900))
    }

    #[test]
    fn tao_va_doc_token() {
        let k = keys();
        let (uid, sid) = (Uuid::now_v7(), Uuid::now_v7());
        let token = k.issue(uid, UserRole::Teacher, sid).unwrap();
        let claims = k.verify(&token).unwrap();
        assert_eq!((claims.sub, claims.role, claims.sid), (uid, UserRole::Teacher, sid));
        assert_eq!(claims.exp - claims.iat, 900);
    }

    #[test]
    fn token_het_han_bi_tu_choi() {
        let k = keys();
        let now = Utc::now().timestamp();
        let token = k
            .encode(&Claims {
                sub: Uuid::now_v7(),
                role: UserRole::Student,
                sid: Uuid::now_v7(),
                iat: now - 1000,
                exp: now - 100,
            })
            .unwrap();
        assert_eq!(k.verify(&token).unwrap_err().code, "TOKEN_EXPIRED");
    }

    #[test]
    fn token_sai_chu_ky_bi_tu_choi() {
        let other = JwtKeys::new(b"ffffffffffffffffffffffffffffffff", Duration::from_secs(900));
        let token = other.issue(Uuid::now_v7(), UserRole::Admin, Uuid::now_v7()).unwrap();
        assert_eq!(keys().verify(&token).unwrap_err().code, "INVALID_TOKEN");
        assert_eq!(keys().verify("abc.def.ghi").unwrap_err().code, "INVALID_TOKEN");
    }
}
