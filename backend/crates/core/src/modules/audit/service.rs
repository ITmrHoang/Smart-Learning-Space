use sqlx::PgExecutor;

use super::{
    dto::{AuditListQuery, AuditLogResponse},
    model::AuditEntry,
    repository::{self, AuditFilter},
};
use crate::{
    context::AppContext,
    error::AppResult,
    pagination::{Page, PageRequest},
    security::{Actor, Permission},
};

/// Ghi một dòng nhật ký. Truyền `&mut *tx` để ghi cùng transaction với thao tác chính.
pub async fn record<'e, E: PgExecutor<'e>>(db: E, entry: AuditEntry<'_>) -> AppResult<()> {
    repository::insert(db, &entry).await?;
    Ok(())
}

pub async fn list(ctx: &AppContext, actor: &Actor, q: AuditListQuery) -> AppResult<Page<AuditLogResponse>> {
    actor.require(Permission::AuditRead)?;
    let page = PageRequest::new(q.page, q.page_size);
    let filter =
        AuditFilter { action: q.action.as_deref(), target_type: q.target_type.as_deref(), target_id: q.target_id };
    let total = repository::count(&ctx.db, &filter).await?;
    let rows = repository::list(&ctx.db, &filter, page.limit(), page.offset()).await?;
    Ok(Page::new(rows.into_iter().map(Into::into).collect(), total, page))
}
