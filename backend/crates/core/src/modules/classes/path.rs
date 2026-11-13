//! Lộ trình của lớp học (M9): soạn (bản nháp) → xuất bản; học sinh học theo lộ trình; tiến độ.
//!
//! Trạng thái từng bước của học sinh:
//! - Bài: xong khi học sinh bấm "Đã học xong" (`step_progress`).
//! - Đề: xong khi đã nộp ít nhất một lần và (nếu có `passScore`) điểm cao nhất ≥ `passScore`.
//! - Buổi học: xong khi học sinh đã được điểm danh (bấm "Vào học" hoặc giáo viên đánh dấu có mặt).
//! - Mốc: chỉ để chia giai đoạn, không tính vào tiến độ.
//! - Mở khóa `previous`: phải xong bước bài / đề ngay trước (bỏ qua mốc).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{
        ClassCard, ClassProgress, DraftStep, DueDeck, LearnStep, LearnView, PathInput, PathResponse, ProgressCell,
        ProgressStep, ProgressSummary, StartedAttempt, StepLesson, StepStatus, StudentProgress,
    },
    model::{ClassRow, Step, StepKind, Unlock},
    repository as repo,
    service::{load_own, not_found},
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        audit::{model::AuditEntry, service as audit},
        exams::{
            dto::AttemptSummary,
            model::{BestScoreRow, ExamRef},
            service::{self as exams, AttemptContext},
        },
        flashcards::{self, DeckRef, ReviewInput, ReviewResult, StudyDeck},
        lessons::{model::LessonRef, service as lessons},
        notifications::{self, NotificationKind},
        sessions::{model::SessionRef, service as sessions},
    },
    security::Actor,
};

/// Tính trạng thái các bước theo tập bước đã xong (hàm thuần).
pub fn compute_statuses(steps: &[Step], done: &HashSet<Uuid>) -> Vec<StepStatus> {
    let mut out = Vec::with_capacity(steps.len());
    let mut prev_done: Option<bool> = None; // bước bài / đề gần nhất phía trước đã xong chưa
    for s in steps {
        if s.kind == StepKind::Milestone {
            out.push(StepStatus::Available);
            continue;
        }
        let is_done = done.contains(&s.id);
        let locked = s.unlock == Unlock::Previous && prev_done == Some(false);
        out.push(if is_done {
            StepStatus::Done
        } else if locked {
            StepStatus::Locked
        } else {
            StepStatus::Available
        });
        // Bước bị khóa thì các bước "previous" sau nó cũng khóa (vì nó chưa xong).
        prev_done = Some(is_done);
    }
    out
}

fn display_title(s: &Step, refs: &Refs) -> String {
    if !s.title.trim().is_empty() {
        return s.title.clone();
    }
    s.lesson_id
        .and_then(|id| refs.lessons.get(&id).map(|l| l.title.clone()))
        .or_else(|| s.exam_id.and_then(|id| refs.exams.get(&id).map(|e| e.title.clone())))
        .or_else(|| s.session_id.and_then(|id| refs.sessions.get(&id).map(|x| x.title.clone())))
        .or_else(|| s.deck_id.and_then(|id| refs.decks.get(&id).map(|d| d.title.clone())))
        .unwrap_or_default()
}

struct Refs {
    lessons: HashMap<Uuid, LessonRef>,
    exams: HashMap<Uuid, ExamRef>,
    sessions: HashMap<Uuid, SessionRef>,
    decks: HashMap<Uuid, DeckRef>,
}

async fn load_refs(ctx: &AppContext, steps: &[Step]) -> AppResult<Refs> {
    let lesson_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.lesson_id).collect();
    let exam_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.exam_id).collect();
    let session_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.session_id).collect();
    let deck_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.deck_id).collect();
    Ok(Refs {
        lessons: lessons::refs(ctx, &lesson_ids).await?.into_iter().map(|l| (l.id, l)).collect(),
        exams: exams::refs(ctx, &exam_ids).await?.into_iter().map(|e| (e.id, e)).collect(),
        sessions: sessions::refs(ctx, &session_ids).await?.into_iter().map(|x| (x.id, x)).collect(),
        decks: flashcards::refs(ctx, &deck_ids).await?.into_iter().map(|d| (d.id, d)).collect(),
    })
}

fn deck_status(d: &DeckRef) -> &'static str {
    if d.archived {
        "archived"
    } else if d.card_count == 0 {
        "empty"
    } else {
        "ready"
    }
}

// ---------- Soạn lộ trình (giáo viên) ----------

async fn path_response(ctx: &AppContext, class: &ClassRow) -> AppResult<PathResponse> {
    let steps = repo::draft_steps(&ctx.db, class.id).await?;
    let refs = load_refs(ctx, &steps).await?;
    let out = steps
        .iter()
        .map(|s| {
            let lesson = s.lesson_id.and_then(|id| refs.lessons.get(&id));
            let exam = s.exam_id.and_then(|id| refs.exams.get(&id));
            let session = s.session_id.and_then(|id| refs.sessions.get(&id));
            let deck = s.deck_id.and_then(|id| refs.decks.get(&id));
            let ref_status = match (lesson, exam) {
                (Some(l), _) if l.archived => Some("archived"),
                (Some(l), _) if l.published_revision_id.is_some() => Some("published"),
                (Some(_), _) => Some("draft"),
                (_, Some(e)) if e.archived => Some("archived"),
                (_, Some(e)) if e.item_count == 0 => Some("empty"),
                (_, Some(_)) => Some("ready"),
                _ => session.map(|x| x.status.as_str()).or_else(|| deck.map(deck_status)),
            };
            DraftStep {
                id: s.id,
                kind: s.kind,
                title: s.title.clone(),
                lesson_id: s.lesson_id,
                exam_id: s.exam_id,
                session_id: s.session_id,
                deck_id: s.deck_id,
                starts_at: session.map(|x| x.starts_at),
                unlock: s.unlock,
                pass_score: s.pass_score,
                ref_title: lesson
                    .map(|l| l.title.clone())
                    .or_else(|| exam.map(|e| e.title.clone()))
                    .or_else(|| session.map(|x| x.title.clone()))
                    .or_else(|| deck.map(|d| d.title.clone())),
                ref_status: ref_status.map(str::to_string),
                duration_minutes: lesson.and_then(|l| l.duration_minutes),
                exam_kind: exam.map(|e| e.kind.clone()),
                item_count: exam.map(|e| e.item_count).or_else(|| deck.map(|d| d.card_count)),
            }
        })
        .collect();
    Ok(PathResponse {
        steps: out,
        published_version: class.path_version,
        published_at: class.path_published_at,
        has_changes: super::dto::ClassResponse::from(class.clone()).path_has_changes,
    })
}

pub async fn get_path(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<PathResponse> {
    let class = load_own(ctx, actor, class_id).await?;
    path_response(ctx, &class).await
}

/// Lưu cả bản nháp lộ trình (thứ tự theo mảng gửi lên). Bước giữ ID cũ thì giữ tiến độ.
pub async fn save_path(ctx: &AppContext, actor: &Actor, class_id: Uuid, input: PathInput) -> AppResult<PathResponse> {
    input.validate()?;
    load_own(ctx, actor, class_id).await?;
    let existing: HashSet<Uuid> = repo::draft_steps(&ctx.db, class_id).await?.into_iter().map(|s| s.id).collect();
    let lesson_ids: Vec<Uuid> = input.steps.iter().filter_map(|s| s.lesson_id).collect();
    let exam_ids: Vec<Uuid> = input.steps.iter().filter_map(|s| s.exam_id).collect();
    let lesson_refs: HashMap<Uuid, LessonRef> =
        lessons::refs(ctx, &lesson_ids).await?.into_iter().map(|l| (l.id, l)).collect();
    let exam_refs: HashMap<Uuid, ExamRef> = exams::refs(ctx, &exam_ids).await?.into_iter().map(|e| (e.id, e)).collect();
    let session_ids: Vec<Uuid> = input.steps.iter().filter_map(|s| s.session_id).collect();
    let session_refs: HashMap<Uuid, SessionRef> =
        sessions::refs(ctx, &session_ids).await?.into_iter().map(|x| (x.id, x)).collect();
    let deck_ids: Vec<Uuid> = input.steps.iter().filter_map(|s| s.deck_id).collect();
    let deck_refs: HashMap<Uuid, DeckRef> =
        flashcards::refs(ctx, &deck_ids).await?.into_iter().map(|d| (d.id, d)).collect();

    let mut used = HashSet::new();
    let mut steps = Vec::with_capacity(input.steps.len());
    for (i, s) in input.steps.into_iter().enumerate() {
        let n = i + 1;
        let mut deck_id = None;
        let (lesson_id, exam_id, session_id) = match s.kind {
            StepKind::Milestone => {
                if s.title.trim().is_empty() {
                    return Err(AppError::field("steps", format!("Bước {n}: mốc cần có tên.")));
                }
                (None, None, None)
            }
            StepKind::Deck => {
                let id = s.deck_id.ok_or_else(|| AppError::field("steps", format!("Bước {n}: chưa chọn bộ thẻ.")))?;
                match deck_refs.get(&id) {
                    Some(d) if d.owner_id == actor.id && !d.archived => {}
                    _ => {
                        return Err(AppError::field("steps", format!("Bước {n}: bộ thẻ không tồn tại hoặc đã bị ẩn.")));
                    }
                }
                deck_id = Some(id);
                (None, None, None)
            }
            StepKind::Lesson => {
                let id = s.lesson_id.ok_or_else(|| AppError::field("steps", format!("Bước {n}: chưa chọn bài.")))?;
                match lesson_refs.get(&id) {
                    Some(l) if l.owner_id == actor.id && !l.archived => {}
                    _ => return Err(AppError::field("steps", format!("Bước {n}: bài không tồn tại hoặc đã bị ẩn."))),
                }
                (Some(id), None, None)
            }
            StepKind::Exam => {
                let id = s.exam_id.ok_or_else(|| AppError::field("steps", format!("Bước {n}: chưa chọn đề.")))?;
                match exam_refs.get(&id) {
                    Some(e) if e.owner_id == actor.id && !e.archived => {}
                    _ => return Err(AppError::field("steps", format!("Bước {n}: đề không tồn tại hoặc đã bị ẩn."))),
                }
                (None, Some(id), None)
            }
            StepKind::Session => {
                let id =
                    s.session_id.ok_or_else(|| AppError::field("steps", format!("Bước {n}: chưa chọn buổi học.")))?;
                match session_refs.get(&id) {
                    Some(x) if x.owner_id == actor.id && x.class_id == Some(class_id) => {}
                    _ => {
                        return Err(AppError::field(
                            "steps",
                            format!("Bước {n}: buổi học không tồn tại hoặc không thuộc lớp học này."),
                        ));
                    }
                }
                (None, None, Some(id))
            }
        };
        let id = match s.id {
            Some(id) if existing.contains(&id) && !used.contains(&id) => id,
            _ => Uuid::now_v7(),
        };
        used.insert(id);
        steps.push(Step {
            id,
            kind: s.kind,
            title: s.title.trim().to_string(),
            lesson_id,
            exam_id,
            session_id,
            deck_id,
            unlock: if s.kind == StepKind::Milestone { Unlock::Free } else { s.unlock },
            pass_score: if s.kind == StepKind::Exam { s.pass_score } else { None },
        });
    }
    let mut tx = ctx.db.begin().await?;
    repo::replace_steps(&mut tx, class_id, &steps).await?;
    tx.commit().await?;
    get_path(ctx, actor, class_id).await
}

/// Xuất bản bản nháp: mọi bài phải đã xuất bản, mọi đề phải có câu hỏi.
pub async fn publish_path(
    ctx: &AppContext,
    actor: &Actor,
    class_id: Uuid,
    ip: Option<&str>,
) -> AppResult<PathResponse> {
    load_own(ctx, actor, class_id).await?;
    let steps = repo::draft_steps(&ctx.db, class_id).await?;
    if !steps.iter().any(|s| s.kind != StepKind::Milestone) {
        return Err(AppError::conflict("PATH_EMPTY", "Lộ trình chưa có bài học hoặc đề nào."));
    }
    let refs = load_refs(ctx, &steps).await?;
    for s in &steps {
        if let Some(l) = s.lesson_id.and_then(|id| refs.lessons.get(&id))
            && l.published_revision_id.is_none()
        {
            return Err(AppError::conflict(
                "LESSON_NOT_PUBLISHED",
                format!("Bài \"{}\" chưa xuất bản. Hãy xuất bản bài trước.", l.title),
            ));
        }
        if let Some(e) = s.exam_id.and_then(|id| refs.exams.get(&id))
            && e.item_count == 0
        {
            return Err(AppError::conflict("EXAM_EMPTY", format!("Đề \"{}\" chưa có câu hỏi.", e.title)));
        }
        if let Some(d) = s.deck_id.and_then(|id| refs.decks.get(&id)) {
            if d.archived {
                return Err(AppError::conflict("DECK_ARCHIVED", format!("Bộ thẻ \"{}\" đã bị ẩn.", d.title)));
            }
            if d.card_count == 0 {
                return Err(AppError::conflict("DECK_EMPTY", format!("Bộ thẻ \"{}\" chưa có thẻ nào.", d.title)));
            }
        }
    }
    let mut tx = ctx.db.begin().await?;
    let version = repo::insert_version(&mut *tx, class_id, &steps, actor.id).await?;
    audit::record(
        &mut *tx,
        AuditEntry {
            actor_id: Some(actor.id),
            action: "class.path_publish",
            target_type: "class",
            target_id: Some(class_id),
            data: json!({ "version": version, "steps": steps.len() }),
            ip,
        },
    )
    .await?;
    tx.commit().await?;
    // Báo học sinh đang học: lộ trình có bản mới.
    let class = repo::find(&ctx.db, class_id).await?.ok_or_else(not_found)?;
    let students: Vec<Uuid> =
        repo::members(&ctx.db, class_id, Some("active")).await?.into_iter().map(|m| m.user_id).collect();
    notifications::notify(
        &ctx.db,
        &students,
        NotificationKind::PathPublished,
        &format!("Lộ trình lớp học \"{}\" có bản mới", class.title),
        "Giáo viên vừa cập nhật nội dung học.",
        Some(&format!("/lop-hoc/{class_id}")),
    )
    .await?;
    get_path(ctx, actor, class_id).await
}

// ---------- Học theo lộ trình (học sinh) ----------

struct Learner {
    class: ClassRow,
    /// Giáo viên chủ lớp xem như học sinh.
    preview: bool,
    status: Option<String>,
}

async fn learner(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<Learner> {
    let class = repo::find(&ctx.db, class_id).await?.ok_or_else(not_found)?;
    if class.owner_id == actor.id {
        return Ok(Learner { class, preview: true, status: None });
    }
    let member = repo::membership(&ctx.db, class_id, actor.id).await?;
    let status = member.map(|m| m.status);
    if class.archived_at.is_some() || status.as_deref() != Some("active") {
        if class.visibility == "private" && status.is_none() {
            return Err(not_found());
        }
        return Err(AppError::forbidden(
            "NOT_ENROLLED",
            if status.as_deref() == Some("pending") {
                "Yêu cầu ghi danh của bạn đang chờ giáo viên duyệt."
            } else {
                "Bạn chưa ghi danh vào lớp học này."
            },
        ));
    }
    Ok(Learner { class, preview: false, status })
}

struct Snapshot {
    version: Option<i32>,
    steps: Vec<Step>,
}

async fn published_steps(ctx: &AppContext, class_id: Uuid) -> AppResult<Snapshot> {
    Ok(match repo::latest_version(&ctx.db, class_id).await? {
        Some(v) => Snapshot { version: Some(v.version), steps: v.steps.0 },
        None => Snapshot { version: None, steps: Vec::new() },
    })
}

/// Tập bước đã xong của một học sinh + điểm đề + ngày hoàn thành bài + số thẻ cần ôn của từng bộ thẻ.
struct Done {
    set: HashSet<Uuid>,
    scores: HashMap<Uuid, BestScoreRow>,
    completed: HashMap<Uuid, DateTime<Utc>>,
    due_cards: HashMap<Uuid, i64>,
}

async fn done_for(ctx: &AppContext, class_id: Uuid, user_id: Uuid, steps: &[Step]) -> AppResult<Done> {
    let completed: HashMap<Uuid, DateTime<Utc>> = repo::progress(&ctx.db, class_id, Some(user_id))
        .await?
        .into_iter()
        .map(|p| (p.step_id, p.completed_at))
        .collect();
    let exam_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.exam_id).collect();
    let scores: HashMap<Uuid, BestScoreRow> = exams::best_scores(ctx, class_id, &exam_ids, Some(user_id))
        .await?
        .into_iter()
        .map(|b| (b.exam_id, b))
        .collect();
    let session_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.session_id).collect();
    let attended: HashSet<Uuid> =
        sessions::attended_pairs(ctx, &session_ids, Some(user_id)).await?.into_iter().map(|(_, s)| s).collect();
    let deck_ids: Vec<Uuid> = steps.iter().filter_map(|s| s.deck_id).collect();
    let decks_done: HashSet<Uuid> =
        flashcards::completed_pairs(ctx, &deck_ids, Some(user_id)).await?.into_iter().map(|(_, d)| d).collect();
    let due_cards: HashMap<Uuid, i64> = flashcards::due_counts(ctx, user_id, &deck_ids)
        .await?
        .into_iter()
        .map(|(d, (due, fresh))| (d, due + fresh))
        .collect();
    let set = steps
        .iter()
        .filter(|s| match s.kind {
            StepKind::Lesson => completed.contains_key(&s.id),
            StepKind::Exam => s.exam_id.and_then(|e| scores.get(&e)).is_some_and(|b| exam_passed(s, b)),
            StepKind::Session => s.session_id.is_some_and(|x| attended.contains(&x)),
            StepKind::Deck => s.deck_id.is_some_and(|d| decks_done.contains(&d)),
            StepKind::Milestone => false,
        })
        .map(|s| s.id)
        .collect();
    Ok(Done { set, scores, completed, due_cards })
}

fn exam_passed(step: &Step, best: &BestScoreRow) -> bool {
    best.submitted > 0 && step.pass_score.is_none_or(|p| best.best_score.unwrap_or(0.0) + 1e-9 >= p)
}

fn learn_steps(steps: &[Step], refs: &Refs, done: &Done, preview: bool) -> Vec<LearnStep> {
    let statuses = compute_statuses(steps, &done.set);
    steps
        .iter()
        .zip(statuses)
        .map(|(s, status)| {
            let lesson = s.lesson_id.and_then(|id| refs.lessons.get(&id));
            let exam = s.exam_id.and_then(|id| refs.exams.get(&id));
            let session = s.session_id.and_then(|id| refs.sessions.get(&id));
            let deck = s.deck_id.and_then(|id| refs.decks.get(&id));
            let best = s.exam_id.and_then(|id| done.scores.get(&id));
            LearnStep {
                id: s.id,
                kind: s.kind,
                title: display_title(s, refs),
                status: if preview && status == StepStatus::Locked { StepStatus::Available } else { status },
                lesson_id: s.lesson_id,
                exam_id: s.exam_id,
                session_id: s.session_id,
                deck_id: s.deck_id,
                due_cards: s.deck_id.map(|d| done.due_cards.get(&d).copied().unwrap_or(0)),
                starts_at: session.map(|x| x.starts_at),
                ends_at: session.map(|x| x.ends_at),
                session_status: session.map(|x| x.status.clone()),
                exam_kind: exam.map(|e| e.kind.clone()),
                duration_minutes: lesson.and_then(|l| l.duration_minutes),
                time_limit_minutes: exam.and_then(|e| e.time_limit_minutes),
                item_count: exam.map(|e| e.item_count).or_else(|| deck.map(|d| d.card_count)),
                pass_score: s.pass_score,
                best_score: best.and_then(|b| b.best_score),
                max_score: best.and_then(|b| b.max_score),
                submitted_count: best.map_or(0, |b| b.submitted),
                in_progress_attempt_id: best.and_then(|b| b.in_progress),
                completed_at: done.completed.get(&s.id).copied(),
            }
        })
        .collect()
}

fn summary(steps: &[LearnStep]) -> ProgressSummary {
    let total = steps.iter().filter(|s| s.kind != StepKind::Milestone).count() as i64;
    let done = steps.iter().filter(|s| s.status == StepStatus::Done).count() as i64;
    ProgressSummary::new(done, total)
}

pub async fn learn(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<LearnView> {
    let l = learner(ctx, actor, class_id).await?;
    let snap = published_steps(ctx, class_id).await?;
    let refs = load_refs(ctx, &snap.steps).await?;
    let done = done_for(ctx, class_id, actor.id, &snap.steps).await?;
    let steps = learn_steps(&snap.steps, &refs, &done, l.preview);
    Ok(LearnView {
        progress: summary(&steps),
        class: ClassCard::new(l.class, l.status, Some(actor.id)),
        steps,
        path_version: snap.version,
        is_preview: l.preview,
    })
}

/// % tiến độ của một học sinh (dùng cho "Lớp học của tôi").
pub async fn progress_summary(ctx: &AppContext, class_id: Uuid, user_id: Uuid) -> AppResult<ProgressSummary> {
    let snap = published_steps(ctx, class_id).await?;
    let done = done_for(ctx, class_id, user_id, &snap.steps).await?;
    let total = snap.steps.iter().filter(|s| s.kind != StepKind::Milestone).count() as i64;
    Ok(ProgressSummary::new(done.set.len() as i64, total))
}

/// Bước + trạng thái, chặn bước đang khóa.
async fn open_step(
    ctx: &AppContext,
    actor: &Actor,
    class_id: Uuid,
    step_id: Uuid,
) -> AppResult<(Learner, Vec<LearnStep>, usize)> {
    let l = learner(ctx, actor, class_id).await?;
    let snap = published_steps(ctx, class_id).await?;
    let refs = load_refs(ctx, &snap.steps).await?;
    let done = done_for(ctx, class_id, actor.id, &snap.steps).await?;
    let steps = learn_steps(&snap.steps, &refs, &done, l.preview);
    let idx = steps
        .iter()
        .position(|s| s.id == step_id)
        .ok_or_else(|| AppError::not_found("Không có bước này trong lộ trình."))?;
    if steps[idx].status == StepStatus::Locked {
        return Err(AppError::forbidden("STEP_LOCKED", "Hãy hoàn thành bước trước để mở bước này."));
    }
    Ok((l, steps, idx))
}

pub async fn step_lesson(ctx: &AppContext, actor: &Actor, class_id: Uuid, step_id: Uuid) -> AppResult<StepLesson> {
    let (l, mut steps, idx) = open_step(ctx, actor, class_id, step_id).await?;
    let lesson_id = steps[idx].lesson_id.ok_or_else(|| AppError::not_found("Bước này không phải bài học."))?;
    let lesson = lessons::published(ctx, lesson_id).await?.ok_or_else(|| AppError::not_found("Bài chưa xuất bản."))?;
    let real = |s: &&LearnStep| s.kind != StepKind::Milestone;
    let prev_step_id = steps[..idx].iter().rev().find(real).map(|s| s.id);
    let next_step_id = steps[idx + 1..].iter().find(real).map(|s| s.id);
    Ok(StepLesson { step: steps.swap_remove(idx), lesson, prev_step_id, next_step_id, is_preview: l.preview })
}

pub async fn complete_step(ctx: &AppContext, actor: &Actor, class_id: Uuid, step_id: Uuid) -> AppResult<LearnView> {
    let (l, steps, idx) = open_step(ctx, actor, class_id, step_id).await?;
    if steps[idx].kind != StepKind::Lesson {
        return Err(AppError::bad_request("NOT_LESSON", "Chỉ bước bài học mới đánh dấu đã học xong."));
    }
    if !l.preview {
        repo::mark_done(&ctx.db, actor.id, class_id, step_id).await?;
    }
    learn(ctx, actor, class_id).await
}

/// Bắt đầu (hoặc làm tiếp) đề của một bước. Giáo viên chủ lớp → lượt làm thử.
pub async fn start_exam(ctx: &AppContext, actor: &Actor, class_id: Uuid, step_id: Uuid) -> AppResult<StartedAttempt> {
    let (l, steps, idx) = open_step(ctx, actor, class_id, step_id).await?;
    let exam_id = steps[idx].exam_id.ok_or_else(|| AppError::not_found("Bước này không phải đề."))?;
    let c = if l.preview {
        AttemptContext { class_id: None, step_id: None, preview: true }
    } else {
        AttemptContext { class_id: Some(class_id), step_id: Some(step_id), preview: false }
    };
    Ok(StartedAttempt { attempt_id: exams::start_attempt(ctx, actor.id, exam_id, c).await? })
}

pub async fn step_attempts(
    ctx: &AppContext,
    actor: &Actor,
    class_id: Uuid,
    step_id: Uuid,
) -> AppResult<Vec<AttemptSummary>> {
    let (_, steps, idx) = open_step(ctx, actor, class_id, step_id).await?;
    let exam_id = steps[idx].exam_id.ok_or_else(|| AppError::not_found("Bước này không phải đề."))?;
    exams::my_attempts(ctx, actor.id, exam_id, class_id).await
}

/// Các bước đề trong lộ trình đã xuất bản: (id bước, id đề, tên hiển thị).
pub async fn published_exam_steps(ctx: &AppContext, class_id: Uuid) -> AppResult<Vec<(Uuid, Uuid, String)>> {
    let snap = published_steps(ctx, class_id).await?;
    let refs = load_refs(ctx, &snap.steps).await?;
    Ok(snap
        .steps
        .iter()
        .filter(|s| s.kind == StepKind::Exam)
        .filter_map(|s| Some((s.id, s.exam_id?, display_title(s, &refs))))
        .collect())
}

/// Tên hiển thị của các bước (không tính mốc) trong lộ trình đã xuất bản — dùng cho hỏi đáp.
pub async fn published_step_titles(ctx: &AppContext, class_id: Uuid) -> AppResult<HashMap<Uuid, String>> {
    let snap = published_steps(ctx, class_id).await?;
    let refs = load_refs(ctx, &snap.steps).await?;
    Ok(snap.steps.iter().filter(|s| s.kind != StepKind::Milestone).map(|s| (s.id, display_title(s, &refs))).collect())
}

// ---------- Bộ thẻ ghi nhớ ----------

/// Bộ thẻ của một bước + lịch ôn của người học (giáo viên xem thử: luôn như thẻ mới).
pub async fn step_deck(ctx: &AppContext, actor: &Actor, class_id: Uuid, step_id: Uuid) -> AppResult<StudyDeck> {
    let (l, steps, idx) = open_step(ctx, actor, class_id, step_id).await?;
    let deck_id = steps[idx].deck_id.ok_or_else(|| AppError::not_found("Bước này không phải bộ thẻ."))?;
    let mut deck = flashcards::study(ctx, actor.id, deck_id).await?;
    deck.title = steps[idx].title.clone();
    if l.preview {
        deck.due_count = 0;
        deck.new_count = deck.cards.len();
    }
    Ok(deck)
}

/// Ghi một lần ôn thẻ của bước bộ thẻ.
pub async fn review_card(
    ctx: &AppContext,
    actor: &Actor,
    class_id: Uuid,
    step_id: Uuid,
    input: ReviewInput,
) -> AppResult<ReviewResult> {
    let (l, steps, idx) = open_step(ctx, actor, class_id, step_id).await?;
    let deck_id = steps[idx].deck_id.ok_or_else(|| AppError::not_found("Bước này không phải bộ thẻ."))?;
    flashcards::review(ctx, actor.id, deck_id, input, !l.preview).await
}

/// Bộ thẻ cần ôn của học sinh trong mọi lớp học đang học (bỏ bước đang khóa).
pub async fn my_reviews(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<DueDeck>> {
    let classes = repo::my_active_class_ids(&ctx.db, actor.id).await?;
    let mut out = Vec::new();
    for (class_id, class_title) in classes {
        let snap = published_steps(ctx, class_id).await?;
        if !snap.steps.iter().any(|s| s.kind == StepKind::Deck) {
            continue;
        }
        let refs = load_refs(ctx, &snap.steps).await?;
        let done = done_for(ctx, class_id, actor.id, &snap.steps).await?;
        let deck_ids: Vec<Uuid> = snap.steps.iter().filter_map(|s| s.deck_id).collect();
        let counts = flashcards::due_counts(ctx, actor.id, &deck_ids).await?;
        for s in learn_steps(&snap.steps, &refs, &done, false) {
            let (Some(deck_id), true) = (s.deck_id, s.status != StepStatus::Locked) else { continue };
            let (due, fresh) = counts.get(&deck_id).copied().unwrap_or((0, 0));
            out.push(DueDeck {
                class_id,
                class_title: class_title.clone(),
                step_id: s.id,
                deck_id,
                title: s.title,
                due,
                fresh,
                total: s.item_count.unwrap_or(0),
            });
        }
    }
    // Cùng một bộ thẻ ở nhiều lớp học: chỉ giữ một dòng (lịch ôn dùng chung).
    let mut seen = HashSet::new();
    out.retain(|d| seen.insert(d.deck_id));
    out.sort_by_key(|d| std::cmp::Reverse(d.due + d.fresh));
    Ok(out)
}

// ---------- Tiến độ cả lớp (giáo viên) ----------

pub async fn class_progress(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<ClassProgress> {
    load_own(ctx, actor, class_id).await?;
    let snap = published_steps(ctx, class_id).await?;
    let refs = load_refs(ctx, &snap.steps).await?;
    let students = repo::members(&ctx.db, class_id, Some("active")).await?;
    let progress = repo::progress(&ctx.db, class_id, None).await?;
    let exam_ids: Vec<Uuid> = snap.steps.iter().filter_map(|s| s.exam_id).collect();
    let scores = exams::best_scores(ctx, class_id, &exam_ids, None).await?;
    let session_ids: Vec<Uuid> = snap.steps.iter().filter_map(|s| s.session_id).collect();
    let attended = sessions::attended_pairs(ctx, &session_ids, None).await?;
    let deck_ids: Vec<Uuid> = snap.steps.iter().filter_map(|s| s.deck_id).collect();
    let decks_done: HashSet<(Uuid, Uuid)> =
        flashcards::completed_pairs(ctx, &deck_ids, None).await?.into_iter().collect();

    let real_steps: Vec<&Step> = snap.steps.iter().filter(|s| s.kind != StepKind::Milestone).collect();
    let out_students = students
        .into_iter()
        .map(|m| {
            let mut done = 0;
            let cells: Vec<ProgressCell> = real_steps
                .iter()
                .map(|s| {
                    let best = s.exam_id.and_then(|e| scores.iter().find(|b| b.user_id == m.user_id && b.exam_id == e));
                    let is_done = match s.kind {
                        StepKind::Lesson => progress.iter().any(|p| p.user_id == m.user_id && p.step_id == s.id),
                        StepKind::Session => {
                            s.session_id.is_some_and(|x| attended.iter().any(|(u, sid)| *u == m.user_id && *sid == x))
                        }
                        StepKind::Deck => s.deck_id.is_some_and(|d| decks_done.contains(&(m.user_id, d))),
                        _ => best.is_some_and(|b| exam_passed(s, b)),
                    };
                    if is_done {
                        done += 1;
                    }
                    ProgressCell {
                        step_id: s.id,
                        done: is_done,
                        score: best.and_then(|b| b.best_score),
                        max_score: best.and_then(|b| b.max_score),
                    }
                })
                .collect();
            StudentProgress {
                user_id: m.user_id,
                username: m.username,
                full_name: m.full_name,
                progress: ProgressSummary::new(done, real_steps.len() as i64),
                cells,
            }
        })
        .collect();
    Ok(ClassProgress {
        steps: real_steps
            .iter()
            .map(|s| ProgressStep { id: s.id, kind: s.kind, title: display_title(s, &refs) })
            .collect(),
        students: out_students,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(kind: StepKind, unlock: Unlock) -> Step {
        Step {
            id: Uuid::now_v7(),
            kind,
            title: String::new(),
            lesson_id: None,
            exam_id: None,
            session_id: None,
            deck_id: None,
            unlock,
            pass_score: None,
        }
    }

    #[test]
    fn mo_khoa_theo_buoc_truoc_bo_qua_moc() {
        let steps = vec![
            step(StepKind::Milestone, Unlock::Free),
            step(StepKind::Lesson, Unlock::Free),
            step(StepKind::Milestone, Unlock::Free),
            step(StepKind::Lesson, Unlock::Previous),
            step(StepKind::Exam, Unlock::Previous),
            step(StepKind::Lesson, Unlock::Free),
        ];
        let none = HashSet::new();
        use StepStatus::*;
        assert_eq!(compute_statuses(&steps, &none), vec![Available, Available, Available, Locked, Locked, Available]);

        let done: HashSet<Uuid> = [steps[1].id].into();
        assert_eq!(compute_statuses(&steps, &done), vec![Available, Done, Available, Available, Locked, Available]);

        let done: HashSet<Uuid> = [steps[1].id, steps[3].id].into();
        assert_eq!(compute_statuses(&steps, &done), vec![Available, Done, Available, Done, Available, Available]);
    }

    #[test]
    fn buoc_dau_tien_luon_mo() {
        let steps = vec![step(StepKind::Lesson, Unlock::Previous)];
        assert_eq!(compute_statuses(&steps, &HashSet::new()), vec![StepStatus::Available]);
    }
}
