//! Giáo án (M34): tài liệu soạn bài của giáo viên, khác "bài học" học sinh xem.
//!
//! Giáo án là danh sách **khối** theo mẫu (khung khối ban đầu do giao diện tạo, xem `frontend/src/utils/lessonPlan.js`):
//! - CV 5512 (THCS / THPT): Mục tiêu → Thiết bị → 4 hoạt động (Mở đầu, Hình thành kiến thức, Luyện tập, Vận dụng),
//!   mỗi hoạt động a) Mục tiêu b) Nội dung c) Sản phẩm d) Tổ chức thực hiện (4 bước).
//! - CV 2345 (tiểu học): bảng hai cột "Hoạt động của GV | Hoạt động của HS".
//! - Dạy thêm: kiến thức cần nhớ → các dạng bài (phương pháp, ví dụ mẫu, bài tập) → bài tập về nhà.
//!
//! Mọi trường chữ là Markdown + công thức `$…$`. Khối câu hỏi chỉ trỏ tới câu trong ngân hàng của chính giáo viên.
//! Xuất Word / trang in làm ở trình duyệt (`frontend/src/utils/docx/`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use validator::Validate;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        catalog::service as catalog,
        files::{self, AttachmentRef, FileView},
        lessons::service as lessons,
        teachers::service as teachers,
    },
    security::{Actor, Permission},
};

pub const MAX_BLOCKS: usize = 80;
const MAX_TEXT: usize = 20_000;
const MAX_TITLE: usize = 300;
const MAX_ROWS: usize = 60;
const MAX_QUESTIONS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Template {
    Cv5512,
    Cv2345,
    DayThem,
    Blank,
}

impl Template {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cv5512 => "cv5512",
            Self::Cv2345 => "cv2345",
            Self::DayThem => "day_them",
            Self::Blank => "blank",
        }
    }
}

/// Thông tin chung ở đầu giáo án (chữ tự do: "Tuần 10 — tiết 19, 20", "12/11/2026"…).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlanInfo {
    pub school: String,
    pub group: String,
    pub teacher_name: String,
    pub periods: String,
    pub prepared_on: String,
    pub taught_on: String,
    pub class_name: String,
}

/// Tổ chức thực hiện của một hoạt động (CV 5512): giao nhiệm vụ → thực hiện → báo cáo, thảo luận → kết luận.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ActivitySteps {
    pub assign: String,
    pub perform: String,
    pub report: String,
    pub conclude: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TwoColumnRow {
    pub teacher: String,
    pub student: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum BlockBody {
    /// Mục chữ: "I. Mục tiêu", "II. Thiết bị dạy học và học liệu"…
    Section {
        #[serde(default)]
        title: String,
        #[serde(default)]
        body: String,
    },
    /// Hoạt động theo CV 5512.
    Activity {
        #[serde(default)]
        title: String,
        #[serde(default)]
        minutes: Option<i32>,
        #[serde(default)]
        goal: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        product: String,
        #[serde(default)]
        steps: ActivitySteps,
    },
    /// Bảng hai cột GV / HS (CV 2345).
    TwoColumns {
        #[serde(default)]
        title: String,
        #[serde(default)]
        rows: Vec<TwoColumnRow>,
    },
    /// Một dạng bài (giáo án dạy thêm).
    ProblemType {
        #[serde(default)]
        title: String,
        #[serde(default)]
        method: String,
        #[serde(default)]
        examples: String,
        #[serde(default)]
        exercises: String,
    },
    /// Phiếu học tập / bài tập lấy từ ngân hàng câu hỏi.
    Questions {
        #[serde(default)]
        title: String,
        #[serde(default)]
        question_ids: Vec<Uuid>,
        #[serde(default)]
        show_answers: bool,
    },
}

/// Khối của giáo án; `id` do giao diện đặt (giữ vị trí khi kéo thả, không bắt buộc duy nhất ở api).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    #[serde(default)]
    pub id: String,
    #[serde(flatten)]
    pub body: BlockBody,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LessonPlanInput {
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    #[validate(length(min = 1, max = 300, message = "Tên bài dạy từ 1 đến 300 ký tự."))]
    pub title: String,
    pub template: Template,
    #[serde(default)]
    pub info: PlanInfo,
    #[serde(default)]
    pub blocks: Vec<Block>,
    pub lesson_id: Option<Uuid>,
    #[serde(default)]
    pub attachment_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LessonPlanQuery {
    pub q: Option<String>,
    pub subject_id: Option<Uuid>,
    pub grade_id: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LessonPlanSummary {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: String,
    pub template: String,
    pub block_count: i32,
    pub lesson_id: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonPlanDetail {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: String,
    pub template: String,
    pub info: Value,
    pub blocks: Value,
    pub lesson_id: Option<Uuid>,
    pub lesson_title: Option<String>,
    pub attachments: Vec<FileView>,
    pub copied_from: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct Row {
    id: Uuid,
    subject_id: Uuid,
    grade_id: Option<Uuid>,
    title: String,
    template: String,
    info: Value,
    blocks: Value,
    lesson_id: Option<Uuid>,
    lesson_title: Option<String>,
    attachments: Value,
    copied_from: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const COLS: &str = "p.id, p.subject_id, p.grade_id, p.title, p.template, p.info, p.blocks, p.lesson_id, \
     (SELECT l.title FROM lessons l WHERE l.id = p.lesson_id AND l.archived_at IS NULL) AS lesson_title, \
     p.attachments, p.copied_from, p.created_at, p.updated_at";

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy giáo án.")
}

fn too_long(what: &str, max: &str) -> AppError {
    AppError::field("blocks", format!("{what} tối đa {max} ký tự."))
}

/// Kiểm tra số khối, độ dài từng trường; trả về các câu hỏi được trỏ tới.
fn check_blocks(blocks: &[Block]) -> AppResult<Vec<Uuid>> {
    if blocks.len() > MAX_BLOCKS {
        return Err(AppError::field("blocks", format!("Giáo án tối đa {MAX_BLOCKS} mục.")));
    }
    let text =
        |s: &str| if s.chars().count() > MAX_TEXT { Err(too_long("Mỗi ô nội dung", "20.000")) } else { Ok(()) };
    let title =
        |s: &str| if s.chars().count() > MAX_TITLE { Err(too_long("Tiêu đề mục", "300")) } else { Ok(()) };
    let mut questions = Vec::new();
    for b in blocks {
        if b.id.len() > 64 {
            return Err(AppError::field("blocks", "Mã khối không hợp lệ."));
        }
        match &b.body {
            BlockBody::Section { title: t, body } => {
                title(t)?;
                text(body)?;
            }
            BlockBody::Activity { title: t, minutes, goal, content, product, steps } => {
                title(t)?;
                if minutes.is_some_and(|m| !(0..=600).contains(&m)) {
                    return Err(AppError::field("blocks", "Thời lượng hoạt động từ 0 đến 600 phút."));
                }
                for s in [goal, content, product, &steps.assign, &steps.perform, &steps.report, &steps.conclude] {
                    text(s)?;
                }
            }
            BlockBody::TwoColumns { title: t, rows } => {
                title(t)?;
                if rows.len() > MAX_ROWS {
                    return Err(AppError::field("blocks", format!("Mỗi bảng tối đa {MAX_ROWS} dòng.")));
                }
                for r in rows {
                    text(&r.teacher)?;
                    text(&r.student)?;
                }
            }
            BlockBody::ProblemType { title: t, method, examples, exercises } => {
                title(t)?;
                for s in [method, examples, exercises] {
                    text(s)?;
                }
            }
            BlockBody::Questions { title: t, question_ids, .. } => {
                title(t)?;
                if question_ids.len() > MAX_QUESTIONS {
                    return Err(AppError::field("blocks", format!("Mỗi phiếu tối đa {MAX_QUESTIONS} câu hỏi.")));
                }
                questions.extend(question_ids.iter().copied());
            }
        }
    }
    questions.sort();
    questions.dedup();
    Ok(questions)
}

fn check_info(info: &PlanInfo) -> AppResult<()> {
    let fields = [
        &info.school,
        &info.group,
        &info.teacher_name,
        &info.periods,
        &info.prepared_on,
        &info.taught_on,
        &info.class_name,
    ];
    if fields.iter().any(|f| f.chars().count() > 200) {
        return Err(AppError::field("info", "Mỗi ô thông tin chung tối đa 200 ký tự."));
    }
    Ok(())
}

/// Kiểm tra toàn bộ đầu vào; trả về danh sách tài liệu đính kèm để lưu.
async fn validate(ctx: &AppContext, actor: &Actor, input: &LessonPlanInput) -> AppResult<Vec<AttachmentRef>> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    if let Some(grade) = input.grade_id {
        catalog::ensure_grades_visible(ctx, &[grade], actor.id)
            .await
            .map_err(|_| AppError::field("gradeId", "Lớp không tồn tại."))?;
    }
    check_info(&input.info)?;
    let questions = check_blocks(&input.blocks)?;
    if !questions.is_empty() {
        let own: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM questions WHERE id = ANY($1) AND owner_id = $2 AND archived_at IS NULL",
        )
        .bind(&questions)
        .bind(actor.id)
        .fetch_one(&ctx.db)
        .await?;
        if own != questions.len() as i64 {
            return Err(AppError::field("blocks", "Có câu hỏi không có trong ngân hàng của bạn."));
        }
    }
    if let Some(lesson) = input.lesson_id {
        let ok = lessons::refs(ctx, &[lesson]).await?.first().is_some_and(|l| l.owner_id == actor.id && !l.archived);
        if !ok {
            return Err(AppError::field("lessonId", "Không tìm thấy bài học."));
        }
    }
    files::check_attachments(ctx, actor.id, &input.attachment_ids).await
}

async fn find_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<Row> {
    actor.require(Permission::TeachingManage)?;
    sqlx::query_as::<_, Row>(crate::sql!(
        "SELECT {COLS} FROM lesson_plans p WHERE p.id = $1 AND p.owner_id = $2 AND p.archived_at IS NULL"
    ))
    .bind(id)
    .bind(actor.id)
    .fetch_optional(&ctx.db)
    .await?
    .ok_or_else(not_found)
}

async fn detail(ctx: &AppContext, row: Row) -> AppResult<LessonPlanDetail> {
    let refs: Vec<AttachmentRef> = serde_json::from_value(row.attachments).unwrap_or_default();
    Ok(LessonPlanDetail {
        id: row.id,
        subject_id: row.subject_id,
        grade_id: row.grade_id,
        title: row.title,
        template: row.template,
        info: row.info,
        blocks: row.blocks,
        lesson_id: row.lesson_id,
        lesson_title: row.lesson_title,
        attachments: files::attachment_views(ctx, &refs).await?,
        copied_from: row.copied_from,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

pub async fn list(ctx: &AppContext, actor: &Actor, q: LessonPlanQuery) -> AppResult<Vec<LessonPlanSummary>> {
    actor.require(Permission::TeachingManage)?;
    let pattern =
        q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| format!("%{}%", crate::db::escape_like(s)));
    Ok(sqlx::query_as::<_, LessonPlanSummary>(
        "SELECT id, subject_id, grade_id, title, template, jsonb_array_length(blocks) AS block_count, lesson_id, updated_at \
         FROM lesson_plans WHERE owner_id = $1 AND archived_at IS NULL \
         AND ($2::text IS NULL OR unaccent(lower(title)) LIKE unaccent(lower($2))) \
         AND ($3::uuid IS NULL OR subject_id = $3) AND ($4::uuid IS NULL OR grade_id = $4) \
         ORDER BY updated_at DESC LIMIT 300",
    )
    .bind(actor.id)
    .bind(pattern)
    .bind(q.subject_id)
    .bind(q.grade_id)
    .fetch_all(&ctx.db)
    .await?)
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<LessonPlanDetail> {
    let row = find_own(ctx, actor, id).await?;
    detail(ctx, row).await
}

fn to_json<T: Serialize>(v: &T) -> AppResult<Value> {
    serde_json::to_value(v).map_err(AppError::internal)
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: LessonPlanInput) -> AppResult<LessonPlanDetail> {
    let attachments = validate(ctx, actor, &input).await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO lesson_plans (owner_id, subject_id, grade_id, title, template, info, blocks, lesson_id, attachments) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(actor.id)
    .bind(input.subject_id)
    .bind(input.grade_id)
    .bind(input.title.trim())
    .bind(input.template.as_str())
    .bind(to_json(&input.info)?)
    .bind(to_json(&input.blocks)?)
    .bind(input.lesson_id)
    .bind(to_json(&attachments)?)
    .fetch_one(&ctx.db)
    .await?;
    get(ctx, actor, id).await
}

/// Lưu cả giáo án (giao diện tự lưu sau mỗi lần ngừng gõ).
pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: LessonPlanInput) -> AppResult<LessonPlanDetail> {
    find_own(ctx, actor, id).await?;
    let attachments = validate(ctx, actor, &input).await?;
    sqlx::query(
        "UPDATE lesson_plans SET subject_id = $2, grade_id = $3, title = $4, template = $5, info = $6, blocks = $7, \
         lesson_id = $8, attachments = $9, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(input.subject_id)
    .bind(input.grade_id)
    .bind(input.title.trim())
    .bind(input.template.as_str())
    .bind(to_json(&input.info)?)
    .bind(to_json(&input.blocks)?)
    .bind(input.lesson_id)
    .bind(to_json(&attachments)?)
    .execute(&ctx.db)
    .await?;
    get(ctx, actor, id).await
}

/// Sao chép (dùng lại giáo án năm trước, soạn cho lớp khác).
pub async fn copy(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<LessonPlanDetail> {
    let src = find_own(ctx, actor, id).await?;
    let mut title = format!("{} (bản sao)", src.title);
    if title.chars().count() > MAX_TITLE {
        title = title.chars().take(MAX_TITLE).collect();
    }
    let new_id: Uuid = sqlx::query_scalar(
        "INSERT INTO lesson_plans (owner_id, subject_id, grade_id, title, template, info, blocks, lesson_id, attachments, copied_from) \
         SELECT owner_id, subject_id, grade_id, $2, template, info, blocks, lesson_id, attachments, id \
         FROM lesson_plans WHERE id = $1 RETURNING id",
    )
    .bind(id)
    .bind(title)
    .fetch_one(&ctx.db)
    .await?;
    get(ctx, actor, new_id).await
}

pub async fn archive(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    find_own(ctx, actor, id).await?;
    sqlx::query("UPDATE lesson_plans SET archived_at = now() WHERE id = $1").bind(id).execute(&ctx.db).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_khoi_tu_json_cua_giao_dien() {
        let blocks: Vec<Block> = serde_json::from_value(serde_json::json!([
            { "id": "a1", "kind": "section", "title": "I. Mục tiêu", "body": "1. Kiến thức" },
            { "kind": "activity", "title": "Hoạt động 1: Mở đầu", "minutes": 5, "goal": "Gợi động cơ",
              "steps": { "assign": "GV chiếu hình", "conclude": "GV chốt" } },
            { "kind": "two_columns", "rows": [{ "teacher": "Hỏi", "student": "Trả lời" }] },
            { "kind": "problem_type", "title": "Dạng 1", "method": "Đặt ẩn phụ $t = x^2$" },
            { "kind": "questions", "questionIds": [], "showAnswers": true }
        ]))
        .unwrap();
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[0].id, "a1");
        match &blocks[1].body {
            BlockBody::Activity { minutes, steps, product, .. } => {
                assert_eq!(*minutes, Some(5));
                assert_eq!(steps.assign, "GV chiếu hình");
                assert_eq!(product, "");
            }
            other => panic!("{other:?}"),
        }
        assert!(check_blocks(&blocks).unwrap().is_empty());
        // Ghi ra JSON giữ đúng tên trường của giao diện
        let json = serde_json::to_value(&blocks[4]).unwrap();
        assert_eq!(json["kind"], "questions");
        assert_eq!(json["showAnswers"], true);
    }

    #[test]
    fn gioi_han_khoi() {
        let many: Vec<Block> = (0..=MAX_BLOCKS)
            .map(|_| Block {
                id: String::new(),
                body: BlockBody::Section { title: String::new(), body: String::new() },
            })
            .collect();
        assert!(check_blocks(&many).is_err());
        let long = vec![Block {
            id: String::new(),
            body: BlockBody::Section { title: "x".into(), body: "a".repeat(MAX_TEXT + 1) },
        }];
        assert!(check_blocks(&long).is_err());
        assert!(serde_json::from_value::<Vec<Block>>(serde_json::json!([{ "kind": "lạ" }])).is_err());
    }
}
