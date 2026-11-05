//! `/api/admin/audit-logs` — nhật ký thao tác (M3).

use axum::{Json, Router, extract::State, routing::get};
use sls_core::{
    AppContext, AppResult,
    modules::audit::{
        dto::{AuditListQuery, AuditLogResponse},
        service as audit,
    },
    pagination::Page,
    security::Actor,
};

use crate::extract::QueryParams;

pub fn admin_routes() -> Router<AppContext> {
    Router::new().route("/audit-logs", get(list))
}

async fn list(
    State(ctx): State<AppContext>,
    actor: Actor,
    QueryParams(q): QueryParams<AuditListQuery>,
) -> AppResult<Json<Page<AuditLogResponse>>> {
    Ok(Json(audit::list(&ctx, &actor, q).await?))
}
