//! Bài tập về nhà trong lớp học (M22, D2 + D9): học sinh nộp ảnh / PDF bài làm, giáo viên chấm điểm,
//! nhận xét, vẽ nhận xét lên ảnh hoặc trả lại để làm lại; nhắc học sinh chưa nộp trước hạn 24 giờ.
//!
//! - Mỗi học sinh một bài nộp cho mỗi bước bài tập; nộp lại được khi chưa chấm hoặc bị trả lại (thay toàn bộ file).
//! - Nộp sau hạn vẫn nhận, đánh dấu "nộp muộn".
//! - File bài làm là file `homework` (link có chữ ký): chỉ học sinh đó và giáo viên chủ lớp nhận được link.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::{
    assistants,
    model::{ClassRow, Step, StepKind},
    path, repository as repo,
    service::{load_staff_class, not_found},
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    jobs::{Email, Job},
    modules::{
        assignments::{self, AssignmentRef},
        files::{self, FileView, Purpose},
        notifications::{self, NotificationKind},
    },
    security::Actor,
};

/// Tối đa số file mỗi bài nộp.
pub const MAX_FILES: usize = 10;
/// Nhắc học sinh chưa nộp khi còn ≤ 24 giờ tới hạn.
const REMIND_BEFORE_HOURS: i64 = 24;

// ---------- DTO ----------

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SubmitInput {
    #[serde(default)]
    #[validate(length(max = 2000, message = "Lời nhắn tối đa 2.000 ký tự."))]
    pub note: String,
    pub file_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeAction {
    /// Chấm xong (điểm + nhận xét).
    Grade,
    /// Trả lại để học sinh làm lại / nộp thêm.
    Return,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationInput {
    pub position: i32,
    /// Ảnh giáo viên đã vẽ nhận xét (file `homework` của giáo viên); `null` = bỏ ảnh đã vẽ.
    pub file_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GradeInput {
    pub action: GradeAction,
    pub score: Option<f64>,
    #[serde(default)]
    #[validate(length(max = 10000, message = "Nhận xét tối đa 10.000 ký tự."))]
    pub feedback: String,
    #[serde(default)]
    pub annotations: Vec<AnnotationInput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionFile {
    pub position: i32,
    pub file: FileView,
    /// Ảnh có nét nhận xét của giáo viên.
    pub annotated: Option<FileView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionView {
    pub id: Uuid,
    pub class_id: Uuid,
    pub step_id: Uuid,
    pub student_id: Uuid,
    pub username: String,
    pub full_name: String,
    /// submitted | returned | graded
    pub status: String,
    pub note: String,
    pub submitted_at: DateTime<Utc>,
    pub late: bool,
    pub submit_count: i32,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub feedback: String,
    pub graded_at: Option<DateTime<Utc>>,
    pub files: Vec<SubmissionFile>,
}

/// Bước bài tập + bài nộp của chính mình (học sinh) hoặc bản xem thử (giáo viên).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeworkView {
    pub class_id: Uuid,
    pub class_title: String,
    pub step_id: Uuid,
    pub title: String,
    pub instructions: String,
    pub max_score: Option<f64>,
    pub due_at: Option<DateTime<Utc>>,
    pub overdue: bool,
    pub submission: Option<SubmissionView>,
    pub can_submit: bool,
    pub is_preview: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeworkStepSummary {
    pub step_id: Uuid,
    pub title: String,
    pub due_at: Option<DateTime<Utc>>,
    pub max_score: Option<f64>,
    /// Số học sinh được giao (lộ trình riêng).
    pub assigned: i64,
    pub submitted: i64,
    pub graded: i64,
    pub returned: i64,
    pub late: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionBrief {
    pub id: Uuid,
    pub status: String,
    pub submitted_at: DateTime<Utc>,
    pub late: bool,
    pub score: Option<f64>,
    pub file_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentSubmissionRow {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub submission: Option<SubmissionBrief>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepSubmissions {
    pub step: HomeworkStepSummary,
    pub instructions: String,
    pub students: Vec<StudentSubmissionRow>,
}

/// Trạng thái bài nộp (tiến độ lộ trình).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StatusRow {
    pub student_id: Uuid,
    pub step_id: Uuid,
    pub status: String,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct SubmissionRow {
    id: Uuid,
    class_id: Uuid,
    step_id: Uuid,
    assignment_id: Uuid,
    student_id: Uuid,
    username: String,
    full_name: String,
    status: String,
    note: String,
    submitted_at: DateTime<Utc>,
    late: bool,
    submit_count: i32,
    score: Option<f64>,
    feedback: String,
    graded_at: Option<DateTime<Utc>>,
}

const SUBMISSION_COLS: &str = "s.id, s.class_id, s.step_id, s.assignment_id, s.student_id, u.username, u.full_name, \
     s.status, s.note, s.submitted_at, s.late, s.submit_count, s.score, s.feedback, s.graded_at";

fn submission_not_found() -> AppError {
    AppError::not_found("Không tìm thấy bài nộp.")
}

// ---------- Truy vấn ----------

pub(super) async fn statuses(ctx: &AppContext, class_id: Uuid, user_id: Option<Uuid>) -> AppResult<Vec<StatusRow>> {
    Ok(sqlx::query_as::<_, StatusRow>(
        "SELECT student_id, step_id, status, score FROM homework_submissions \
         WHERE class_id = $1 AND ($2::uuid IS NULL OR student_id = $2)",
    )
    .bind(class_id)
    .bind(user_id)
    .fetch_all(&ctx.db)
    .await?)
}

async fn find_submission(
    ctx: &AppContext,
    where_sql: &str,
    a: Uuid,
    b: Option<(Uuid, Uuid)>,
) -> AppResult<Option<SubmissionRow>> {
    let mut q = sqlx::query_as::<_, SubmissionRow>(crate::sql!(
        "SELECT {SUBMISSION_COLS} FROM homework_submissions s JOIN users u ON u.id = s.student_id WHERE {where_sql}"
    ))
    .bind(a);
    if let Some((x, y)) = b {
        q = q.bind(x).bind(y);
    }
    Ok(q.fetch_optional(&ctx.db).await?)
}

async fn view(ctx: &AppContext, row: SubmissionRow, max_score: Option<f64>) -> AppResult<SubmissionView> {
    let links: Vec<(i32, Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT position, file_id, annotated_file_id FROM submission_files WHERE submission_id = $1 ORDER BY position",
    )
    .bind(row.id)
    .fetch_all(&ctx.db)
    .await?;
    let ids: Vec<Uuid> = links.iter().flat_map(|(_, f, a)| std::iter::once(*f).chain(*a)).collect();
    let mut views = files::views(ctx, &ids).await?;
    let files = links
        .into_iter()
        .filter_map(|(position, f, a)| {
            Some(SubmissionFile { position, file: views.get(&f)?.clone(), annotated: a.and_then(|a| views.remove(&a)) })
        })
        .collect();
    Ok(SubmissionView {
        id: row.id,
        class_id: row.class_id,
        step_id: row.step_id,
        student_id: row.student_id,
        username: row.username,
        full_name: row.full_name,
        status: row.status,
        note: row.note,
        submitted_at: row.submitted_at,
        late: row.late,
        submit_count: row.submit_count,
        score: row.score,
        max_score,
        feedback: row.feedback,
        graded_at: row.graded_at,
        files,
    })
}

async fn assignment(ctx: &AppContext, id: Uuid) -> AppResult<AssignmentRef> {
    assignments::refs(ctx, &[id]).await?.pop().ok_or_else(|| AppError::not_found("Không tìm thấy bài tập."))
}

/// Bước bài tập trong lộ trình đã xuất bản.
async fn published_step(ctx: &AppContext, class_id: Uuid, step_id: Uuid) -> AppResult<Step> {
    path::published_steps(ctx, class_id)
        .await?
        .steps
        .into_iter()
        .find(|s| s.id == step_id && s.kind == StepKind::Homework)
        .ok_or_else(|| AppError::not_found("Không có bài tập này trong lộ trình."))
}

fn step_title(step: &Step, a: &AssignmentRef) -> String {
    if step.title.trim().is_empty() { a.title.clone() } else { step.title.clone() }
}

// ---------- Học sinh ----------

pub async fn student_view(ctx: &AppContext, actor: &Actor, class_id: Uuid, step_id: Uuid) -> AppResult<HomeworkView> {
    let (l, steps, idx) = path::open_step(ctx, actor, class_id, step_id).await?;
    let step = &steps[idx];
    let assignment_id = step.assignment_id.ok_or_else(|| AppError::not_found("Bước này không phải bài tập."))?;
    let a = assignment(ctx, assignment_id).await?;
    let submission = if l.preview {
        None
    } else {
        match find_submission(
            ctx,
            "s.class_id = $1 AND s.step_id = $2 AND s.student_id = $3",
            class_id,
            Some((step_id, actor.id)),
        )
        .await?
        {
            Some(row) => Some(view(ctx, row, a.max_score).await?),
            None => None,
        }
    };
    Ok(HomeworkView {
        class_id,
        class_title: l.class.title.clone(),
        step_id,
        title: step.title.clone(),
        instructions: a.instructions,
        max_score: a.max_score,
        due_at: step.due_at,
        overdue: step.due_at.is_some_and(|d| d < Utc::now()),
        can_submit: !l.preview && submission.as_ref().is_none_or(|s| s.status != "graded"),
        submission,
        is_preview: l.preview,
    })
}

/// Nộp (hoặc nộp lại) bài: thay toàn bộ file của bài nộp cũ.
pub async fn submit(
    ctx: &AppContext,
    actor: &Actor,
    class_id: Uuid,
    step_id: Uuid,
    input: SubmitInput,
) -> AppResult<HomeworkView> {
    input.validate()?;
    let (l, steps, idx) = path::open_step(ctx, actor, class_id, step_id).await?;
    if l.preview {
        return Err(AppError::forbidden("PREVIEW_ONLY", "Giáo viên xem thử không nộp bài được."));
    }
    let step = &steps[idx];
    let assignment_id = step.assignment_id.ok_or_else(|| AppError::not_found("Bước này không phải bài tập."))?;
    if input.file_ids.is_empty() {
        return Err(AppError::field("fileIds", "Hãy chọn ít nhất một ảnh hoặc file PDF bài làm."));
    }
    if input.file_ids.len() > MAX_FILES {
        return Err(AppError::field("fileIds", format!("Tối đa {MAX_FILES} file mỗi bài nộp.")));
    }
    let mut ids = input.file_ids.clone();
    let mut seen = std::collections::HashSet::new();
    ids.retain(|id| seen.insert(*id));
    let rows = files::owned_ready(ctx, actor.id, &ids, Purpose::Homework).await?;
    let late = step.due_at.is_some_and(|d| Utc::now() > d);

    let mut tx = ctx.db.begin().await?;
    let id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO homework_submissions (class_id, step_id, assignment_id, student_id, note, late) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (class_id, step_id, student_id) DO UPDATE SET status = 'submitted', note = EXCLUDED.note, \
           submitted_at = now(), late = EXCLUDED.late, submit_count = homework_submissions.submit_count + 1, \
           assignment_id = EXCLUDED.assignment_id, updated_at = now() \
         WHERE homework_submissions.status <> 'graded' RETURNING id",
    )
    .bind(class_id)
    .bind(step_id)
    .bind(assignment_id)
    .bind(actor.id)
    .bind(input.note.trim())
    .bind(late)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(id) = id else {
        return Err(AppError::conflict("HOMEWORK_GRADED", "Bài đã được chấm, không nộp lại được."));
    };
    sqlx::query("DELETE FROM submission_files WHERE submission_id = $1").bind(id).execute(&mut *tx).await?;
    for (i, f) in rows.iter().enumerate() {
        sqlx::query("INSERT INTO submission_files (submission_id, position, file_id) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(i as i32)
            .bind(f.id)
            .execute(&mut *tx)
            .await?;
    }
    let title = steps[idx].title.clone();
    let staff = assistants::staff_ids(ctx, &l.class).await?;
    notifications::notify(
        &mut *tx,
        &staff,
        NotificationKind::HomeworkSubmitted,
        &format!("{} đã nộp bài \"{title}\"{}", actor_name(ctx, actor.id).await?, if late { " (muộn)" } else { "" }),
        &format!("Lớp học \"{}\"", l.class.title),
        Some(&format!("/day-hoc/bai-nop/{id}")),
    )
    .await?;
    tx.commit().await?;
    student_view(ctx, actor, class_id, step_id).await
}

async fn actor_name(ctx: &AppContext, id: Uuid) -> AppResult<String> {
    Ok(sqlx::query_scalar("SELECT full_name FROM users WHERE id = $1").bind(id).fetch_one(&ctx.db).await?)
}

// ---------- Giáo viên ----------

fn summarize(step: &Step, a: &AssignmentRef, assigned: i64, rows: &[&StatusRowLate]) -> HomeworkStepSummary {
    let count = |st: &str| rows.iter().filter(|r| r.status == st).count() as i64;
    HomeworkStepSummary {
        step_id: step.id,
        title: step_title(step, a),
        due_at: step.due_at,
        max_score: a.max_score,
        assigned,
        submitted: count("submitted"),
        graded: count("graded"),
        returned: count("returned"),
        late: rows.iter().filter(|r| r.late).count() as i64,
    }
}

#[derive(Debug, sqlx::FromRow)]
struct StatusRowLate {
    student_id: Uuid,
    step_id: Uuid,
    status: String,
    late: bool,
    score: Option<f64>,
    submitted_at: DateTime<Utc>,
    id: Uuid,
    file_count: i64,
}

async fn class_rows(ctx: &AppContext, class_id: Uuid) -> AppResult<Vec<StatusRowLate>> {
    Ok(sqlx::query_as::<_, StatusRowLate>(
        "SELECT s.student_id, s.step_id, s.status, s.late, s.score, s.submitted_at, s.id, \
         (SELECT count(*) FROM submission_files f WHERE f.submission_id = s.id) AS file_count \
         FROM homework_submissions s WHERE s.class_id = $1",
    )
    .bind(class_id)
    .fetch_all(&ctx.db)
    .await?)
}

/// Các bước bài tập của lớp học + số bài đã nộp / đã chấm.
pub async fn class_homework(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<Vec<HomeworkStepSummary>> {
    let class = load_staff_class(ctx, actor, class_id).await?;
    let steps: Vec<Step> = path::published_steps(ctx, class_id)
        .await?
        .steps
        .into_iter()
        .filter(|s| s.kind == StepKind::Homework)
        .collect();
    let refs = path::load_refs(ctx, &steps).await?;
    let rows = class_rows(ctx, class_id).await?;
    let mut out = Vec::new();
    for s in &steps {
        let Some(a) = s.assignment_id.and_then(|id| refs.assignments.get(&id)) else { continue };
        let assigned = path::assigned_students(ctx, &class, s).await?;
        let mine: Vec<&StatusRowLate> =
            rows.iter().filter(|r| r.step_id == s.id && assigned.contains(&r.student_id)).collect();
        out.push(summarize(s, a, assigned.len() as i64, &mine));
    }
    Ok(out)
}

/// Danh sách học sinh được giao một bước bài tập + bài nộp của từng người.
pub async fn step_submissions(
    ctx: &AppContext,
    actor: &Actor,
    class_id: Uuid,
    step_id: Uuid,
) -> AppResult<StepSubmissions> {
    let class = load_staff_class(ctx, actor, class_id).await?;
    let step = published_step(ctx, class_id, step_id).await?;
    let a = assignment(ctx, step.assignment_id.ok_or_else(not_found)?).await?;
    let assigned = path::assigned_students(ctx, &class, &step).await?;
    let rows = class_rows(ctx, class_id).await?;
    let by_student: HashMap<Uuid, &StatusRowLate> =
        rows.iter().filter(|r| r.step_id == step_id).map(|r| (r.student_id, r)).collect();
    let mut students: Vec<StudentSubmissionRow> = repo::members(&ctx.db, class_id, Some("active"))
        .await?
        .into_iter()
        .filter(|m| assigned.contains(&m.user_id))
        .map(|m| StudentSubmissionRow {
            submission: by_student.get(&m.user_id).map(|r| SubmissionBrief {
                id: r.id,
                status: r.status.clone(),
                submitted_at: r.submitted_at,
                late: r.late,
                score: r.score,
                file_count: r.file_count,
            }),
            user_id: m.user_id,
            username: m.username,
            full_name: m.full_name,
        })
        .collect();
    // Chưa chấm lên đầu (theo giờ nộp), rồi đã chấm, cuối cùng là chưa nộp.
    let rank = |s: &StudentSubmissionRow| match s.submission.as_ref().map(|x| x.status.as_str()) {
        Some("submitted") => 0,
        Some("returned") => 1,
        Some(_) => 2,
        None => 3,
    };
    students.sort_by(|x, y| {
        rank(x).cmp(&rank(y)).then_with(|| {
            x.submission.as_ref().map(|s| s.submitted_at).cmp(&y.submission.as_ref().map(|s| s.submitted_at))
        })
    });
    let mine: Vec<&StatusRowLate> = by_student.values().copied().filter(|r| assigned.contains(&r.student_id)).collect();
    Ok(StepSubmissions {
        step: summarize(&step, &a, assigned.len() as i64, &mine),
        instructions: a.instructions.clone(),
        students,
    })
}

/// Bài nộp + lớp học (kiểm tra giáo viên chủ lớp hoặc trợ giảng).
async fn own_submission(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<(SubmissionRow, ClassRow)> {
    let row = find_submission(ctx, "s.id = $1", id, None).await?.ok_or_else(submission_not_found)?;
    let class = load_staff_class(ctx, actor, row.class_id).await.map_err(|_| submission_not_found())?;
    Ok((row, class))
}

pub async fn submission_detail(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<SubmissionView> {
    let (row, _) = own_submission(ctx, actor, id).await?;
    let a = assignment(ctx, row.assignment_id).await?;
    view(ctx, row, a.max_score).await
}

/// Chấm bài (điểm + nhận xét + ảnh đã vẽ) hoặc trả lại để làm lại.
pub async fn grade(ctx: &AppContext, actor: &Actor, id: Uuid, input: GradeInput) -> AppResult<SubmissionView> {
    input.validate()?;
    let (row, class) = own_submission(ctx, actor, id).await?;
    let a = assignment(ctx, row.assignment_id).await?;
    let score = match (input.action, a.max_score, input.score) {
        (GradeAction::Return, _, _) => None,
        (_, None, Some(_)) => return Err(AppError::field("score", "Bài tập này chỉ nhận xét, không chấm điểm.")),
        (_, Some(max), Some(s)) if !(0.0..=max).contains(&s) => {
            return Err(AppError::field("score", format!("Điểm từ 0 đến {max}.")));
        }
        (_, Some(_), None) => return Err(AppError::field("score", "Hãy nhập điểm.")),
        (_, _, s) => s,
    };
    let annotated: Vec<Uuid> = input.annotations.iter().filter_map(|x| x.file_id).collect();
    files::owned_ready(ctx, actor.id, &annotated, Purpose::Homework).await?;

    let mut tx = ctx.db.begin().await?;
    for x in &input.annotations {
        sqlx::query("UPDATE submission_files SET annotated_file_id = $3 WHERE submission_id = $1 AND position = $2")
            .bind(id)
            .bind(x.position)
            .bind(x.file_id)
            .execute(&mut *tx)
            .await?;
    }
    let status = if input.action == GradeAction::Grade { "graded" } else { "returned" };
    sqlx::query(
        "UPDATE homework_submissions SET status = $2, score = $3, feedback = $4, graded_by = $5, \
         graded_at = CASE WHEN $2 = 'graded' THEN now() ELSE graded_at END, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(score)
    .bind(input.feedback.trim())
    .bind(actor.id)
    .execute(&mut *tx)
    .await?;
    let step = published_step(ctx, row.class_id, row.step_id).await.ok();
    let title = step.as_ref().map(|s| step_title(s, &a)).unwrap_or_else(|| a.title.clone());
    let (kind, heading) = match input.action {
        GradeAction::Grade => (
            NotificationKind::HomeworkGraded,
            match (score, a.max_score) {
                (Some(s), Some(max)) => format!("Bài \"{title}\" đã được chấm: {s}/{max}"),
                _ => format!("Bài \"{title}\" đã có nhận xét"),
            },
        ),
        GradeAction::Return => (NotificationKind::HomeworkReturned, format!("Bài \"{title}\" cần làm lại")),
    };
    notifications::notify(
        &mut *tx,
        &[row.student_id],
        kind,
        &heading,
        &format!("Lớp học \"{}\"", class.title),
        Some(&format!("/lop-hoc/{}/bai-tap/{}", row.class_id, row.step_id)),
    )
    .await?;
    tx.commit().await?;
    submission_detail(ctx, actor, id).await
}

// ---------- Nhắc hạn nộp ----------

#[derive(Debug, sqlx::FromRow)]
struct DueStep {
    class_id: Uuid,
    step: sqlx::types::Json<Step>,
    due_at: DateTime<Utc>,
}

/// Nhắc (trong app + email) học sinh chưa nộp các bài tập còn ≤ 24 giờ tới hạn. Mỗi (bước, hạn nộp) nhắc
/// một lần nhờ "nhận" bằng `INSERT … ON CONFLICT DO NOTHING RETURNING` — chạy nhiều instance không gửi trùng.
/// Trả về số học sinh được nhắc.
pub async fn send_due_reminders(ctx: &AppContext, now: DateTime<Utc>) -> AppResult<usize> {
    let due: Vec<DueStep> = sqlx::query_as(
        "SELECT pv.class_id, s.step, (s.step->>'dueAt')::timestamptz AS due_at \
         FROM path_versions pv \
         JOIN classes c ON c.id = pv.class_id AND c.archived_at IS NULL \
         CROSS JOIN LATERAL jsonb_array_elements(pv.steps) AS s(step) \
         WHERE pv.version = (SELECT max(version) FROM path_versions x WHERE x.class_id = pv.class_id) \
           AND s.step->>'kind' = 'homework' AND s.step->>'dueAt' IS NOT NULL \
           AND (s.step->>'dueAt')::timestamptz > $1 AND (s.step->>'dueAt')::timestamptz <= $2",
    )
    .bind(now)
    .bind(now + Duration::hours(REMIND_BEFORE_HOURS))
    .fetch_all(&ctx.db)
    .await?;
    let mut sent = 0;
    for d in due {
        let claimed: Option<Uuid> = sqlx::query_scalar(
            "INSERT INTO homework_due_reminders (class_id, step_id, due_at) VALUES ($1, $2, $3) \
             ON CONFLICT DO NOTHING RETURNING class_id",
        )
        .bind(d.class_id)
        .bind(d.step.id)
        .bind(d.due_at)
        .fetch_optional(&ctx.db)
        .await?;
        if claimed.is_none() {
            continue;
        }
        let Some(class) = repo::find(&ctx.db, d.class_id).await? else { continue };
        let Some(a) = d.step.assignment_id else { continue };
        let a = assignment(ctx, a).await?;
        let title = step_title(&d.step, &a);
        let assigned = path::assigned_students(ctx, &class, &d.step).await?;
        let done: Vec<Uuid> = statuses(ctx, d.class_id, None)
            .await?
            .into_iter()
            .filter(|r| r.step_id == d.step.id && path::homework_done(&r.status))
            .map(|r| r.student_id)
            .collect();
        let missing: Vec<Uuid> = assigned.into_iter().filter(|u| !done.contains(u)).collect();
        if missing.is_empty() {
            continue;
        }
        let link = format!("/lop-hoc/{}/bai-tap/{}", d.class_id, d.step.id);
        let when = crate::modules::sessions::time::format_vn(d.due_at);
        notifications::notify(
            &ctx.db,
            &missing,
            NotificationKind::HomeworkDue,
            &format!("Sắp hết hạn nộp \"{title}\""),
            &format!("Hạn nộp: {when} · Lớp học \"{}\"", class.title),
            Some(&link),
        )
        .await?;
        let people: Vec<(String, String)> =
            sqlx::query_as("SELECT email, full_name FROM users WHERE id = ANY($1) AND status = 'active'")
                .bind(&missing)
                .fetch_all(&ctx.db)
                .await?;
        for (email, name) in people {
            let text = format!(
                "Chào {name},\n\nBạn chưa nộp bài tập \"{title}\" của lớp học \"{}\".\nHạn nộp: {when}.\n\n\
                 Nộp bài tại: {}{link}\n\nSmart Learning Space",
                class.title, ctx.config.app_base_url
            );
            let email =
                Email {
                    to: email, subject: format!("Sắp hết hạn nộp bài: {title}"), text, attachments: Vec::new()
                };
            if let Err(e) = ctx.jobs.publish(Job::SendEmail(email)).await {
                tracing::error!(error = %e, "không gửi được email nhắc hạn nộp");
            }
        }
        sent += missing.len();
    }
    Ok(sent)
}

/// Vòng 60 giây của `api`: nhắc hạn nộp theo giờ hiện tại.
pub async fn reminder_tick(ctx: &AppContext) -> AppResult<usize> {
    send_due_reminders(ctx, Utc::now()).await
}
