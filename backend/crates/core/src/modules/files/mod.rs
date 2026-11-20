//! File người dùng upload (M21): ảnh trong bài học / câu hỏi / hỏi đáp / thẻ ghi nhớ và bài làm của học sinh.
//!
//! Luồng upload: `POST /api/files` (API ghi bản ghi `pending`, trả presigned URL) → trình duyệt `PUT` thẳng
//! lên Garage → `POST /api/files/{id}/complete` (API kiểm tra file có thật, đúng kích thước, đúng định dạng
//! theo vài byte đầu) → `ready`.
//!
//! Link xem file `GET /api/files/{id}` chuyển hướng sang presigned URL (1 giờ):
//! - `content`: ai có link cũng xem được (ID ngẫu nhiên 122 bit, không đoán được) — để `<img>` trong
//!   Markdown hiện được mà không cần đăng nhập.
//! - `homework`: bài làm của học sinh, link phải có chữ ký `?exp=&sig=` do API cấp cho người có quyền xem.
//! - `import`: file Word (.docx) đề câu hỏi hoặc PDF đề đưa lên bảng — chỉ để API đọc một lần rồi xóa (`take_import`).
//!
//! Ảnh API tự lưu (lấy ra từ file Word) đi qua `save_bytes`: trùng nội dung (SHA-256) thì dùng lại file cũ.

use std::{collections::HashMap, time::Duration};

use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use validator::Validate;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    security::Actor,
    slug::slugify,
    storage::hmac_sha256,
};

/// Ảnh trong nội dung: tối đa 5 MB.
pub const CONTENT_MAX_BYTES: i64 = 5 * 1024 * 1024;
/// Ảnh / PDF bài làm: tối đa 15 MB mỗi file.
pub const HOMEWORK_MAX_BYTES: i64 = 15 * 1024 * 1024;
const UPLOAD_TTL: Duration = Duration::from_secs(15 * 60);
const DOWNLOAD_TTL: Duration = Duration::from_secs(3600);
/// Link có chữ ký của file bài làm: hết hạn sau 6–7 giờ (làm tròn lên đầu giờ để trình duyệt cache được).
const SIGNED_LINK_SECS: i64 = 6 * 3600;
const UPLOADS_PER_HOUR: u32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Content,
    Homework,
    /// File tạm để API đọc một lần rồi xóa: Word đề câu hỏi (.docx) hoặc PDF đề đưa lên bảng.
    Import,
    /// Tài liệu đính kèm bài học / giáo án (PDF, Word, Excel, PowerPoint): link có chữ ký như bài làm.
    Attachment,
}

impl Purpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Homework => "homework",
            Self::Import => "import",
            Self::Attachment => "attachment",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "homework" => Self::Homework,
            "import" => Self::Import,
            "attachment" => Self::Attachment,
            _ => Self::Content,
        }
    }

    pub fn max_bytes(self) -> i64 {
        match self {
            Self::Content => CONTENT_MAX_BYTES,
            Self::Homework => HOMEWORK_MAX_BYTES,
            Self::Import => IMPORT_MAX_BYTES,
            Self::Attachment => ATTACHMENT_MAX_BYTES,
        }
    }

    fn allows(self, content_type: &str) -> bool {
        match self {
            Self::Content => IMAGE_TYPES.contains(&content_type),
            Self::Homework => IMAGE_TYPES.contains(&content_type) || content_type == "application/pdf",
            Self::Import => content_type == DOCX || content_type == PDF,
            Self::Attachment => ATTACHMENT_TYPES.contains(&content_type),
        }
    }

    fn type_hint(self) -> &'static str {
        match self {
            Self::Content => "Chỉ nhận ảnh PNG, JPG, GIF hoặc WebP.",
            Self::Homework => "Chỉ nhận ảnh (PNG, JPG, GIF, WebP) hoặc PDF.",
            Self::Import => {
                "Chỉ nhận file Word .docx (Word 2007 trở lên; file .doc cũ hãy mở bằng Word rồi lưu lại dạng .docx) hoặc PDF."
            }
            Self::Attachment => "Chỉ nhận PDF, Word (.docx), Excel (.xlsx) hoặc PowerPoint (.pptx).",
        }
    }
}

const IMAGE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];
pub const PDF: &str = "application/pdf";
pub const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
pub const IMPORT_MAX_BYTES: i64 = 20 * 1024 * 1024;
pub const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub const PPTX: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
const ATTACHMENT_TYPES: [&str; 4] = ["application/pdf", DOCX, XLSX, PPTX];
pub const ATTACHMENT_MAX_BYTES: i64 = 20 * 1024 * 1024;

/// Kiểm tra vài byte đầu khớp định dạng khai báo (chống đổi đuôi file).
pub fn magic_matches(content_type: &str, head: &[u8]) -> bool {
    match content_type {
        "image/png" => head.starts_with(&[0x89, b'P', b'N', b'G']),
        "image/jpeg" => head.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/gif" => head.starts_with(b"GIF8"),
        "image/webp" => head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP",
        "application/pdf" => head.starts_with(b"%PDF"),
        // .docx / .xlsx / .pptx là file zip
        DOCX | XLSX | PPTX => head.starts_with(b"PK\x03\x04"),
        _ => false,
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UploadInput {
    pub purpose: Purpose,
    #[validate(length(min = 1, max = 200, message = "Tên file 1–200 ký tự."))]
    pub name: String,
    pub content_type: String,
    #[validate(range(min = 1, message = "File rỗng."))]
    pub size: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadTicket {
    pub id: Uuid,
    /// Trình duyệt `PUT` file vào đây, kèm đúng các header trong `headers`.
    pub upload_url: String,
    pub headers: HashMap<String, String>,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub id: Uuid,
    pub name: String,
    pub content_type: String,
    pub size: i64,
    pub is_image: bool,
    /// Link xem file (`/api/files/{id}`, file bài làm có thêm chữ ký).
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenQuery {
    pub exp: Option<i64>,
    pub sig: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FileRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub purpose: String,
    pub object_key: String,
    pub name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub status: String,
}

const COLUMNS: &str = "id, owner_id, purpose, object_key, name, content_type, size_bytes, status";

/// UUID v4 từ số ngẫu nhiên của hệ điều hành (link ảnh không đoán được).
fn random_id() -> Uuid {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("hệ điều hành cung cấp số ngẫu nhiên");
    uuid::Builder::from_random_bytes(bytes).into_uuid()
}

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy file.")
}

/// Tên lưu trên kho: phần tên đã bỏ dấu + đuôi file theo định dạng thật.
fn object_key(purpose: Purpose, id: Uuid, name: &str, content_type: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    let mut stem = slugify(stem);
    stem.truncate(60);
    if stem.is_empty() {
        stem = "file".into();
    }
    let ext = match content_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        DOCX => "docx",
        XLSX => "xlsx",
        PPTX => "pptx",
        _ => "pdf",
    };
    let now = Utc::now();
    format!("{}/{}/{:02}/{id}/{stem}.{ext}", purpose.as_str(), now.year(), now.month())
}

fn signing_key(ctx: &AppContext) -> [u8; 32] {
    hmac_sha256(ctx.config.jwt_secret.as_bytes(), b"sls-file-links")
}

fn signature(ctx: &AppContext, id: Uuid, exp: i64) -> String {
    hex::encode(&hmac_sha256(&signing_key(ctx), format!("{id}.{exp}").as_bytes())[..16])
}

/// Link xem file. File bài làm: kèm chữ ký hết hạn sau ~6 giờ.
pub fn file_url(ctx: &AppContext, id: Uuid, purpose: &str) -> String {
    if purpose == Purpose::Content.as_str() {
        return format!("/api/files/{id}");
    }
    let exp = (Utc::now().timestamp() + SIGNED_LINK_SECS) / 3600 * 3600 + 3600;
    format!("/api/files/{id}?exp={exp}&sig={}", signature(ctx, id, exp))
}

pub fn view(ctx: &AppContext, row: &FileRow) -> FileView {
    FileView {
        id: row.id,
        name: row.name.clone(),
        content_type: row.content_type.clone(),
        size: row.size_bytes,
        is_image: row.content_type.starts_with("image/"),
        url: file_url(ctx, row.id, &row.purpose),
    }
}

async fn find(ctx: &AppContext, id: Uuid) -> AppResult<Option<FileRow>> {
    Ok(sqlx::query_as::<_, FileRow>(crate::sql!("SELECT {COLUMNS} FROM files WHERE id = $1"))
        .bind(id)
        .fetch_optional(&ctx.db)
        .await?)
}

pub async fn create_upload(ctx: &AppContext, actor: &Actor, input: UploadInput) -> AppResult<UploadTicket> {
    input.validate()?;
    let ct = input.content_type.trim().to_ascii_lowercase();
    if !input.purpose.allows(&ct) {
        return Err(AppError::field("contentType", input.purpose.type_hint()));
    }
    let max = input.purpose.max_bytes();
    if input.size > max {
        return Err(AppError::field("size", format!("File tối đa {} MB.", max / 1024 / 1024)));
    }
    if !ctx.storage.is_enabled() {
        return Err(AppError::new(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "STORAGE_DISABLED",
            "Chưa cấu hình nơi lưu file (Garage). Hãy báo quản trị viên.",
        ));
    }
    ctx.limiter.hit_and_check(&format!("upload:{}", actor.id), UPLOADS_PER_HOUR, Duration::from_secs(3600)).await?;
    let id = random_id();
    let key = object_key(input.purpose, id, &input.name, &ct);
    let upload_url = ctx.storage.presign_put(&key, &ct, UPLOAD_TTL)?;
    sqlx::query(
        "INSERT INTO files (id, owner_id, purpose, object_key, name, content_type, size_bytes)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(actor.id)
    .bind(input.purpose.as_str())
    .bind(&key)
    .bind(input.name.trim())
    .bind(&ct)
    .bind(input.size)
    .execute(&ctx.db)
    .await?;
    Ok(UploadTicket {
        id,
        upload_url,
        headers: HashMap::from([("Content-Type".to_string(), ct)]),
        expires_in: UPLOAD_TTL.as_secs(),
    })
}

/// Trình duyệt báo đã upload xong: kiểm tra file trên kho rồi chuyển sang `ready`.
pub async fn complete(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<FileView> {
    let row = find(ctx, id).await?.filter(|r| r.owner_id == actor.id).ok_or_else(not_found)?;
    if row.status == "ready" {
        return Ok(view(ctx, &row));
    }
    let info = ctx
        .storage
        .head(&row.object_key)
        .await?
        .ok_or_else(|| AppError::conflict("FILE_NOT_UPLOADED", "Chưa nhận được file. Hãy thử upload lại."))?;
    let purpose = Purpose::parse(&row.purpose);
    let head = ctx.storage.read_prefix(&row.object_key, 16).await?;
    let problem = if info.size <= 0 || info.size > purpose.max_bytes() {
        Some(format!("File phải nhỏ hơn {} MB.", purpose.max_bytes() / 1024 / 1024))
    } else if !magic_matches(&row.content_type, &head) {
        Some("Nội dung file không đúng định dạng đã chọn.".to_string())
    } else {
        None
    };
    if let Some(message) = problem {
        ctx.storage.delete(&row.object_key).await?;
        sqlx::query("DELETE FROM files WHERE id = $1").bind(id).execute(&ctx.db).await?;
        return Err(AppError::field("file", message));
    }
    let row = sqlx::query_as::<_, FileRow>(crate::sql!(
        "UPDATE files SET status = 'ready', size_bytes = $2, ready_at = now() WHERE id = $1 RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(info.size)
    .fetch_one(&ctx.db)
    .await?;
    Ok(view(ctx, &row))
}

/// Link presigned để trình duyệt xem file (`GET /api/files/{id}` chuyển hướng tới đây).
pub async fn open(ctx: &AppContext, id: Uuid, q: OpenQuery) -> AppResult<String> {
    let row = find(ctx, id).await?.filter(|r| r.status == "ready").ok_or_else(not_found)?;
    if row.purpose != Purpose::Content.as_str() {
        let (Some(exp), Some(sig)) = (q.exp, q.sig.as_deref()) else { return Err(not_found()) };
        let expected = signature(ctx, id, exp);
        // So sánh không phụ thuộc vị trí sai đầu tiên.
        let same =
            expected.len() == sig.len() && expected.bytes().zip(sig.bytes()).fold(0u8, |a, (x, y)| a | (x ^ y)) == 0;
        if !same {
            return Err(not_found());
        }
        if exp < Utc::now().timestamp() {
            return Err(AppError::new(
                axum::http::StatusCode::GONE,
                "LINK_EXPIRED",
                "Link xem file đã hết hạn. Hãy tải lại trang.",
            ));
        }
    }
    let inline = row.content_type.starts_with("image/") || row.content_type == "application/pdf";
    ctx.storage.presign_get(&row.object_key, &row.content_type, &row.name, inline, DOWNLOAD_TTL)
}

/// File đã upload xong của `owner_id` (dùng khi nộp bài: chỉ được gắn file của chính mình).
pub async fn owned_ready(ctx: &AppContext, owner_id: Uuid, ids: &[Uuid], purpose: Purpose) -> AppResult<Vec<FileRow>> {
    let rows = sqlx::query_as::<_, FileRow>(crate::sql!(
        "SELECT {COLUMNS} FROM files WHERE id = ANY($1) AND owner_id = $2 AND purpose = $3 AND status = 'ready'"
    ))
    .bind(ids)
    .bind(owner_id)
    .bind(purpose.as_str())
    .fetch_all(&ctx.db)
    .await?;
    if rows.len() != ids.len() {
        return Err(AppError::field("fileIds", "File không tồn tại hoặc chưa upload xong."));
    }
    // Giữ thứ tự gửi lên.
    let mut by_id: HashMap<Uuid, FileRow> = rows.into_iter().map(|r| (r.id, r)).collect();
    Ok(ids.iter().filter_map(|id| by_id.remove(id)).collect())
}

/// Thông tin + link của nhiều file (giữ thứ tự `ids`, bỏ file không còn).
pub async fn views(ctx: &AppContext, ids: &[Uuid]) -> AppResult<HashMap<Uuid, FileView>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, FileRow>(crate::sql!("SELECT {COLUMNS} FROM files WHERE id = ANY($1)"))
        .bind(ids)
        .fetch_all(&ctx.db)
        .await?;
    Ok(rows.iter().map(|r| (r.id, view(ctx, r))).collect())
}

/// Tài liệu đính kèm lưu trong jsonb của bài học / giáo án (tên giữ lại để bản xuất bản cũ vẫn hiện đúng).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentRef {
    pub file_id: Uuid,
    pub name: String,
}

pub const MAX_ATTACHMENTS: usize = 10;

/// Kiểm tra tài liệu đính kèm: file của chính người dùng, mục đích `attachment`, đã upload xong. Giữ thứ tự, bỏ trùng.
pub async fn check_attachments(ctx: &AppContext, owner_id: Uuid, ids: &[Uuid]) -> AppResult<Vec<AttachmentRef>> {
    let mut unique: Vec<Uuid> = Vec::new();
    for id in ids {
        if !unique.contains(id) {
            unique.push(*id);
        }
    }
    if unique.len() > MAX_ATTACHMENTS {
        return Err(AppError::field("attachmentIds", format!("Tối đa {MAX_ATTACHMENTS} tài liệu.")));
    }
    let rows = owned_ready(ctx, owner_id, &unique, Purpose::Attachment)
        .await
        .map_err(|_| AppError::field("attachmentIds", "Tài liệu không tồn tại hoặc chưa upload xong."))?;
    Ok(rows.into_iter().map(|r| AttachmentRef { file_id: r.id, name: r.name }).collect())
}

/// Thông tin + link (có chữ ký) của tài liệu đính kèm, giữ thứ tự; file đã bị xóa thì bỏ qua.
pub async fn attachment_views(ctx: &AppContext, refs: &[AttachmentRef]) -> AppResult<Vec<FileView>> {
    let ids: Vec<Uuid> = refs.iter().map(|r| r.file_id).collect();
    let mut found = views(ctx, &ids).await?;
    Ok(refs
        .iter()
        .filter_map(|r| {
            found.remove(&r.file_id).map(|mut v| {
                v.name = r.name.clone();
                v
            })
        })
        .collect())
}

/// API tự lưu một file (ảnh lấy ra từ file Word…) thành file `ready` của `owner_id`.
/// Cùng người, cùng mục đích, cùng nội dung (SHA-256) → dùng lại file đã có, không lưu thêm bản mới.
pub async fn save_bytes(
    ctx: &AppContext,
    owner_id: Uuid,
    purpose: Purpose,
    name: &str,
    content_type: &str,
    bytes: &[u8],
) -> AppResult<FileRow> {
    if !purpose.allows(content_type) {
        return Err(AppError::field("file", purpose.type_hint()));
    }
    if bytes.is_empty() || bytes.len() as i64 > purpose.max_bytes() {
        return Err(AppError::field("file", format!("File tối đa {} MB.", purpose.max_bytes() / 1024 / 1024)));
    }
    if !magic_matches(content_type, &bytes[..bytes.len().min(16)]) {
        return Err(AppError::field("file", "Nội dung file không đúng định dạng."));
    }
    let sha = hex::encode(Sha256::digest(bytes));
    let existing = sqlx::query_as::<_, FileRow>(crate::sql!(
        "SELECT {COLUMNS} FROM files
         WHERE owner_id = $1 AND purpose = $2 AND sha256 = $3 AND status = 'ready' LIMIT 1"
    ))
    .bind(owner_id)
    .bind(purpose.as_str())
    .bind(&sha)
    .fetch_optional(&ctx.db)
    .await?;
    if let Some(row) = existing {
        return Ok(row);
    }
    let id = random_id();
    let key = object_key(purpose, id, name, content_type);
    ctx.storage.put(&key, bytes, content_type).await?;
    Ok(sqlx::query_as::<_, FileRow>(crate::sql!(
        "INSERT INTO files (id, owner_id, purpose, object_key, name, content_type, size_bytes, status, ready_at, sha256)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'ready', now(), $8) RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(owner_id)
    .bind(purpose.as_str())
    .bind(&key)
    .bind(name)
    .bind(content_type)
    .bind(bytes.len() as i64)
    .bind(&sha)
    .fetch_one(&ctx.db)
    .await?)
}

/// Đọc file tạm người dùng vừa upload (Word đề câu hỏi, PDF đề đưa lên bảng), rồi xóa luôn (không giữ lại).
/// `content_type`: loại file nơi gọi cần (`DOCX` / `PDF`) — sai loại thì coi như không có file.
pub async fn take_import(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    content_type: &str,
) -> AppResult<(String, Vec<u8>)> {
    let row = find(ctx, id)
        .await?
        .filter(|r| {
            r.owner_id == actor.id
                && r.purpose == Purpose::Import.as_str()
                && r.status == "ready"
                && r.content_type == content_type
        })
        .ok_or_else(not_found)?;
    let bytes = ctx.storage.get(&row.object_key, IMPORT_MAX_BYTES as usize).await;
    ctx.storage.delete(&row.object_key).await?;
    sqlx::query("DELETE FROM files WHERE id = $1").bind(id).execute(&ctx.db).await?;
    Ok((row.name, bytes?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kiem_tra_chu_ky_dinh_dang() {
        assert!(magic_matches("image/png", &[0x89, b'P', b'N', b'G', 0x0D, 0x0A]));
        assert!(magic_matches("image/jpeg", &[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(magic_matches("image/webp", b"RIFF\x10\x00\x00\x00WEBPVP8 "));
        assert!(magic_matches("application/pdf", b"%PDF-1.7"));
        assert!(!magic_matches("image/png", b"<html><script>"));
        assert!(!magic_matches("text/html", b"<html>"));
    }

    #[test]
    fn loai_file_theo_muc_dich() {
        assert!(Purpose::Content.allows("image/webp"));
        assert!(!Purpose::Content.allows("application/pdf"));
        assert!(Purpose::Homework.allows("application/pdf"));
        assert!(!Purpose::Homework.allows("image/svg+xml"));
        assert!(Purpose::Import.allows(DOCX));
        assert!(!Purpose::Import.allows("image/png"));
        assert!(magic_matches(DOCX, b"PK\x03\x04\x14\x00"));
        assert!(!magic_matches(DOCX, b"\xD0\xCF\x11\xE0")); // .doc cũ (OLE)
        assert!(Purpose::Attachment.allows("application/pdf") && Purpose::Attachment.allows(PPTX));
        assert!(!Purpose::Attachment.allows("image/png"));
        assert!(magic_matches(XLSX, b"PK\x03\x04") && !magic_matches(XLSX, b"%PDF"));
    }

    #[test]
    fn ten_luu_tren_kho_bo_dau_va_dung_duoi_that() {
        let id = Uuid::nil();
        let key = object_key(Purpose::Homework, id, "Bài làm câu 3 (trang 1).JPEG", "image/jpeg");
        assert!(key.starts_with("homework/"));
        assert!(key.ends_with(&format!("/{id}/bai-lam-cau-3-trang-1.jpg")), "{key}");
        assert!(object_key(Purpose::Content, id, "....png", "image/png").ends_with("/file.png"));
    }
}
