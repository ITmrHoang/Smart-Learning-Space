//! Middleware riêng của API.
//!
//! Các middleware nền (request id, log, CORS, timeout, giới hạn body) là layer của `tower-http`,
//! ghép ở `app.rs`. Xác thực từng route dùng extractor `Actor`; giới hạn tần suất nằm trong service.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sls_core::{
    AppContext, AppError,
    security::{Actor, Permission},
};

/// Chặn toàn bộ nhóm route `/api/admin/*`: phải đăng nhập và có quyền `admin.access`.
/// Từng handler vẫn kiểm tra quyền chi tiết (`users.manage`…) trong service.
pub async fn require_admin(State(ctx): State<AppContext>, mut req: Request, next: Next) -> Result<Response, AppError> {
    let actor = Actor::from_headers(&ctx, req.headers())?
        .ok_or_else(|| AppError::unauthorized("UNAUTHENTICATED", "Bạn cần đăng nhập."))?;
    actor.require(Permission::AdminAccess)?;
    req.extensions_mut().insert(actor);
    Ok(next.run(req).await)
}
