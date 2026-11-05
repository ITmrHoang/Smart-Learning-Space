//! Nghiệp vụ ngân hàng câu hỏi. Xem mô tả ở `mod.rs`.

use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{
        BulkCreateRequest, BulkCreateResponse, FolderInput, FolderResponse, ImportPreview, QuestionInput,
        QuestionQuery, QuestionResponse, RegradeResponse, SaveQuestionResponse,
    },
    import,
    model::{QOption, QuestionBody, QuestionKind, QuestionRow, QuestionVersion},
    repository::{self as repo, QuestionFields, QuestionFilter},
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{catalog::service as catalog, exams::service as exams, teachers::service as teachers},
    pagination::{Page, PageRequest},
    security::{Actor, Permission},
    slug::fold,
};

const MAX_OPTIONS: usize = 8;

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy câu hỏi.")
}

async fn load_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<QuestionRow> {
    actor.require(Permission::TeachingManage)?;
    let q = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if q.owner_id != actor.id || q.archived_at.is_some() {
        return Err(not_found());
    }
    Ok(q)
}

/// Kiểm tra cấu trúc theo dạng câu và dựng nội dung phiên bản. Giữ ID phương án cũ nếu có trong `previous`.
pub fn build_body(input: &QuestionInput, previous: Option<&QuestionBody>) -> AppResult<QuestionBody> {
    input.validate()?;
    let n = input.options.len();
    let correct_count = input.options.iter().filter(|o| o.correct).count();
    match input.kind {
        QuestionKind::Single | QuestionKind::Multiple if !(2..=MAX_OPTIONS).contains(&n) => {
            return Err(AppError::field("options", format!("Cần từ 2 đến {MAX_OPTIONS} phương án.")));
        }
        QuestionKind::Single if correct_count != 1 => {
            return Err(AppError::field("options", "Câu một đáp án phải có đúng 1 phương án đúng."));
        }
        QuestionKind::Multiple if correct_count == 0 => {
            return Err(AppError::field("options", "Chọn ít nhất 1 phương án đúng."));
        }
        QuestionKind::TrueFalse if !(2..=6).contains(&n) => {
            return Err(AppError::field("options", "Câu đúng/sai cần từ 2 đến 6 ý (thường là 4 ý a, b, c, d)."));
        }
        QuestionKind::Short if n > 0 => {
            return Err(AppError::field("options", "Câu trả lời ngắn không có phương án."));
        }
        _ => {}
    }
    let accepted: Vec<String> = input.accepted.iter().map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
    if input.kind == QuestionKind::Short
        && (accepted.is_empty() || accepted.len() > 10 || accepted.iter().any(|a| a.chars().count() > 200))
    {
        return Err(AppError::field("accepted", "Nhập từ 1 đến 10 đáp án chấp nhận, mỗi đáp án tối đa 200 ký tự."));
    }

    // Giữ ID cũ (chỉ khi ID thật sự thuộc phiên bản trước, không trùng) để chấm theo ID qua các phiên bản.
    let old_ids: Vec<Uuid> = previous.map(|p| p.options.iter().map(|o| o.id).collect()).unwrap_or_default();
    let mut used = Vec::new();
    let mut options = Vec::with_capacity(n);
    let mut correct = Vec::new();
    for o in &input.options {
        let id = match o.id {
            Some(id) if old_ids.contains(&id) && !used.contains(&id) => id,
            _ => Uuid::now_v7(),
        };
        used.push(id);
        if o.correct {
            correct.push(id);
        }
        options.push(QOption { id, content: o.content.trim().to_string() });
    }
    Ok(QuestionBody {
        content: input.content.trim().to_string(),
        options,
        correct,
        accepted: if input.kind == QuestionKind::Short { accepted } else { Vec::new() },
        explanation: input.explanation.trim().to_string(),
        shuffle_options: input.shuffle_options,
    })
}

fn search_text(body: &QuestionBody) -> String {
    let mut s = body.content.clone();
    for o in &body.options {
        s.push(' ');
        s.push_str(&o.content);
    }
    fold(&s).chars().take(4000).collect()
}

async fn validate_meta(ctx: &AppContext, actor: &Actor, input: &QuestionInput) -> AppResult<()> {
    teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    if let Some(grade) = input.grade_id {
        catalog::ensure_grades_visible(ctx, &[grade], actor.id)
            .await
            .map_err(|_| AppError::field("gradeId", "Lớp không tồn tại."))?;
    }
    if let Some(folder) = input.folder_id
        && repo::folder_owner(&ctx.db, folder).await? != Some(actor.id)
    {
        return Err(AppError::field("folderId", "Thư mục không tồn tại."));
    }
    catalog::ensure_tags_visible(ctx, input.subject_id, &input.tag_ids, actor.id).await
}

async fn insert_question(
    conn: &mut sqlx::PgConnection,
    actor: &Actor,
    input: &QuestionInput,
    body: &QuestionBody,
) -> AppResult<Uuid> {
    let search = search_text(body);
    let fields = QuestionFields {
        subject_id: input.subject_id,
        grade_id: input.grade_id,
        folder_id: input.folder_id,
        kind: input.kind.as_str(),
        level: input.level.as_str(),
        search_text: &search,
    };
    let id = repo::insert(&mut *conn, actor.id, &fields).await?;
    repo::add_version(&mut *conn, id, body, actor.id).await?;
    repo::set_tags(&mut *conn, id, &input.tag_ids).await?;
    Ok(id)
}

pub async fn list(ctx: &AppContext, actor: &Actor, query: QuestionQuery) -> AppResult<Page<QuestionResponse>> {
    actor.require(Permission::TeachingManage)?;
    let page = PageRequest::new(query.page, query.page_size);
    let folder = match query.folder_id.as_deref() {
        None | Some("") => None,
        Some("none") => Some(None),
        Some(id) => Some(Some(id.parse::<Uuid>().map_err(|_| AppError::field("folderId", "Thư mục không hợp lệ."))?)),
    };
    let filter = QuestionFilter {
        owner_id: actor.id,
        q: query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()),
        subject_id: query.subject_id,
        folder_id: folder,
        tag_id: query.tag_id,
        level: query.level.map(|l| l.as_str()),
        kind: query.kind.map(QuestionKind::as_str),
        exclude: &[],
    };
    let items = repo::list(&ctx.db, &filter, page.limit(), page.offset()).await?;
    let total = repo::count(&ctx.db, &filter).await?;
    Ok(Page::new(items.into_iter().map(Into::into).collect(), total, page))
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<QuestionResponse> {
    Ok(load_own(ctx, actor, id).await?.into())
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: QuestionInput) -> AppResult<QuestionResponse> {
    actor.require(Permission::TeachingManage)?;
    let body = build_body(&input, None)?;
    validate_meta(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    let id = insert_question(&mut tx, actor, &input, &body).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

/// Sửa câu hỏi → phiên bản mới. Báo đáp án đúng có đổi không và số bài đã nộp bị ảnh hưởng.
pub async fn update(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    input: QuestionInput,
) -> AppResult<SaveQuestionResponse> {
    let current = load_own(ctx, actor, id).await?;
    let old = current.body.0.clone();
    let old_kind: QuestionKind = current.kind.parse().map_err(|_| AppError::internal("dạng câu lạ"))?;
    if old_kind != input.kind && current.used_count > 0 {
        return Err(AppError::conflict(
            "QUESTION_IN_USE",
            "Câu hỏi đã có học sinh làm nên không đổi được dạng câu. Hãy tạo câu mới.",
        ));
    }
    let body = build_body(&input, Some(&old))?;
    validate_meta(ctx, actor, &input).await?;

    let mut old_correct = old.correct.clone();
    old_correct.sort();
    let mut new_correct = body.correct.clone();
    new_correct.sort();
    let answer_changed = old_correct != new_correct || old.accepted != body.accepted;

    let mut tx = ctx.db.begin().await?;
    let search = search_text(&body);
    let fields = QuestionFields {
        subject_id: input.subject_id,
        grade_id: input.grade_id,
        folder_id: input.folder_id,
        kind: input.kind.as_str(),
        level: input.level.as_str(),
        search_text: &search,
    };
    repo::update(&mut tx, id, &fields).await?;
    if body != old {
        repo::add_version(&mut tx, id, &body, actor.id).await?;
    }
    repo::set_tags(&mut tx, id, &input.tag_ids).await?;
    tx.commit().await?;

    let affected_attempts = if answer_changed { repo::count_submitted_attempts(&ctx.db, id).await? } else { 0 };
    Ok(SaveQuestionResponse { question: get(ctx, actor, id).await?, answer_changed, affected_attempts })
}

/// Ẩn câu khỏi ngân hàng (đề đang dùng cố định câu này vẫn giữ; ma trận không rút câu này nữa).
pub async fn archive(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    load_own(ctx, actor, id).await?;
    repo::archive(&ctx.db, id).await?;
    Ok(())
}

/// Chấm lại mọi bài đã nộp có câu này theo đáp án hiện tại.
pub async fn regrade(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<RegradeResponse> {
    let q = load_own(ctx, actor, id).await?;
    let kind: QuestionKind = q.kind.parse().map_err(|_| AppError::internal("dạng câu lạ"))?;
    let version_id = q.current_version_id.ok_or_else(|| AppError::internal("câu hỏi chưa có phiên bản"))?;
    exams::regrade_question(ctx, id, kind, &q.body.0, version_id).await
}

pub async fn bulk_create(ctx: &AppContext, actor: &Actor, req: BulkCreateRequest) -> AppResult<BulkCreateResponse> {
    actor.require(Permission::TeachingManage)?;
    req.validate()?;
    let mut bodies = Vec::with_capacity(req.items.len());
    for (i, item) in req.items.iter().enumerate() {
        let body = build_body(item, None).map_err(|e| {
            let detail = e.fields.as_ref().and_then(|f| f.values().next()).and_then(|v| v.first()).cloned();
            AppError::field("items", format!("Câu thứ {}: {}", i + 1, detail.unwrap_or(e.message)))
        })?;
        bodies.push(body);
    }
    // Kiểm tra môn / lớp / thư mục / danh mục một lần cho mỗi bộ giá trị khác nhau.
    type MetaKey = (Uuid, Option<Uuid>, Option<Uuid>, Vec<Uuid>);
    let mut checked: Vec<MetaKey> = Vec::new();
    for item in &req.items {
        let key = (item.subject_id, item.grade_id, item.folder_id, item.tag_ids.clone());
        if !checked.contains(&key) {
            validate_meta(ctx, actor, item).await?;
            checked.push(key);
        }
    }
    let mut tx = ctx.db.begin().await?;
    for (item, body) in req.items.iter().zip(&bodies) {
        insert_question(&mut tx, actor, item, body).await?;
    }
    tx.commit().await?;
    Ok(BulkCreateResponse { created: req.items.len() })
}

// ---------- Import Excel ----------

pub async fn import_preview(
    ctx: &AppContext,
    actor: &Actor,
    subject_id: Uuid,
    bytes: &[u8],
) -> AppResult<ImportPreview> {
    teachers::ensure_can_teach(ctx, actor, subject_id).await?;
    let tags = catalog::list_tags(ctx, Some(actor), subject_id).await?;
    let tag_lookup: Vec<(String, Uuid)> = tags.into_iter().map(|t| (fold(&t.name), t.id)).collect();
    let mut items = import::parse_xlsx(bytes, &tag_lookup)?;
    // Kiểm tra lại từng dòng bằng đúng luật lưu câu hỏi.
    for item in &mut items {
        if !item.errors.is_empty() {
            continue;
        }
        let (Some(kind), Some(level)) = (item.kind, item.level) else { continue };
        let input = QuestionInput {
            subject_id,
            grade_id: None,
            folder_id: None,
            kind,
            level,
            tag_ids: item.tag_ids.clone(),
            content: item.content.clone(),
            options: item.options.clone(),
            accepted: item.accepted.clone(),
            explanation: item.explanation.clone(),
            shuffle_options: true,
        };
        if let Err(e) = build_body(&input, None) {
            let detail = e.fields.as_ref().and_then(|f| f.values().next()).and_then(|v| v.first()).cloned();
            item.errors.push(detail.unwrap_or(e.message));
        }
    }
    let valid_count = items.iter().filter(|i| i.errors.is_empty()).count();
    Ok(ImportPreview { items, valid_count })
}

pub fn import_template() -> AppResult<Vec<u8>> {
    import::template_xlsx()
}

// ---------- Thư mục ----------

pub async fn list_folders(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<FolderResponse>> {
    actor.require(Permission::TeachingManage)?;
    Ok(repo::list_folders(&ctx.db, actor.id).await?.into_iter().map(Into::into).collect())
}

pub async fn ensure_own_folder(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    if repo::folder_owner(&ctx.db, id).await? != Some(actor.id) {
        return Err(AppError::not_found("Không tìm thấy thư mục."));
    }
    Ok(())
}

pub async fn create_folder(ctx: &AppContext, actor: &Actor, input: FolderInput) -> AppResult<FolderResponse> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    if let Some(parent) = input.parent_id {
        ensure_own_folder(ctx, actor, parent).await?;
    }
    let id = repo::insert_folder(&ctx.db, actor.id, input.parent_id, input.name.trim()).await?;
    Ok(FolderResponse { id, parent_id: input.parent_id, name: input.name.trim().to_string(), question_count: 0 })
}

pub async fn update_folder(ctx: &AppContext, actor: &Actor, id: Uuid, input: FolderInput) -> AppResult<()> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    ensure_own_folder(ctx, actor, id).await?;
    if let Some(parent) = input.parent_id {
        ensure_own_folder(ctx, actor, parent).await?;
        if repo::descendant_ids(&ctx.db, id).await?.contains(&parent) {
            return Err(AppError::field("parentId", "Không chuyển thư mục vào bên trong chính nó."));
        }
    }
    repo::update_folder(&ctx.db, id, input.parent_id, input.name.trim()).await?;
    Ok(())
}

pub async fn delete_folder(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    actor.require(Permission::TeachingManage)?;
    ensure_own_folder(ctx, actor, id).await?;
    repo::delete_folder(&ctx.db, id).await?;
    Ok(())
}

// ---------- Dùng cho module đề ----------

pub async fn find_many(ctx: &AppContext, ids: &[Uuid]) -> AppResult<Vec<QuestionRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(repo::find_many(&ctx.db, ids).await?)
}

pub async fn find_versions(ctx: &AppContext, ids: &[Uuid]) -> AppResult<Vec<QuestionVersion>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(repo::find_versions(&ctx.db, ids).await?)
}

/// Bộ lọc của một ô ma trận đề.
pub struct PoolFilter {
    pub owner_id: Uuid,
    pub subject_id: Uuid,
    pub tag_id: Option<Uuid>,
    pub level: Option<String>,
    pub kind: Option<String>,
    pub folder_id: Option<Uuid>,
}

fn pool_filter<'a>(p: &'a PoolFilter, exclude: &'a [Uuid]) -> QuestionFilter<'a> {
    QuestionFilter {
        owner_id: p.owner_id,
        q: None,
        subject_id: Some(p.subject_id),
        folder_id: p.folder_id.map(Some),
        tag_id: p.tag_id,
        level: p.level.as_deref(),
        kind: p.kind.as_deref(),
        exclude,
    }
}

/// Số câu có thể rút cho một ô ma trận.
pub async fn count_pool(ctx: &AppContext, p: &PoolFilter, exclude: &[Uuid]) -> AppResult<i64> {
    Ok(repo::count(&ctx.db, &pool_filter(p, exclude)).await?)
}

/// Rút ngẫu nhiên `count` câu cho một ô ma trận.
pub async fn draw(ctx: &AppContext, p: &PoolFilter, exclude: &[Uuid], count: i64) -> AppResult<Vec<Uuid>> {
    Ok(repo::random_ids(&ctx.db, &pool_filter(p, exclude), count).await?)
}
