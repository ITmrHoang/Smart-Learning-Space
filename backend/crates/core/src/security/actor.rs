//! `Actor`: người dùng đang gọi API, đọc từ header `Authorization: Bearer <access token>`.
//!
//! Dùng làm tham số của handler:
//! - `actor: Actor` — bắt buộc đăng nhập (không có token → 401).
//! - `actor: Option<Actor>` — không bắt buộc; có token mà sai thì vẫn 401.

use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts},
    http::{HeaderMap, header, request::Parts},
};
use uuid::Uuid;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::users::model::UserRole,
    security::permissions::{Permission, has_permission},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Actor {
    pub id: Uuid,
    pub role: UserRole,
    /// ID chuỗi phiên đăng nhập hiện tại.
    pub session_id: Uuid,
}

impl Actor {
    pub fn can(&self, permission: Permission) -> bool {
        has_permission(self.role, permission)
    }

    pub fn require(&self, permission: Permission) -> AppResult<()> {
        if self.can(permission) {
            Ok(())
        } else {
            Err(AppError::forbidden("FORBIDDEN", "Bạn không có quyền thực hiện thao tác này."))
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == UserRole::Admin
    }

    /// Đọc và kiểm tra access token trong header. `Ok(None)` nếu không có header.
    pub fn from_headers(ctx: &AppContext, headers: &HeaderMap) -> AppResult<Option<Self>> {
        let Some(value) = headers.get(header::AUTHORIZATION) else {
            return Ok(None);
        };
        let token = value
            .to_str()
            .ok()
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::unauthorized("INVALID_TOKEN", "Token không hợp lệ."))?;
        let claims = ctx.jwt.verify(token.trim())?;
        Ok(Some(Self { id: claims.sub, role: claims.role, session_id: claims.sid }))
    }
}

fn missing() -> AppError {
    AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập.")
}

impl<S> FromRequestParts<S> for Actor
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        if let Some(actor) = parts.extensions.get::<Actor>() {
            return Ok(*actor);
        }
        let ctx = AppContext::from_ref(state);
        let actor = Self::from_headers(&ctx, &parts.headers)?.ok_or_else(missing)?;
        parts.extensions.insert(actor);
        Ok(actor)
    }
}

impl<S> OptionalFromRequestParts<S> for Actor
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>, Self::Rejection> {
        if let Some(actor) = parts.extensions.get::<Actor>() {
            return Ok(Some(*actor));
        }
        let ctx = AppContext::from_ref(state);
        Self::from_headers(&ctx, &parts.headers)
    }
}
