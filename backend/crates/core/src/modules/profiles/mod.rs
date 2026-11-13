//! Hồ sơ giáo viên công khai + trang công khai cho máy tìm kiếm (M25, D1 + D14).
//!
//! - Giáo viên sửa hồ sơ (một dòng giới thiệu, giới thiệu chi tiết Markdown, ảnh đại diện, công khai hay không).
//!   Chưa sửa lần nào thì dùng phần giới thiệu trong hồ sơ đăng ký làm giáo viên đã được duyệt.
//! - Ứng dụng (SPA) đọc `GET /api/public/teachers/{username|id}`.
//! - Máy tìm kiếm / mạng xã hội đọc trang HTML do api dựng sẵn: `/p/giao-vien/{username}`, `/p/lop-hoc/{id}`
//!   (tiêu đề, mô tả, Open Graph, dữ liệu có cấu trúc schema.org) + `/sitemap.xml` + `/robots.txt`.
//!   SPA không có nội dung khi chưa chạy JavaScript nên cần các trang này.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        classes::{dto::ClassDetail, service as classes},
        files::{self, Purpose},
        tutoring::{self, OfferView},
    },
    security::{Actor, Permission},
};

// ---------- DTO ----------

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    #[serde(default)]
    #[validate(length(max = 150, message = "Dòng giới thiệu tối đa 150 ký tự."))]
    pub headline: String,
    #[serde(default)]
    #[validate(length(max = 10000, message = "Giới thiệu tối đa 10.000 ký tự."))]
    pub bio: String,
    pub avatar_file_id: Option<Uuid>,
    #[serde(default = "yes")]
    pub is_public: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyProfile {
    pub username: String,
    pub headline: String,
    pub bio: String,
    pub avatar_file_id: Option<Uuid>,
    pub avatar_url: Option<String>,
    pub is_public: bool,
    /// Đường dẫn trang công khai (cho máy tìm kiếm / chia sẻ).
    pub public_path: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PublicClass {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub subject_name: String,
    pub mode: String,
    pub enrollment: String,
    pub student_count: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeacherPublic {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub headline: String,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub subjects: Vec<String>,
    pub classes: Vec<PublicClass>,
    /// Khung giờ học kèm người xem đặt được (chưa đăng nhập: khung mở cho mọi tài khoản).
    pub offers: Vec<OfferView>,
    pub is_me: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct TeacherRow {
    id: Uuid,
    username: String,
    full_name: String,
    headline: Option<String>,
    bio: Option<String>,
    avatar_file_id: Option<Uuid>,
    is_public: Option<bool>,
    application_bio: Option<String>,
    updated_at: DateTime<Utc>,
}

impl TeacherRow {
    fn bio(&self) -> String {
        self.bio.clone().filter(|b| !b.trim().is_empty()).or_else(|| self.application_bio.clone()).unwrap_or_default()
    }
}

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy giáo viên.")
}

const TEACHER_SELECT: &str = "SELECT u.id, u.username, u.full_name, p.headline, p.bio, p.avatar_file_id, p.is_public, \
     (SELECT a.bio FROM teacher_applications a WHERE a.user_id = u.id AND a.status = 'approved' \
       ORDER BY a.reviewed_at DESC NULLS LAST LIMIT 1) AS application_bio, \
     greatest(u.updated_at, coalesce(p.updated_at, u.updated_at)) AS updated_at \
     FROM users u LEFT JOIN teacher_profiles p ON p.user_id = u.id \
     WHERE u.role = 'teacher' AND u.status = 'active'";

/// Tìm giáo viên theo username hoặc ID.
async fn find_teacher(ctx: &AppContext, key: &str) -> AppResult<Option<TeacherRow>> {
    let by_id = Uuid::parse_str(key).ok();
    Ok(sqlx::query_as::<_, TeacherRow>(crate::sql!(
        "{TEACHER_SELECT} AND (u.id = $1 OR ($1 IS NULL AND lower(u.username) = lower($2)))"
    ))
    .bind(by_id)
    .bind(key)
    .fetch_optional(&ctx.db)
    .await?)
}

fn avatar_url(id: Option<Uuid>) -> Option<String> {
    id.map(|id| format!("/api/files/{id}"))
}

// ---------- Giáo viên sửa hồ sơ ----------

pub async fn my_profile(ctx: &AppContext, actor: &Actor) -> AppResult<MyProfile> {
    actor.require(Permission::TeachingManage)?;
    let username: String =
        sqlx::query_scalar("SELECT username FROM users WHERE id = $1").bind(actor.id).fetch_one(&ctx.db).await?;
    let row = find_teacher(ctx, &actor.id.to_string()).await?;
    let bio = row.as_ref().map(TeacherRow::bio).unwrap_or_default();
    let avatar = row.as_ref().and_then(|r| r.avatar_file_id);
    Ok(MyProfile {
        public_path: format!("/p/giao-vien/{username}"),
        username,
        headline: row.as_ref().and_then(|r| r.headline.clone()).unwrap_or_default(),
        bio,
        avatar_file_id: avatar,
        avatar_url: avatar_url(avatar),
        is_public: row.as_ref().and_then(|r| r.is_public).unwrap_or(true),
    })
}

pub async fn update_profile(ctx: &AppContext, actor: &Actor, input: ProfileInput) -> AppResult<MyProfile> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    if let Some(id) = input.avatar_file_id {
        let ok = files::owned_ready(ctx, actor.id, &[id], Purpose::Content).await.is_ok();
        if !ok {
            return Err(AppError::field("avatarFileId", "Ảnh đại diện không hợp lệ."));
        }
    }
    sqlx::query(
        "INSERT INTO teacher_profiles (user_id, headline, bio, avatar_file_id, is_public) VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (user_id) DO UPDATE SET headline = EXCLUDED.headline, bio = EXCLUDED.bio, \
           avatar_file_id = EXCLUDED.avatar_file_id, is_public = EXCLUDED.is_public, updated_at = now()",
    )
    .bind(actor.id)
    .bind(input.headline.trim())
    .bind(input.bio.trim())
    .bind(input.avatar_file_id)
    .bind(input.is_public)
    .execute(&ctx.db)
    .await?;
    my_profile(ctx, actor).await
}

// ---------- Trang công khai (dữ liệu) ----------

async fn public_classes(ctx: &AppContext, teacher: Uuid) -> AppResult<Vec<PublicClass>> {
    Ok(sqlx::query_as::<_, PublicClass>(
        "SELECT c.id, c.title, c.description, s.name AS subject_name, c.mode, c.enrollment, \
         (SELECT count(*) FROM class_members m WHERE m.class_id = c.id AND m.role = 'student' AND m.status = 'active') \
           AS student_count, c.updated_at \
         FROM classes c JOIN subjects s ON s.id = c.subject_id \
         WHERE c.owner_id = $1 AND c.visibility = 'public' AND c.archived_at IS NULL ORDER BY c.updated_at DESC LIMIT 100",
    )
    .bind(teacher)
    .fetch_all(&ctx.db)
    .await?)
}

async fn subjects(ctx: &AppContext, teacher: Uuid) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT s.name FROM teacher_subjects t JOIN subjects s ON s.id = t.subject_id WHERE t.teacher_id = $1 ORDER BY s.name",
    )
    .bind(teacher)
    .fetch_all(&ctx.db)
    .await?)
}

/// Hồ sơ công khai; giáo viên tắt công khai thì chỉ chính họ xem được.
async fn visible_teacher(ctx: &AppContext, viewer: Option<&Actor>, key: &str) -> AppResult<TeacherRow> {
    let row = find_teacher(ctx, key).await?.ok_or_else(not_found)?;
    let is_me = viewer.is_some_and(|a| a.id == row.id);
    if row.is_public == Some(false) && !is_me {
        return Err(not_found());
    }
    Ok(row)
}

pub async fn teacher_public(ctx: &AppContext, viewer: Option<&Actor>, key: &str) -> AppResult<TeacherPublic> {
    let row = visible_teacher(ctx, viewer, key).await?;
    let offers = match viewer {
        Some(a) if a.id != row.id => tutoring::visible_offers(ctx, a, Some(row.id)).await?,
        _ => tutoring::public_offers(ctx, row.id).await?,
    };
    Ok(TeacherPublic {
        bio: row.bio(),
        is_me: viewer.is_some_and(|a| a.id == row.id),
        id: row.id,
        username: row.username,
        full_name: row.full_name,
        headline: row.headline.unwrap_or_default(),
        avatar_url: avatar_url(row.avatar_file_id),
        subjects: subjects(ctx, row.id).await?,
        classes: public_classes(ctx, row.id).await?,
        offers,
    })
}

// ---------- Trang HTML cho máy tìm kiếm ----------

/// Thoát ký tự HTML.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

/// Bỏ ký hiệu Markdown / công thức để làm mô tả ngắn (≤ `max` ký tự, cắt ở ranh giới từ).
pub fn plain_summary(md: &str, max: usize) -> String {
    let text: String = md
        .lines()
        .map(|l| l.trim_start_matches(['#', '>', '-', '*', ' ']).trim())
        .filter(|l| !l.is_empty() && !l.starts_with("![") && !l.starts_with('|'))
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['*', '_', '`', '$'], "");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        return text;
    }
    let cut: String = text.chars().take(max).collect();
    // Cắt đúng ở khoảng trắng thì giữ nguyên; cắt giữa từ thì bỏ phần từ dở.
    let at_boundary = text.chars().nth(max) == Some(' ');
    let cut = if at_boundary { cut } else { cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head).to_string() };
    format!("{cut}…")
}

/// Đoạn văn từ Markdown (giữ xuống dòng, bỏ ký hiệu `**` / `$`, không dựng Markdown — đủ cho máy tìm kiếm
/// và người mở link đọc).
fn paragraphs(md: &str) -> String {
    md.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty() && !p.starts_with("!["))
        .map(|p| format!("<p>{}</p>", esc(&p.replace("**", "").replace('$', "")).replace('\n', "<br>")))
        .collect()
}

/// JSON trong `<script>`: không để lọt `</script>`.
fn json_ld(v: &serde_json::Value) -> String {
    v.to_string().replace('<', "\\u003c")
}

struct Page<'a> {
    title: &'a str,
    description: &'a str,
    path: &'a str,
    og_type: &'a str,
    image: Option<String>,
    json_ld: serde_json::Value,
    body: String,
}

fn render(ctx: &AppContext, p: Page) -> String {
    let base = &ctx.config.app_base_url;
    let image = p
        .image
        .map(|i| format!("<meta property=\"og:image\" content=\"{}\">", esc(&format!("{base}{i}"))))
        .unwrap_or_default();
    format!(
        "<!doctype html>\n<html lang=\"vi\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{title} — Smart Learning Space</title>\
<meta name=\"description\" content=\"{desc}\">\
<link rel=\"canonical\" href=\"{base}{path}\">\
<meta property=\"og:site_name\" content=\"Smart Learning Space\">\
<meta property=\"og:type\" content=\"{og_type}\">\
<meta property=\"og:title\" content=\"{title}\">\
<meta property=\"og:description\" content=\"{desc}\">\
<meta property=\"og:url\" content=\"{base}{path}\">{image}\
<meta name=\"twitter:card\" content=\"summary\">\
<script type=\"application/ld+json\">{ld}</script>\
<style>{CSS}</style></head><body>\
<header><a href=\"/\">Smart Learning Space</a></header><main>{body}</main>\
<footer>Smart Learning Space — học và dạy trực tuyến</footer></body></html>",
        title = esc(p.title),
        desc = esc(p.description),
        path = p.path,
        og_type = p.og_type,
        ld = json_ld(&p.json_ld),
        body = p.body,
    )
}

const CSS: &str = "body{margin:0;font-family:system-ui,-apple-system,'Segoe UI',Roboto,sans-serif;color:#1f2937;\
background:#f6f8fb;line-height:1.6}header,footer{padding:14px 20px;background:#fff;border-bottom:1px solid #e5e7eb}\
footer{border-top:1px solid #e5e7eb;border-bottom:0;color:#6b7280;font-size:14px;margin-top:32px}\
header a{font-weight:700;color:#0864ad;text-decoration:none}main{max-width:860px;margin:0 auto;padding:24px 16px}\
.card{background:#fff;border:1px solid #e5e7eb;border-radius:12px;padding:20px;margin-bottom:16px}\
.muted{color:#6b7280}.btn{display:inline-block;background:#0864ad;color:#fff;padding:10px 18px;border-radius:8px;\
text-decoration:none;font-weight:600}.row{display:flex;gap:16px;align-items:center;flex-wrap:wrap}\
.avatar{width:96px;height:96px;border-radius:50%;object-fit:cover;background:#e5e7eb}h1{margin:0 0 4px}\
ul.outline{padding-left:20px}ul.outline li.m{list-style:none;margin:12px 0 4px -20px;font-weight:600}\
.tag{display:inline-block;background:#eef5fb;color:#0864ad;border-radius:999px;padding:2px 10px;font-size:13px;margin:2px}";

/// Trang công khai của giáo viên (HTML).
pub async fn teacher_page(ctx: &AppContext, username: &str) -> AppResult<String> {
    let row = visible_teacher(ctx, None, username).await?;
    let subjects = subjects(ctx, row.id).await?;
    let classes = public_classes(ctx, row.id).await?;
    let offers = tutoring::public_offers(ctx, row.id).await?;
    let bio = row.bio();
    let headline = row.headline.clone().unwrap_or_default();
    let description = if headline.trim().is_empty() {
        let s = plain_summary(&bio, 160);
        if s.is_empty() { format!("Giáo viên {} trên Smart Learning Space.", row.full_name) } else { s }
    } else {
        headline.clone()
    };
    let path = format!("/p/giao-vien/{}", row.username);
    let mut body = format!(
        "<section class=\"card\"><div class=\"row\">{avatar}<div><h1>{name}</h1><div class=\"muted\">{headline}</div>\
         <div>{tags}</div></div></div>{bio}\
         <p><a class=\"btn\" href=\"/giao-vien/{username}\">Xem trong ứng dụng</a></p></section>",
        avatar = avatar_url(row.avatar_file_id)
            .map(|u| format!("<img class=\"avatar\" src=\"{u}\" alt=\"{}\">", esc(&row.full_name)))
            .unwrap_or_default(),
        name = esc(&row.full_name),
        headline = esc(&headline),
        tags = subjects.iter().map(|s| format!("<span class=\"tag\">{}</span>", esc(s))).collect::<String>(),
        bio = paragraphs(&bio),
        username = esc(&row.username),
    );
    if !classes.is_empty() {
        body.push_str("<section class=\"card\"><h2>Lớp học</h2>");
        for c in &classes {
            body.push_str(&format!(
                "<article><h3><a href=\"/p/lop-hoc/{}\">{}</a></h3><div class=\"muted\">{} · {} học sinh</div><p>{}</p></article>",
                c.id,
                esc(&c.title),
                esc(&c.subject_name),
                c.student_count,
                esc(&plain_summary(&c.description, 220)),
            ));
        }
        body.push_str("</section>");
    }
    if !offers.is_empty() {
        body.push_str("<section class=\"card\"><h2>Học kèm 1-1</h2><ul>");
        for o in &offers {
            body.push_str(&format!("<li>{} — {} phút / buổi</li>", esc(&o.title), o.duration_minutes));
        }
        body.push_str(&format!(
            "</ul><p><a class=\"btn\" href=\"/giao-vien/{}\">Đặt lịch học kèm</a></p></section>",
            esc(&row.username)
        ));
    }
    let ld = json!({
        "@context": "https://schema.org",
        "@type": "Person",
        "name": row.full_name,
        "description": description,
        "jobTitle": "Giáo viên",
        "knowsAbout": subjects,
        "url": format!("{}{path}", ctx.config.app_base_url),
    });
    Ok(render(
        ctx,
        Page {
            title: &format!("{} — giáo viên", row.full_name),
            description: &description,
            path: &path,
            og_type: "profile",
            image: avatar_url(row.avatar_file_id),
            json_ld: ld,
            body,
        },
    ))
}

/// Trang công khai của lớp học (HTML): chỉ lớp học công khai, chưa lưu trữ.
pub async fn class_page(ctx: &AppContext, id: Uuid) -> AppResult<String> {
    let detail: ClassDetail = classes::detail(ctx, None, id).await?;
    let c = &detail.card;
    if c.visibility != "public" {
        return Err(AppError::not_found("Không tìm thấy lớp học."));
    }
    let teacher = find_teacher(ctx, &c.teacher_id.to_string()).await?;
    let subject: String =
        sqlx::query_scalar("SELECT name FROM subjects WHERE id = $1").bind(c.subject_id).fetch_one(&ctx.db).await?;
    let summary = plain_summary(&c.description, 160);
    let description = if summary.is_empty() {
        format!("Lớp học {} — {} — giáo viên {}.", c.title, subject, c.teacher_name)
    } else {
        summary
    };
    let path = format!("/p/lop-hoc/{}", c.id);
    let mut outline = String::new();
    for s in &detail.outline {
        let milestone = matches!(s.kind, crate::modules::classes::model::StepKind::Milestone);
        outline.push_str(&format!("<li{}>{}</li>", if milestone { " class=\"m\"" } else { "" }, esc(&s.title)));
    }
    let teacher_link = match &teacher {
        Some(t) if t.is_public != Some(false) => {
            format!("<a href=\"/p/giao-vien/{}\">{}</a>", esc(&t.username), esc(&c.teacher_name))
        }
        _ => esc(&c.teacher_name),
    };
    let body = format!(
        "<section class=\"card\"><h1>{title}</h1><div class=\"muted\">{subject} · {mode} · Giáo viên {teacher} · {count} học sinh</div>\
         {desc}<p><a class=\"btn\" href=\"/lop-hoc/{id}\">Vào học</a></p></section>\
         {outline}",
        title = esc(&c.title),
        subject = esc(&subject),
        mode = if c.mode == "live" { "Có buổi học trực tuyến" } else { "Tự học" },
        teacher = teacher_link,
        count = c.student_count,
        desc = paragraphs(&c.description),
        id = c.id,
        outline = if outline.is_empty() {
            String::new()
        } else {
            format!("<section class=\"card\"><h2>Lộ trình học</h2><ul class=\"outline\">{outline}</ul></section>")
        },
    );
    let ld = json!({
        "@context": "https://schema.org",
        "@type": "Course",
        "name": c.title,
        "description": description,
        "inLanguage": "vi",
        "about": subject,
        "provider": { "@type": "Person", "name": c.teacher_name },
        "url": format!("{}{path}", ctx.config.app_base_url),
    });
    Ok(render(
        ctx,
        Page {
            title: &c.title,
            description: &description,
            path: &path,
            og_type: "website",
            image: None,
            json_ld: ld,
            body,
        },
    ))
}

/// Sitemap: trang công khai của giáo viên + lớp học công khai.
pub async fn sitemap(ctx: &AppContext) -> AppResult<String> {
    let base = &ctx.config.app_base_url;
    let teachers = sqlx::query_as::<_, TeacherRow>(crate::sql!(
        "{TEACHER_SELECT} AND coalesce(p.is_public, true) ORDER BY u.username LIMIT 10000"
    ))
    .fetch_all(&ctx.db)
    .await?;
    let classes: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, updated_at FROM classes WHERE visibility = 'public' AND archived_at IS NULL ORDER BY id LIMIT 40000",
    )
    .fetch_all(&ctx.db)
    .await?;
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for t in &teachers {
        out.push_str(&format!(
            "<url><loc>{base}/p/giao-vien/{}</loc><lastmod>{}</lastmod></url>\n",
            esc(&t.username),
            t.updated_at.format("%Y-%m-%d")
        ));
    }
    for (id, updated) in &classes {
        out.push_str(&format!(
            "<url><loc>{base}/p/lop-hoc/{id}</loc><lastmod>{}</lastmod></url>\n",
            updated.format("%Y-%m-%d")
        ));
    }
    out.push_str("</urlset>\n");
    Ok(out)
}

pub fn robots(ctx: &AppContext) -> String {
    format!(
        "User-agent: *\nAllow: /p/\nDisallow: /api/\nDisallow: /files/\n\nSitemap: {}/sitemap.xml\n",
        ctx.config.app_base_url
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mo_ta_ngan_bo_ky_hieu_markdown() {
        let md = "# Giới thiệu\n\nTôi dạy **Toán** 9 và ôn thi vào 10, công thức $x^2$.\n\n![ảnh](/api/files/x)\n- Kinh nghiệm 10 năm";
        assert_eq!(
            plain_summary(md, 200),
            "Giới thiệu Tôi dạy Toán 9 và ôn thi vào 10, công thức x^2. Kinh nghiệm 10 năm"
        );
        assert_eq!(plain_summary("một hai ba bốn năm", 10), "một hai ba…");
    }

    #[test]
    fn thoat_html_va_json_trong_script() {
        assert_eq!(esc("<b>\"a\" & 'b'</b>"), "&lt;b&gt;&quot;a&quot; &amp; &#39;b&#39;&lt;/b&gt;");
        let ld = json_ld(&json!({ "name": "</script><script>alert(1)" }));
        assert!(!ld.contains('<'), "{ld}");
        assert!(ld.contains("\\u003c/script>"), "{ld}");
        assert_eq!(paragraphs("a <x>\nb\n\nc"), "<p>a &lt;x&gt;<br>b</p><p>c</p>");
    }
}
