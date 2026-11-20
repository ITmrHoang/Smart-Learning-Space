//! Nghiệp vụ đề và lượt làm bài. Xem mô tả ở `mod.rs`.

use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{
        AnswerFile, AttemptQuery, AttemptSummary, AttemptView, ExamInput, ExamQuery, ExamResponse, ExamSummary,
        GradeItemInput, ItemGrading, ItemResult, ItemView, PaperOption, PaperQuestion, PaperView, PapersInput,
        PapersResponse, QueueItem, RuleResponse, SectionResponse, StudentInfo, ViewOption,
    },
    model::{AttemptRow, BestScoreRow, ExamKind, ExamRef, ExamRow, PaperItem, ShowAnswers},
    repository::{self as repo, ExamFields, ExamFilter, ManualGrade, NewAttempt, NewItem, NewRule, NewSection},
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        catalog::service as catalog,
        classes::service as classes,
        files::{self, Purpose},
        notifications::{self, NotificationKind},
        questions::{
            dto::{QuestionResponse, RegradeResponse},
            grading::{self, Answer, Scoring, round2},
            model::{MatchMode, QuestionBody, QuestionKind},
            service::{self as questions, PoolFilter},
        },
        teachers::service as teachers,
        users::service as users,
    },
    pagination::{Page, PageRequest},
    security::{Actor, Permission, token::shuffle},
};

/// Cho phép gửi đáp án trễ vài giây so với hạn (mạng chậm).
const DEADLINE_GRACE_SECS: i64 = 15;
/// Câu tự luận: bài gõ tối đa bao nhiêu ký tự, nộp tối đa bao nhiêu ảnh / file.
const MAX_ESSAY_TEXT: usize = 10_000;
const MAX_ESSAY_FILES: usize = 10;

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy đề.")
}

fn attempt_not_found() -> AppError {
    AppError::not_found("Không tìm thấy bài làm.")
}

fn exam_kind(e: &ExamRow) -> ExamKind {
    if e.kind == "test" { ExamKind::Test } else { ExamKind::Practice }
}

fn show_answers(e: &ExamRow) -> ShowAnswers {
    match e.show_answers.as_str() {
        "after_close" => ShowAnswers::AfterClose,
        "never" => ShowAnswers::Never,
        _ => ShowAnswers::AfterSubmit,
    }
}

async fn load_own_exam(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<ExamRow> {
    actor.require(Permission::TeachingManage)?;
    let exam = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if exam.owner_id != actor.id || exam.archived_at.is_some() {
        return Err(not_found());
    }
    Ok(exam)
}

// ---------- Soạn đề ----------

async fn validate_input(ctx: &AppContext, actor: &Actor, input: &ExamInput) -> AppResult<()> {
    input.validate()?;
    teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    if let Some(grade) = input.grade_id {
        catalog::ensure_grades_visible(ctx, &[grade], actor.id)
            .await
            .map_err(|_| AppError::field("gradeId", "Lớp không tồn tại."))?;
    }
    if let (Some(open), Some(close)) = (input.opens_at, input.closes_at)
        && close <= open
    {
        return Err(AppError::field("closesAt", "Giờ đóng đề phải sau giờ mở đề."));
    }
    let mut all_ids: Vec<Uuid> = Vec::new();
    for (i, s) in input.sections.iter().enumerate() {
        if s.question_ids.is_empty() && s.rules.is_empty() {
            return Err(AppError::field("sections", format!("Phần {} chưa có câu hỏi nào.", i + 1)));
        }
        for (id, p) in &s.question_points {
            if !s.question_ids.contains(id) || !(0.0..=100.0).contains(p) {
                return Err(AppError::field(
                    "sections",
                    format!("Phần {}: điểm riêng của câu phải từ 0 đến 100 và câu phải có trong phần.", i + 1),
                ));
            }
        }
        for id in &s.question_ids {
            if all_ids.contains(id) {
                return Err(AppError::field("sections", "Một câu hỏi chỉ được xuất hiện một lần trong đề."));
            }
            all_ids.push(*id);
        }
        for r in &s.rules {
            if let Some(tag) = r.tag_id {
                catalog::ensure_tags_visible(ctx, input.subject_id, &[tag], actor.id).await?;
            }
            if let Some(folder) = r.folder_id {
                questions::ensure_own_folder(ctx, actor, folder)
                    .await
                    .map_err(|_| AppError::field("sections", "Thư mục trong ma trận không tồn tại."))?;
            }
        }
    }
    let found = questions::find_many(ctx, &all_ids).await?;
    let ok = found.len() == all_ids.len()
        && found.iter().all(|q| q.owner_id == actor.id && q.subject_id == input.subject_id && q.archived_at.is_none());
    if !ok {
        return Err(AppError::field("sections", "Có câu hỏi không tồn tại, đã bị ẩn hoặc không thuộc môn của đề."));
    }
    Ok(())
}

fn exam_fields(input: &ExamInput) -> ExamFields<'_> {
    ExamFields {
        subject_id: input.subject_id,
        grade_id: input.grade_id,
        title: input.title.trim(),
        description: input.description.trim(),
        kind: input.kind.as_str(),
        time_limit_minutes: input.time_limit_minutes,
        max_attempts: input.max_attempts,
        opens_at: input.opens_at,
        closes_at: input.closes_at,
        shuffle_questions: input.shuffle_questions,
        shuffle_options: input.shuffle_options,
        show_answers: input.show_answers.as_str(),
    }
}

fn new_sections(input: &ExamInput) -> Vec<NewSection<'_>> {
    input
        .sections
        .iter()
        .map(|s| NewSection {
            title: s.title.trim(),
            scoring: s.scoring.as_str(),
            points_per_question: round2(s.points_per_question),
            question_ids: &s.question_ids,
            question_points: s.question_ids.iter().map(|id| s.question_points.get(id).map(|p| round2(*p))).collect(),
            rules: s
                .rules
                .iter()
                .map(|r| NewRule {
                    tag_id: r.tag_id,
                    level: r.level.map(|l| l.as_str()),
                    kind: r.kind.map(QuestionKind::as_str),
                    folder_id: r.folder_id,
                    count: r.count,
                })
                .collect(),
        })
        .collect()
}

/// Điểm của câu cố định trong đề: điểm riêng đã đặt → tổng điểm các bước chấm (câu tự luận) → điểm mỗi câu của phần.
pub fn fixed_points(section_points: f64, kind: &str, body: &QuestionBody, custom: Option<f64>) -> f64 {
    if let Some(p) = custom {
        return p;
    }
    let total = body.rubric_total();
    if kind == QuestionKind::Essay.as_str() && total > 0.0 { round2(total) } else { section_points }
}

async fn build_response(ctx: &AppContext, exam: ExamRow) -> AppResult<ExamResponse> {
    let sections = repo::sections(&ctx.db, exam.id).await?;
    let section_ids: Vec<Uuid> = sections.iter().map(|s| s.id).collect();
    let fixed = repo::section_questions(&ctx.db, &section_ids).await?;
    let rules = repo::rules(&ctx.db, &section_ids).await?;
    let fixed_ids: Vec<Uuid> = fixed.iter().map(|f| f.question_id).collect();
    let mut by_id: HashMap<Uuid, _> =
        questions::find_many(ctx, &fixed_ids).await?.into_iter().map(|q| (q.id, q)).collect();

    let mut warnings = Vec::new();
    let mut out = Vec::with_capacity(sections.len());
    let (mut question_count, mut max_score) = (0i64, 0f64);
    for (si, s) in sections.into_iter().enumerate() {
        let mut qs: Vec<QuestionResponse> = Vec::new();
        let mut question_points = HashMap::new();
        for f in fixed.iter().filter(|f| f.section_id == s.id) {
            if let Some(p) = f.points {
                question_points.insert(f.question_id, p);
            }
            match by_id.remove(&f.question_id) {
                Some(q) if q.archived_at.is_some() => {
                    warnings.push(format!("Phần {}: có câu hỏi đã bị ẩn khỏi ngân hàng, sẽ không xuất hiện.", si + 1));
                }
                Some(q) => {
                    max_score += fixed_points(s.points_per_question, &q.kind, &q.body.0, f.points);
                    question_count += 1;
                    qs.push(q.into());
                }
                None => {}
            }
        }
        let mut rs = Vec::new();
        for r in rules.iter().filter(|r| r.section_id == s.id) {
            let pool = PoolFilter {
                owner_id: exam.owner_id,
                subject_id: exam.subject_id,
                tag_id: r.tag_id,
                level: r.level.clone(),
                kind: r.kind.clone(),
                folder_id: r.folder_id,
            };
            let available = questions::count_pool(ctx, &pool, &fixed_ids).await?;
            if available < i64::from(r.count) {
                warnings.push(format!(
                    "Phần {}, ô ma trận {}: ngân hàng chỉ có {available} câu phù hợp, cần {}.",
                    si + 1,
                    r.position + 1,
                    r.count
                ));
            }
            rs.push(RuleResponse {
                tag_id: r.tag_id,
                level: r.level.clone(),
                kind: r.kind.clone(),
                folder_id: r.folder_id,
                count: r.count,
                available,
            });
        }
        let drawn = rs.iter().map(|r| i64::from(r.count)).sum::<i64>();
        question_count += drawn;
        max_score += drawn as f64 * s.points_per_question;
        out.push(SectionResponse {
            id: s.id,
            title: s.title,
            scoring: s.scoring,
            points_per_question: s.points_per_question,
            questions: qs,
            rules: rs,
            question_points,
        });
    }
    Ok(ExamResponse {
        id: exam.id,
        subject_id: exam.subject_id,
        grade_id: exam.grade_id,
        title: exam.title,
        description: exam.description,
        kind: exam.kind,
        time_limit_minutes: exam.time_limit_minutes,
        max_attempts: exam.max_attempts,
        opens_at: exam.opens_at,
        closes_at: exam.closes_at,
        shuffle_questions: exam.shuffle_questions,
        shuffle_options: exam.shuffle_options,
        show_answers: exam.show_answers,
        sections: out,
        question_count,
        max_score: round2(max_score),
        warnings,
        attempt_count: exam.attempt_count,
        created_at: exam.created_at,
        updated_at: exam.updated_at,
    })
}

pub async fn list(ctx: &AppContext, actor: &Actor, query: ExamQuery) -> AppResult<Page<ExamSummary>> {
    actor.require(Permission::TeachingManage)?;
    let page = PageRequest::new(query.page, query.page_size);
    let filter = ExamFilter {
        owner_id: actor.id,
        q: query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()),
        subject_id: query.subject_id,
        kind: query.kind.map(ExamKind::as_str),
    };
    let rows = repo::list(&ctx.db, &filter, page.limit(), page.offset()).await?;
    let total = repo::count(&ctx.db, &filter).await?;
    let items = rows
        .into_iter()
        .map(|(e, n)| ExamSummary {
            id: e.id,
            subject_id: e.subject_id,
            grade_id: e.grade_id,
            title: e.title,
            kind: e.kind,
            time_limit_minutes: e.time_limit_minutes,
            question_count: n,
            attempt_count: e.attempt_count,
            updated_at: e.updated_at,
        })
        .collect();
    Ok(Page::new(items, total, page))
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<ExamResponse> {
    let exam = load_own_exam(ctx, actor, id).await?;
    build_response(ctx, exam).await
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: ExamInput) -> AppResult<ExamResponse> {
    actor.require(Permission::TeachingManage)?;
    validate_input(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    let id = repo::insert(&mut tx, actor.id, &exam_fields(&input)).await?;
    repo::replace_sections(&mut tx, id, &new_sections(&input)).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

/// Sửa đề. Lượt làm đã có giữ nguyên câu hỏi của nó (đã chép ra `attempt_items`).
pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: ExamInput) -> AppResult<ExamResponse> {
    load_own_exam(ctx, actor, id).await?;
    validate_input(ctx, actor, &input).await?;
    let mut tx = ctx.db.begin().await?;
    repo::update(&mut tx, id, &exam_fields(&input)).await?;
    repo::replace_sections(&mut tx, id, &new_sections(&input)).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

pub async fn archive(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    load_own_exam(ctx, actor, id).await?;
    repo::archive(&ctx.db, id).await?;
    Ok(())
}

pub async fn refs(ctx: &AppContext, ids: &[Uuid]) -> AppResult<Vec<ExamRef>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(repo::refs(&ctx.db, ids).await?)
}

// ---------- Bắt đầu làm bài ----------

/// Bối cảnh lượt làm: trong lớp học (bước nào) hay giáo viên làm thử.
#[derive(Debug, Clone, Copy)]
pub struct AttemptContext {
    pub class_id: Option<Uuid>,
    pub step_id: Option<Uuid>,
    pub preview: bool,
    /// Hạn làm của bước đề trong lớp (giao nhanh / lộ trình): quá hạn không bắt đầu lượt mới, lượt đang làm hết giờ
    /// đúng hạn.
    pub due_at: Option<DateTime<Utc>>,
}

/// Tạo lượt làm mới (hoặc trả lượt đang làm dở). Gọi **sau khi** đã kiểm tra quyền vào đề.
pub async fn start_attempt(ctx: &AppContext, user_id: Uuid, exam_id: Uuid, c: AttemptContext) -> AppResult<Uuid> {
    let exam = repo::find(&ctx.db, exam_id).await?.ok_or_else(not_found)?;
    if exam.archived_at.is_some() {
        return Err(AppError::conflict("EXAM_ARCHIVED", "Đề đã bị giáo viên ẩn."));
    }
    if let Some(existing) = repo::find_in_progress(&ctx.db, user_id, exam_id, c.class_id, c.preview).await? {
        let existing = finalize_if_overdue(ctx, existing).await?;
        if existing.status == "in_progress" {
            return Ok(existing.id);
        }
    }
    let now = Utc::now();
    if !c.preview {
        if let Some(open) = exam.opens_at
            && now < open
        {
            return Err(AppError::conflict(
                "EXAM_NOT_OPEN",
                format!(
                    "Đề chưa mở. Giờ mở: {}.",
                    open.with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).expect("UTC+7"))
                        .format("%H:%M %d/%m/%Y")
                ),
            ));
        }
        if exam.closes_at.is_some_and(|close| now >= close) {
            return Err(AppError::conflict("EXAM_CLOSED", "Đề đã đóng."));
        }
        if let Some(due) = c.due_at
            && now >= due
        {
            return Err(AppError::conflict(
                "STEP_OVERDUE",
                format!("Đã quá hạn làm bài ({}).", crate::modules::sessions::time::format_vn(due)),
            ));
        }
        if let Some(max) = exam.max_attempts
            && repo::count_attempts(&ctx.db, user_id, exam_id, c.class_id).await? >= i64::from(max)
        {
            return Err(AppError::conflict("ATTEMPT_LIMIT", format!("Bạn đã làm hết {max} lần cho phép.")));
        }
    }

    let planned = plan_items(ctx, &exam).await?;
    let max_score: f64 = planned.iter().map(|p| p.points).sum();
    let items: Vec<NewItem> = planned
        .iter()
        .enumerate()
        .map(|(i, p)| NewItem {
            position: i as i32,
            section_position: p.section_position,
            section_title: &p.section_title,
            scoring: &p.scoring,
            points: p.points,
            question_id: p.question_id,
            question_version_id: p.version_id,
            option_order: p.option_order.clone(),
            manual: p.manual,
        })
        .collect();

    let mut deadline = exam.time_limit_minutes.map(|m| now + TimeDelta::minutes(i64::from(m)));
    if !c.preview
        && let Some(close) = exam.closes_at
    {
        deadline = Some(deadline.map_or(close, |d| d.min(close)));
    }
    if !c.preview
        && let Some(due) = c.due_at
    {
        deadline = Some(deadline.map_or(due, |d| d.min(due)));
    }
    let mut tx = ctx.db.begin().await?;
    let id = repo::insert_attempt(
        &mut tx,
        &NewAttempt {
            exam_id,
            user_id,
            class_id: c.class_id,
            step_id: c.step_id,
            is_preview: c.preview,
            deadline_at: deadline,
            max_score: round2(max_score),
        },
        &items,
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Rút câu theo ma trận + trộn câu trong từng phần + trộn phương án (theo cài đặt của đề) cho một lượt làm / một
/// mã đề. Câu cố định đã bị ẩn khỏi ngân hàng thì bỏ qua.
pub async fn plan_items(ctx: &AppContext, exam: &ExamRow) -> AppResult<Vec<PaperItem>> {
    let sections = repo::sections(&ctx.db, exam.id).await?;
    let section_ids: Vec<Uuid> = sections.iter().map(|s| s.id).collect();
    let fixed = repo::section_questions(&ctx.db, &section_ids).await?;
    let rules = repo::rules(&ctx.db, &section_ids).await?;
    let fixed_ids: Vec<Uuid> = fixed.iter().map(|f| f.question_id).collect();
    let fixed_rows: HashMap<Uuid, bool> =
        questions::find_many(ctx, &fixed_ids).await?.into_iter().map(|q| (q.id, q.archived_at.is_none())).collect();
    // Câu cố định → điểm riêng (câu rút theo ma trận không có trong đây: điểm mỗi câu của phần).
    let custom_points: HashMap<Uuid, Option<f64>> = fixed.iter().map(|f| (f.question_id, f.points)).collect();

    let mut chosen: Vec<Uuid> = Vec::new();
    let mut plan: Vec<(usize, Vec<Uuid>)> = Vec::new();
    for (si, s) in sections.iter().enumerate() {
        let mut ids: Vec<Uuid> = fixed
            .iter()
            .filter(|f| f.section_id == s.id && fixed_rows.get(&f.question_id) == Some(&true))
            .map(|f| f.question_id)
            .collect();
        chosen.extend(&ids);
        for r in rules.iter().filter(|r| r.section_id == s.id) {
            let pool = PoolFilter {
                owner_id: exam.owner_id,
                subject_id: exam.subject_id,
                tag_id: r.tag_id,
                level: r.level.clone(),
                kind: r.kind.clone(),
                folder_id: r.folder_id,
            };
            let mut exclude = fixed_ids.clone();
            exclude.extend(&chosen);
            let drawn = questions::draw(ctx, &pool, &exclude, i64::from(r.count)).await?;
            chosen.extend(&drawn);
            ids.extend(drawn);
        }
        if exam.shuffle_questions {
            shuffle(&mut ids);
        }
        plan.push((si, ids));
    }
    let all: Vec<Uuid> = plan.iter().flat_map(|(_, ids)| ids.clone()).collect();
    if all.is_empty() {
        return Err(AppError::conflict("EXAM_EMPTY", "Đề chưa có câu hỏi nào."));
    }
    let rows: HashMap<Uuid, _> = questions::find_many(ctx, &all).await?.into_iter().map(|q| (q.id, q)).collect();

    let mut items = Vec::with_capacity(all.len());
    for (si, ids) in &plan {
        let s = &sections[*si];
        for id in ids {
            let Some(q) = rows.get(id) else { continue };
            let Some(version_id) = q.current_version_id else { continue };
            let body = &q.body.0;
            let mut order: Vec<Uuid> = body.options.iter().map(|o| o.id).collect();
            let choice = matches!(q.kind.as_str(), "single" | "multiple");
            if choice && exam.shuffle_options && body.shuffle_options {
                shuffle(&mut order);
            }
            let points = match custom_points.get(&q.id) {
                Some(custom) => fixed_points(s.points_per_question, &q.kind, body, *custom),
                None => s.points_per_question,
            };
            items.push(PaperItem {
                section_position: s.position,
                section_title: s.title.clone(),
                scoring: s.scoring.clone(),
                points,
                question_id: q.id,
                version_id,
                option_order: order,
                manual: q.kind == QuestionKind::Essay.as_str(),
            });
        }
    }
    Ok(items)
}

/// Giáo viên làm thử đề của mình.
pub async fn preview(ctx: &AppContext, actor: &Actor, exam_id: Uuid) -> AppResult<AttemptView> {
    load_own_exam(ctx, actor, exam_id).await?;
    let id = start_attempt(
        ctx,
        actor.id,
        exam_id,
        AttemptContext { class_id: None, step_id: None, preview: true, due_at: None },
    )
    .await?;
    get_attempt(ctx, actor, id).await
}

// ---------- Làm bài ----------

/// Người xem là học sinh làm bài hay giáo viên chủ đề.
async fn load_attempt(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<(AttemptRow, ExamRow, bool)> {
    let attempt = repo::find_attempt(&ctx.db, id).await?.ok_or_else(attempt_not_found)?;
    let exam = repo::find(&ctx.db, attempt.exam_id).await?.ok_or_else(attempt_not_found)?;
    if attempt.user_id == actor.id {
        return Ok((attempt, exam, false));
    }
    if can_grade(ctx, actor, &exam, &attempt).await? {
        return Ok((attempt, exam, true));
    }
    Err(attempt_not_found())
}

/// Giáo viên chủ đề, hoặc trợ giảng của lớp học có lượt làm này: xem bài, chấm câu tự luận.
async fn can_grade(ctx: &AppContext, actor: &Actor, exam: &ExamRow, attempt: &AttemptRow) -> AppResult<bool> {
    if exam.owner_id == actor.id {
        return Ok(true);
    }
    match attempt.class_id {
        Some(class_id) => classes::is_assistant(&ctx.db, class_id, actor.id).await,
        None => Ok(false),
    }
}

fn is_overdue(a: &AttemptRow) -> bool {
    a.status == "in_progress" && a.deadline_at.is_some_and(|d| d + TimeDelta::seconds(DEADLINE_GRACE_SECS) < Utc::now())
}

async fn finalize_if_overdue(ctx: &AppContext, attempt: AttemptRow) -> AppResult<AttemptRow> {
    if !is_overdue(&attempt) {
        return Ok(attempt);
    }
    submit_internal(ctx, attempt.id).await?;
    repo::find_attempt(&ctx.db, attempt.id).await?.ok_or_else(attempt_not_found)
}

/// Phiên bản câu hỏi của các câu trong lượt làm: version id → (dạng câu, nội dung).
async fn versions_of(ctx: &AppContext, ids: Vec<Uuid>) -> AppResult<HashMap<Uuid, (QuestionKind, QuestionBody)>> {
    Ok(questions::find_versions(ctx, &ids)
        .await?
        .into_iter()
        .filter_map(|v| v.kind.parse().ok().map(|k| (v.id, (k, v.body.0))))
        .collect())
}

/// Chấm toàn bộ và nộp bài.
async fn submit_internal(ctx: &AppContext, attempt_id: Uuid) -> AppResult<()> {
    let items = repo::items(&ctx.db, attempt_id).await?;
    let versions = versions_of(ctx, items.iter().map(|i| i.question_version_id).collect()).await?;
    let mut tx = ctx.db.begin().await?;
    let mut total = 0.0;
    for it in &items {
        // Câu tự luận: giữ 0 điểm "chờ chấm" cho giáo viên.
        if it.manual {
            total += it.score;
            continue;
        }
        let Some((kind, body)) = versions.get(&it.question_version_id) else { continue };
        let scoring = Scoring::parse(&it.scoring).unwrap_or(Scoring::PerQuestion);
        let g = grading::grade(*kind, body, it.answer.as_ref().map(|a| &a.0), scoring, it.points);
        total += g.score;
        repo::set_item_grade(&mut *tx, attempt_id, it.position, g.is_correct, g.score, false).await?;
    }
    repo::mark_submitted(&mut *tx, attempt_id, round2(total)).await?;
    tx.commit().await?;
    Ok(())
}

fn can_reveal(exam: &ExamRow, attempt: &AttemptRow, teacher_view: bool) -> bool {
    if teacher_view || (attempt.is_preview && attempt.user_id == exam.owner_id && attempt.status == "submitted") {
        return true;
    }
    if attempt.status != "submitted" {
        return false;
    }
    match exam_kind(exam) {
        ExamKind::Practice => true,
        ExamKind::Test => match show_answers(exam) {
            ShowAnswers::AfterSubmit => true,
            ShowAnswers::AfterClose => exam.closes_at.is_none_or(|c| Utc::now() >= c),
            ShowAnswers::Never => false,
        },
    }
}

async fn build_view(
    ctx: &AppContext,
    attempt: AttemptRow,
    exam: ExamRow,
    teacher_view: bool,
) -> AppResult<AttemptView> {
    let items = repo::items(&ctx.db, attempt.id).await?;
    let versions = versions_of(ctx, items.iter().map(|i| i.question_version_id).collect()).await?;
    let reveal = can_reveal(&exam, &attempt, teacher_view);
    let submitted = attempt.status == "submitted";
    let student = if teacher_view {
        users::find_by_id(&ctx.db, attempt.user_id).await?.map(|u| StudentInfo {
            id: u.id,
            username: u.username,
            full_name: u.full_name,
        })
    } else {
        None
    };
    // Ảnh bài làm câu tự luận + ảnh giáo viên đã vẽ nhận xét (link có chữ ký).
    let mut file_ids: Vec<Uuid> = Vec::new();
    for it in &items {
        if let Some(a) = &it.answer {
            file_ids.extend(&a.0.files);
        }
        if let Some(an) = &it.annotations {
            file_ids.extend(an.0.iter().map(|x| x.annotated_file_id));
        }
    }
    let file_views = if file_ids.is_empty() { HashMap::new() } else { files::views(ctx, &file_ids).await? };
    let mut pending = 0;
    let mut views = Vec::with_capacity(items.len());
    for it in items {
        let Some((kind, body)) = versions.get(&it.question_version_id) else { continue };
        let options = it
            .option_order
            .iter()
            .filter_map(|id| body.options.iter().find(|o| o.id == *id))
            .map(|o| ViewOption { id: o.id, content: o.content.clone() })
            .collect();
        let answer = it.answer.map(|a| a.0);
        let annotations = it.annotations.map(|a| a.0).unwrap_or_default();
        let answer_files = answer
            .as_ref()
            .map(|a| {
                a.files
                    .iter()
                    .filter_map(|f| {
                        let annotated = annotations
                            .iter()
                            .find(|x| x.file_id == *f)
                            .and_then(|x| file_views.get(&x.annotated_file_id).cloned());
                        Some(AnswerFile { file: file_views.get(f)?.clone(), annotated })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let item_pending = it.manual && it.graded_at.is_none();
        if submitted && item_pending {
            pending += 1;
        }
        let grading = (it.manual && submitted).then(|| ItemGrading {
            pending: item_pending,
            score: (!item_pending).then_some(it.score),
            marks: it.marks.map(|m| m.0).unwrap_or_default(),
            feedback: it.feedback,
            graded_at: it.graded_at,
        });
        let show_result = reveal || it.checked;
        views.push(ItemView {
            position: it.position,
            section_position: it.section_position,
            section_title: it.section_title,
            points: it.points,
            kind: kind.as_str().to_string(),
            content: body.content.clone(),
            options,
            answer,
            checked: it.checked,
            result: show_result.then(|| ItemResult {
                is_correct: it.is_correct.unwrap_or(false),
                score: it.score,
                correct: body.correct.clone(),
                accepted: body.accepted.clone(),
                explanation: body.explanation.clone(),
                rubric: body.rubric.clone(),
            }),
            manual: it.manual,
            expression: *kind == QuestionKind::Short && body.match_mode == MatchMode::Expression,
            files: answer_files,
            grading,
        });
    }
    Ok(AttemptView {
        id: attempt.id,
        exam_id: exam.id,
        class_id: attempt.class_id,
        step_id: attempt.step_id,
        title: exam.title.clone(),
        exam_kind: exam.kind.clone(),
        is_preview: attempt.is_preview,
        can_check: !submitted && !teacher_view && exam_kind(&exam) == ExamKind::Practice,
        status: attempt.status,
        started_at: attempt.started_at,
        deadline_at: attempt.deadline_at,
        now: Utc::now(),
        submitted_at: attempt.submitted_at,
        score: submitted.then_some(attempt.score),
        max_score: attempt.max_score,
        reveal,
        student,
        pending,
        can_grade: teacher_view && submitted,
        items: views,
    })
}

pub async fn get_attempt(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<AttemptView> {
    let (attempt, exam, teacher_view) = load_attempt(ctx, actor, id).await?;
    let attempt = finalize_if_overdue(ctx, attempt).await?;
    build_view(ctx, attempt, exam, teacher_view).await
}

/// Bài của chính mình, đang làm, chưa quá hạn.
async fn load_writable(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<(AttemptRow, ExamRow)> {
    let (attempt, exam, teacher_view) = load_attempt(ctx, actor, id).await?;
    if teacher_view {
        return Err(attempt_not_found());
    }
    let attempt = finalize_if_overdue(ctx, attempt).await?;
    if attempt.status != "in_progress" {
        return Err(AppError::conflict("ATTEMPT_CLOSED", "Bài đã nộp (hoặc đã hết giờ và được tự động nộp)."));
    }
    Ok((attempt, exam))
}

pub async fn save_answer(ctx: &AppContext, actor: &Actor, id: Uuid, position: i32, answer: Answer) -> AppResult<()> {
    let (attempt, _) = load_writable(ctx, actor, id).await?;
    let items = repo::items(&ctx.db, attempt.id).await?;
    let item = items.iter().find(|i| i.position == position).ok_or_else(|| AppError::not_found("Không có câu này."))?;
    let versions = versions_of(ctx, vec![item.question_version_id]).await?;
    let (kind, _) =
        versions.get(&item.question_version_id).ok_or_else(|| AppError::internal("thiếu phiên bản câu hỏi"))?;
    let invalid = || AppError::bad_request("INVALID_ANSWER", "Câu trả lời không hợp lệ.");
    let essay = kind.is_manual();
    let max_text = if essay { MAX_ESSAY_TEXT } else { 500 };
    if answer.selected.iter().any(|s| !item.option_order.contains(s))
        || answer.truth.keys().any(|k| !item.option_order.contains(k))
        || answer.text.as_deref().is_some_and(|t| t.chars().count() > max_text)
        || (*kind == QuestionKind::Single && answer.selected.len() > 1)
        || (!essay && !answer.files.is_empty())
    {
        return Err(invalid());
    }
    if essay && !answer.files.is_empty() {
        let mut ids = answer.files.clone();
        ids.sort();
        ids.dedup();
        if ids.len() != answer.files.len() || ids.len() > MAX_ESSAY_FILES {
            return Err(AppError::bad_request(
                "INVALID_ANSWER",
                format!("Mỗi câu tự luận nộp tối đa {MAX_ESSAY_FILES} ảnh / file, không trùng nhau."),
            ));
        }
        files::owned_ready(ctx, actor.id, &ids, Purpose::Homework).await?;
    }
    if !repo::save_answer(&ctx.db, attempt.id, position, &answer).await? {
        return Err(AppError::conflict("ITEM_LOCKED", "Câu này đã kiểm tra đáp án, không sửa được nữa."));
    }
    Ok(())
}

/// Ôn tập: chấm ngay một câu, khóa câu đó và trả lời giải.
pub async fn check_item(ctx: &AppContext, actor: &Actor, id: Uuid, position: i32) -> AppResult<AttemptView> {
    let (attempt, exam) = load_writable(ctx, actor, id).await?;
    if exam_kind(&exam) != ExamKind::Practice {
        return Err(AppError::bad_request("NOT_PRACTICE", "Chỉ đề ôn tập mới kiểm tra từng câu được."));
    }
    let items = repo::items(&ctx.db, attempt.id).await?;
    let item = items.iter().find(|i| i.position == position).ok_or_else(|| AppError::not_found("Không có câu này."))?;
    if item.manual {
        return Err(AppError::bad_request("MANUAL_ITEM", "Câu tự luận do giáo viên chấm, không kiểm tra ngay được."));
    }
    if !item.checked {
        let versions = versions_of(ctx, vec![item.question_version_id]).await?;
        let (kind, body) =
            versions.get(&item.question_version_id).ok_or_else(|| AppError::internal("thiếu phiên bản câu hỏi"))?;
        let scoring = Scoring::parse(&item.scoring).unwrap_or(Scoring::PerQuestion);
        let g = grading::grade(*kind, body, item.answer.as_ref().map(|a| &a.0), scoring, item.points);
        repo::set_item_grade(&ctx.db, attempt.id, position, g.is_correct, g.score, true).await?;
    }
    get_attempt(ctx, actor, id).await
}

pub async fn submit(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<AttemptView> {
    let (attempt, _) = load_writable(ctx, actor, id).await?;
    submit_internal(ctx, attempt.id).await?;
    get_attempt(ctx, actor, id).await
}

// ---------- Kết quả ----------

/// Giáo viên xem các lượt làm của một đề (lọc theo lớp học).
pub async fn exam_attempts(
    ctx: &AppContext,
    actor: &Actor,
    exam_id: Uuid,
    q: AttemptQuery,
) -> AppResult<Vec<AttemptSummary>> {
    load_own_exam(ctx, actor, exam_id).await?;
    Ok(repo::list_for_exam(&ctx.db, exam_id, q.class_id, None).await?.into_iter().map(Into::into).collect())
}

/// Các lượt làm của một học sinh với đề trong lớp học.
pub async fn my_attempts(
    ctx: &AppContext,
    user_id: Uuid,
    exam_id: Uuid,
    class_id: Uuid,
) -> AppResult<Vec<AttemptSummary>> {
    Ok(repo::list_for_exam(&ctx.db, exam_id, Some(class_id), Some(user_id))
        .await?
        .into_iter()
        .filter(|a| !a.is_preview)
        .map(Into::into)
        .collect())
}

/// Điểm cao nhất theo học sinh × đề trong lớp học (tự nộp các bài đã quá giờ trước).
pub async fn best_scores(
    ctx: &AppContext,
    class_id: Uuid,
    exam_ids: &[Uuid],
    user_id: Option<Uuid>,
) -> AppResult<Vec<BestScoreRow>> {
    if exam_ids.is_empty() {
        return Ok(Vec::new());
    }
    for id in repo::overdue_in_progress(&ctx.db, class_id, user_id, DEADLINE_GRACE_SECS).await? {
        submit_internal(ctx, id).await?;
    }
    Ok(repo::best_scores(&ctx.db, class_id, exam_ids, user_id).await?)
}

/// Chấm lại mọi câu `question_id` trong bài đã nộp theo phiên bản `version_id` (đáp án mới).
pub async fn regrade_question(
    ctx: &AppContext,
    question_id: Uuid,
    kind: QuestionKind,
    body: &QuestionBody,
    version_id: Uuid,
) -> AppResult<RegradeResponse> {
    // Câu tự luận giáo viên chấm tay: không chấm lại tự động (sẽ xóa điểm đã chấm).
    if kind.is_manual() {
        return Ok(RegradeResponse { regraded_attempts: 0, changed_attempts: 0 });
    }
    let items = repo::submitted_items_for_question(&ctx.db, question_id).await?;
    let mut attempts: Vec<Uuid> = Vec::new();
    let mut changed: Vec<Uuid> = Vec::new();
    let mut tx = ctx.db.begin().await?;
    for it in &items {
        let scoring = Scoring::parse(&it.scoring).unwrap_or(Scoring::PerQuestion);
        let g = grading::grade(kind, body, it.answer.as_ref().map(|a| &a.0), scoring, it.points);
        repo::set_item_version(&mut *tx, it.attempt_id, it.position, version_id, g.is_correct, g.score).await?;
        if !attempts.contains(&it.attempt_id) {
            attempts.push(it.attempt_id);
        }
        if (g.score - it.score).abs() > 1e-9 && !changed.contains(&it.attempt_id) {
            changed.push(it.attempt_id);
        }
    }
    repo::recompute_scores(&mut *tx, &attempts).await?;
    tx.commit().await?;
    Ok(RegradeResponse { regraded_attempts: attempts.len() as i64, changed_attempts: changed.len() as i64 })
}

// ---------- Chấm tự luận (M39) ----------

/// Giáo viên chủ đề / trợ giảng của lớp chấm một câu tự luận của bài đã nộp.
pub async fn grade_item(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    position: i32,
    input: GradeItemInput,
) -> AppResult<AttemptView> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    let (attempt, exam, grader) = load_attempt(ctx, actor, id).await?;
    if !grader {
        return Err(attempt_not_found());
    }
    let attempt = finalize_if_overdue(ctx, attempt).await?;
    if attempt.status != "submitted" {
        return Err(AppError::conflict("ATTEMPT_NOT_SUBMITTED", "Học sinh chưa nộp bài."));
    }
    let items = repo::items(&ctx.db, attempt.id).await?;
    let item = items.iter().find(|i| i.position == position).ok_or_else(|| AppError::not_found("Không có câu này."))?;
    if !item.manual {
        return Err(AppError::bad_request("NOT_MANUAL", "Câu này được chấm tự động."));
    }
    let versions = versions_of(ctx, vec![item.question_version_id]).await?;
    let (_, body) =
        versions.get(&item.question_version_id).ok_or_else(|| AppError::internal("thiếu phiên bản câu hỏi"))?;

    let mut marks = input.marks.clone();
    for (step, p) in marks.iter_mut() {
        let Some(s) = body.rubric.iter().find(|s| s.id == *step) else {
            return Err(AppError::field("marks", "Có bước chấm không thuộc câu này."));
        };
        if !(0.0..=s.points + 1e-9).contains(p) {
            return Err(AppError::field("marks", format!("Mỗi bước chỉ cho từ 0 đến {} điểm.", s.points)));
        }
        *p = round2(*p);
    }
    if body.rubric.is_empty() && !input.score.is_some_and(|s| (0.0..=item.points + 1e-9).contains(&s)) {
        return Err(AppError::field("score", format!("Nhập điểm từ 0 đến {}.", item.points)));
    }
    // Ảnh đã vẽ: ảnh gốc phải là ảnh bài làm của câu; ảnh vẽ mới phải do người chấm tải lên.
    let answer_files = item.answer.as_ref().map(|a| a.0.files.clone()).unwrap_or_default();
    if input.annotations.iter().any(|a| !answer_files.contains(&a.file_id)) {
        return Err(AppError::field("annotations", "Ảnh không thuộc bài làm của câu này."));
    }
    let old: Vec<Uuid> =
        item.annotations.as_ref().map(|a| a.0.iter().map(|x| x.annotated_file_id).collect()).unwrap_or_default();
    let drawn: Vec<Uuid> = input.annotations.iter().map(|a| a.annotated_file_id).filter(|f| !old.contains(f)).collect();
    if !drawn.is_empty() {
        files::owned_ready(ctx, actor.id, &drawn, Purpose::Homework).await?;
    }

    let score = grading::manual_score(body, item.points, &marks, input.score);
    let mut tx = ctx.db.begin().await?;
    repo::set_manual_grade(
        &mut *tx,
        attempt.id,
        position,
        &ManualGrade {
            marks: &marks,
            feedback: input.feedback.trim(),
            annotations: &input.annotations,
            score,
            is_correct: item.points > 0.0 && score >= item.points - 1e-9,
            graded_by: actor.id,
        },
    )
    .await?;
    repo::recompute_scores(&mut *tx, &[attempt.id]).await?;
    let pending = repo::pending_count(&mut *tx, attempt.id).await?;
    tx.commit().await?;
    // Vừa chấm xong câu cuối cùng còn chờ → báo học sinh (chấm lại sau đó không báo nữa).
    if pending == 0 && item.graded_at.is_none() && !attempt.is_preview {
        let link = format!("/bai-lam/{}", attempt.id);
        notifications::notify(
            &ctx.db,
            &[attempt.user_id],
            NotificationKind::AttemptGraded,
            &format!("Bài \"{}\" đã chấm xong", exam.title),
            "Giáo viên đã chấm phần tự luận. Mở bài để xem điểm và nhận xét.",
            Some(&link),
        )
        .await?;
    }
    get_attempt(ctx, actor, id).await
}

/// Bài đã nộp còn câu tự luận chưa chấm (đề của mình + lớp mình làm trợ giảng), bài nộp trước lên trước.
pub async fn grading_queue(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<QueueItem>> {
    actor.require(Permission::TeachingManage)?;
    Ok(repo::grading_queue(&ctx.db, actor.id).await?.into_iter().map(Into::into).collect())
}

// ---------- Mã đề (M40) ----------

/// Tạo `count` mã đề từ đề (mỗi mã rút + trộn riêng theo cài đặt của đề), thay bộ mã đề cũ.
pub async fn generate_papers(
    ctx: &AppContext,
    actor: &Actor,
    exam_id: Uuid,
    input: PapersInput,
) -> AppResult<PapersResponse> {
    input.validate()?;
    let exam = load_own_exam(ctx, actor, exam_id).await?;
    let mut papers = Vec::with_capacity(input.count as usize);
    for i in 0..input.count {
        papers.push(((input.first_code + i).to_string(), plan_items(ctx, &exam).await?));
    }
    let mut tx = ctx.db.begin().await?;
    repo::replace_papers(&mut tx, exam_id, &papers).await?;
    tx.commit().await?;
    papers_of(ctx, exam).await
}

/// Bộ mã đề đã tạo (kèm nội dung câu + đáp án) để in / xuất Word.
pub async fn papers(ctx: &AppContext, actor: &Actor, exam_id: Uuid) -> AppResult<PapersResponse> {
    let exam = load_own_exam(ctx, actor, exam_id).await?;
    papers_of(ctx, exam).await
}

async fn papers_of(ctx: &AppContext, exam: ExamRow) -> AppResult<PapersResponse> {
    let rows = repo::papers(&ctx.db, exam.id).await?;
    let version_ids: Vec<Uuid> = rows.iter().flat_map(|r| r.items.0.iter().map(|i| i.version_id)).collect();
    let versions = versions_of(ctx, version_ids).await?;
    let created_at = rows.first().map(|r| r.created_at);
    let max_score = rows.first().map_or(0.0, |r| round2(r.items.0.iter().map(|i| i.points).sum()));
    let papers = rows
        .into_iter()
        .map(|r| {
            let mut number = 0;
            let mut section = None;
            let items = r
                .items
                .0
                .into_iter()
                .filter_map(|it| {
                    let (kind, body) = versions.get(&it.version_id)?;
                    if section != Some(it.section_position) {
                        section = Some(it.section_position);
                        number = 0;
                    }
                    number += 1;
                    let options = it
                        .option_order
                        .iter()
                        .filter_map(|id| body.options.iter().find(|o| o.id == *id))
                        .map(|o| PaperOption {
                            id: o.id,
                            content: o.content.clone(),
                            correct: body.correct.contains(&o.id),
                        })
                        .collect();
                    Some(PaperQuestion {
                        number,
                        section_position: it.section_position,
                        section_title: it.section_title,
                        points: it.points,
                        kind: kind.as_str().to_string(),
                        content: body.content.clone(),
                        options,
                        accepted: body.accepted.clone(),
                        explanation: body.explanation.clone(),
                        rubric: body.rubric.clone(),
                    })
                })
                .collect();
            PaperView { id: r.id, code: r.code, items }
        })
        .collect();
    Ok(PapersResponse {
        exam_id: exam.id,
        title: exam.title,
        subject_id: exam.subject_id,
        time_limit_minutes: exam.time_limit_minutes,
        max_score,
        created_at,
        papers,
    })
}
