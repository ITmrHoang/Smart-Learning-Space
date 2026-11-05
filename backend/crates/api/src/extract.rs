//! Extractor dùng chung cho controller. Mọi lỗi đều trả về JSON `AppError` thống nhất
//! (extractor gốc của Axum trả lỗi dạng văn bản).

use std::{convert::Infallible, net::SocketAddr};

use axum::{
    Json,
    extract::{ConnectInfo, FromRef, FromRequest, FromRequestParts, Path, Query, Request},
    http::{header, request::Parts},
};
use serde::de::DeserializeOwned;
use sls_core::{AppContext, AppError, modules::auth::service::ClientInfo};
use validator::Validate;

/// Body JSON đã được validate (`#[derive(Validate)]`).
pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(|rej| {
            AppError::bad_request("INVALID_JSON", format!("Dữ liệu gửi lên không hợp lệ: {}", rej.body_text()))
        })?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Query string (`?page=1&pageSize=20`).
pub struct QueryParams<T>(pub T);

impl<S, T> FromRequestParts<S> for QueryParams<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(v)| Self(v))
            .map_err(|rej| AppError::bad_request("INVALID_QUERY", format!("Tham số không hợp lệ: {}", rej.body_text())))
    }
}

/// Tham số trên đường dẫn (`/users/{id}`). Sai định dạng (ví dụ UUID hỏng) → 404.
pub struct PathParam<T>(pub T);

impl<S, T> FromRequestParts<S> for PathParam<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Path::<T>::from_request_parts(parts, state)
            .await
            .map(|Path(v)| Self(v))
            .map_err(|_| AppError::not_found("Không tìm thấy."))
    }
}

/// IP và User-Agent của người gọi. Sau Caddy (`TRUST_PROXY=true`) thì lấy IP từ `X-Forwarded-For`.
pub struct Client(pub ClientInfo);

impl<S> FromRequestParts<S> for Client
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx = AppContext::from_ref(state);
        let forwarded = ctx
            .config
            .trust_proxy
            .then(|| parts.headers.get("x-forwarded-for"))
            .flatten()
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|ip| ip.trim().to_string())
            .filter(|ip| !ip.is_empty());
        let ip = forwarded.or_else(|| {
            parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|ConnectInfo(addr)| addr.ip().to_string())
        });
        let user_agent = parts
            .headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|ua| ua.chars().take(300).collect());
        Ok(Self(ClientInfo { ip, user_agent }))
    }
}
