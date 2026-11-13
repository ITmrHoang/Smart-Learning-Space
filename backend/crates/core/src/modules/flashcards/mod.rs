//! Thẻ ghi nhớ (M19): giáo viên soạn bộ thẻ (mặt trước / mặt sau là Markdown + công thức) hoặc tạo từ câu hỏi;
//! bộ thẻ là một bước của lộ trình (`path.rs`). Học sinh ôn theo lịch lặp lại ngắt quãng (`srs.rs`), lịch ôn
//! gắn với từng (học sinh, thẻ) nên một bộ thẻ dùng ở nhiều lớp học vẫn chung một lịch ôn.

pub mod srs;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgExecutor;
use uuid::Uuid;
use validator::Validate;

use self::srs::{CardState, Rating};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{questions::model::QuestionBody, teachers::service as teachers},
    security::{Actor, Permission},
};

/// Tối đa số thẻ trong một bộ.
pub const MAX_CARDS: usize = 500;

// ---------- DTO ----------

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CardInput {
    /// ID thẻ cũ (giữ lịch ôn của học sinh); bỏ trống = thẻ mới.
    pub id: Option<Uuid>,
    #[validate(length(min = 1, max = 5000, message = "Mặt trước từ 1 đến 5.000 ký tự."))]
    pub front: String,
    #[validate(length(min = 1, max = 5000, message = "Mặt sau từ 1 đến 5.000 ký tự."))]
    pub back: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeckInput {
    pub subject_id: Uuid,
    #[validate(length(min = 1, max = 200, message = "Tên bộ thẻ từ 1 đến 200 ký tự."))]
    pub title: String,
    #[serde(default)]
    #[validate(length(max = 2000, message = "Mô tả tối đa 2.000 ký tự."))]
    pub description: String,
    #[serde(default)]
    #[validate(length(max = 500, message = "Tối đa 500 thẻ."), nested)]
    pub cards: Vec<CardInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FromQuestionsInput {
    pub question_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeckQuery {
    pub q: Option<String>,
    pub subject_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct DeckSummary {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub title: String,
    pub description: String,
    pub card_count: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: Uuid,
    pub front: String,
    pub back: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckDetail {
    #[serde(flatten)]
    pub deck: DeckSummary,
    pub cards: Vec<Card>,
}

/// Bộ thẻ được trỏ tới từ lộ trình.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeckRef {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub archived: bool,
    pub card_count: i64,
}

/// Thẻ kèm lịch ôn của người học.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyCard {
    pub id: Uuid,
    pub front: String,
    pub back: String,
    /// new | learning | review
    pub state: &'static str,
    pub due_at: Option<DateTime<Utc>>,
    pub interval_days: i32,
    pub review_count: i32,
    /// Thời gian tới lần ôn sau nếu bấm Quên / Khó / Được / Dễ (phút).
    pub next_minutes: [i64; 4],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyDeck {
    pub deck_id: Uuid,
    pub title: String,
    pub description: String,
    pub cards: Vec<StudyCard>,
    /// Thẻ đến hạn ôn (đã học, `dueAt` ≤ bây giờ).
    pub due_count: usize,
    pub new_count: usize,
    pub next_due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInput {
    pub card_id: Uuid,
    pub rating: Rating,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResult {
    pub card_id: Uuid,
    pub due_at: DateTime<Utc>,
    pub interval_days: i32,
    /// Giáo viên xem thử: không lưu.
    pub saved: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ReviewRow {
    card_id: Uuid,
    repetitions: i32,
    interval_days: i32,
    ease: f64,
    lapses: i32,
    review_count: i32,
    due_at: DateTime<Utc>,
}

impl ReviewRow {
    fn state(&self) -> CardState {
        CardState {
            repetitions: self.repetitions,
            interval_days: self.interval_days,
            ease: self.ease,
            lapses: self.lapses,
        }
    }
}

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy bộ thẻ.")
}

// ---------- Truy vấn ----------

const SUMMARY_COLS: &str = "d.id, d.subject_id, d.title, d.description, d.updated_at, \
     (SELECT count(*) FROM cards c WHERE c.deck_id = d.id) AS card_count";

async fn find_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<DeckSummary> {
    actor.require(Permission::TeachingManage)?;
    sqlx::query_as::<_, DeckSummary>(crate::sql!(
        "SELECT {SUMMARY_COLS} FROM decks d WHERE d.id = $1 AND d.owner_id = $2 AND d.archived_at IS NULL"
    ))
    .bind(id)
    .bind(actor.id)
    .fetch_optional(&ctx.db)
    .await?
    .ok_or_else(not_found)
}

async fn cards_of<'e, E: PgExecutor<'e>>(db: E, deck_id: Uuid) -> sqlx::Result<Vec<Card>> {
    sqlx::query_as::<_, Card>("SELECT id, front, back FROM cards WHERE deck_id = $1 ORDER BY position, id")
        .bind(deck_id)
        .fetch_all(db)
        .await
}

/// Ghi lại toàn bộ thẻ theo thứ tự; thẻ giữ ID cũ (của chính bộ này) thì giữ lịch ôn.
async fn replace_cards(conn: &mut sqlx::PgConnection, deck_id: Uuid, cards: &[CardInput]) -> sqlx::Result<()> {
    let existing: HashSet<Uuid> = sqlx::query_scalar::<_, Uuid>("SELECT id FROM cards WHERE deck_id = $1")
        .bind(deck_id)
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .collect();
    let mut kept = Vec::new();
    for (i, c) in cards.iter().enumerate() {
        match c.id.filter(|id| existing.contains(id) && !kept.contains(id)) {
            Some(id) => {
                sqlx::query(
                    "UPDATE cards SET position = $2, front = $3, back = $4, \
                     updated_at = CASE WHEN front <> $3 OR back <> $4 THEN now() ELSE updated_at END WHERE id = $1",
                )
                .bind(id)
                .bind(i as i32)
                .bind(c.front.trim())
                .bind(c.back.trim())
                .execute(&mut *conn)
                .await?;
                kept.push(id);
            }
            None => {
                let id: Uuid = sqlx::query_scalar(
                    "INSERT INTO cards (deck_id, position, front, back) VALUES ($1, $2, $3, $4) RETURNING id",
                )
                .bind(deck_id)
                .bind(i as i32)
                .bind(c.front.trim())
                .bind(c.back.trim())
                .fetch_one(&mut *conn)
                .await?;
                kept.push(id);
            }
        }
    }
    sqlx::query("DELETE FROM cards WHERE deck_id = $1 AND NOT (id = ANY($2))")
        .bind(deck_id)
        .bind(&kept)
        .execute(&mut *conn)
        .await?;
    sqlx::query("UPDATE decks SET updated_at = now() WHERE id = $1").bind(deck_id).execute(&mut *conn).await?;
    Ok(())
}

async fn detail(ctx: &AppContext, deck: DeckSummary) -> AppResult<DeckDetail> {
    let cards = cards_of(&ctx.db, deck.id).await?;
    Ok(DeckDetail { deck, cards })
}

// ---------- Giáo viên ----------

pub async fn list(ctx: &AppContext, actor: &Actor, q: DeckQuery) -> AppResult<Vec<DeckSummary>> {
    actor.require(Permission::TeachingManage)?;
    let pattern = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(|s| format!("%{s}%"));
    Ok(sqlx::query_as::<_, DeckSummary>(crate::sql!(
        "SELECT {SUMMARY_COLS} FROM decks d WHERE d.owner_id = $1 AND d.archived_at IS NULL \
         AND ($2::text IS NULL OR d.title ILIKE $2) AND ($3::uuid IS NULL OR d.subject_id = $3) \
         ORDER BY d.updated_at DESC LIMIT 200"
    ))
    .bind(actor.id)
    .bind(pattern)
    .bind(q.subject_id)
    .fetch_all(&ctx.db)
    .await?)
}

pub async fn get(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<DeckDetail> {
    let deck = find_own(ctx, actor, id).await?;
    detail(ctx, deck).await
}

pub async fn create(ctx: &AppContext, actor: &Actor, input: DeckInput) -> AppResult<DeckDetail> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    let mut tx = ctx.db.begin().await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO decks (owner_id, subject_id, title, description) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(actor.id)
    .bind(input.subject_id)
    .bind(input.title.trim())
    .bind(input.description.trim())
    .fetch_one(&mut *tx)
    .await?;
    replace_cards(&mut tx, id, &input.cards).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: DeckInput) -> AppResult<DeckDetail> {
    input.validate()?;
    let deck = find_own(ctx, actor, id).await?;
    if deck.subject_id != input.subject_id {
        teachers::ensure_can_teach(ctx, actor, input.subject_id).await?;
    }
    let mut tx = ctx.db.begin().await?;
    sqlx::query("UPDATE decks SET subject_id = $2, title = $3, description = $4, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(input.subject_id)
        .bind(input.title.trim())
        .bind(input.description.trim())
        .execute(&mut *tx)
        .await?;
    replace_cards(&mut tx, id, &input.cards).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

/// Ẩn bộ thẻ (lộ trình đang dùng vẫn giữ, nhưng không xuất bản lại được).
pub async fn archive(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    find_own(ctx, actor, id).await?;
    sqlx::query("UPDATE decks SET archived_at = now() WHERE id = $1").bind(id).execute(&ctx.db).await?;
    Ok(())
}

/// Mặt sau của thẻ tạo từ câu hỏi: đáp án đúng + lời giải.
pub fn answer_text(kind: &str, body: &QuestionBody) -> String {
    let letter = |i: usize| char::from(b'A' + i as u8);
    let answer = match kind {
        "single" | "multiple" => {
            let picked: Vec<String> = body
                .options
                .iter()
                .enumerate()
                .filter(|(_, o)| body.correct.contains(&o.id))
                .map(|(i, o)| format!("**{}.** {}", letter(i), o.content))
                .collect();
            format!("Đáp án: {}", picked.join("; "))
        }
        "true_false" => body
            .options
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let mark = if body.correct.contains(&o.id) { "Đúng" } else { "Sai" };
                format!("{}) {} — **{mark}**", char::from(b'a' + i as u8), o.content)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => format!("Đáp án: {}", body.accepted.join("; ")),
    };
    if body.explanation.trim().is_empty() { answer } else { format!("{answer}\n\n{}", body.explanation.trim()) }
}

/// Mặt trước: nội dung câu + các phương án (câu một / nhiều đáp án, đúng sai).
pub fn question_text(kind: &str, body: &QuestionBody) -> String {
    if body.options.is_empty() || kind == "short" {
        return body.content.trim().to_string();
    }
    let opts: Vec<String> = body
        .options
        .iter()
        .enumerate()
        .map(|(i, o)| {
            if kind == "true_false" {
                format!("{}) {}", char::from(b'a' + i as u8), o.content)
            } else {
                format!("**{}.** {}", char::from(b'A' + i as u8), o.content)
            }
        })
        .collect();
    format!("{}\n\n{}", body.content.trim(), opts.join("\n"))
}

/// Thêm thẻ vào cuối bộ từ các câu hỏi của chính giáo viên (phiên bản hiện tại).
pub async fn add_from_questions(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    input: FromQuestionsInput,
) -> AppResult<DeckDetail> {
    let deck = find_own(ctx, actor, id).await?;
    if input.question_ids.is_empty() {
        return Err(AppError::field("questionIds", "Chọn ít nhất một câu hỏi."));
    }
    #[derive(sqlx::FromRow)]
    struct Q {
        id: Uuid,
        kind: String,
        body: sqlx::types::Json<QuestionBody>,
    }
    let rows = sqlx::query_as::<_, Q>(
        "SELECT q.id, q.kind, v.body FROM questions q JOIN question_versions v ON v.id = q.current_version_id \
         WHERE q.id = ANY($1) AND q.owner_id = $2 AND q.archived_at IS NULL",
    )
    .bind(&input.question_ids)
    .bind(actor.id)
    .fetch_all(&ctx.db)
    .await?;
    if rows.len() != input.question_ids.len() {
        return Err(AppError::field("questionIds", "Có câu hỏi không tồn tại hoặc không phải của bạn."));
    }
    let by_id: HashMap<Uuid, &Q> = rows.iter().map(|q| (q.id, q)).collect();
    let mut cards: Vec<CardInput> = cards_of(&ctx.db, deck.id)
        .await?
        .into_iter()
        .map(|c| CardInput { id: Some(c.id), front: c.front, back: c.back })
        .collect();
    for qid in &input.question_ids {
        let q = by_id[qid];
        cards.push(CardInput {
            id: None,
            front: question_text(&q.kind, &q.body.0),
            back: answer_text(&q.kind, &q.body.0),
        });
    }
    if cards.len() > MAX_CARDS {
        return Err(AppError::field("questionIds", format!("Bộ thẻ tối đa {MAX_CARDS} thẻ.")));
    }
    let mut tx = ctx.db.begin().await?;
    replace_cards(&mut tx, deck.id, &cards).await?;
    tx.commit().await?;
    get(ctx, actor, id).await
}

// ---------- Dùng trong lộ trình ----------

pub async fn refs(ctx: &AppContext, ids: &[Uuid]) -> AppResult<Vec<DeckRef>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as::<_, DeckRef>(
        "SELECT d.id, d.owner_id, d.title, d.archived_at IS NOT NULL AS archived, \
         (SELECT count(*) FROM cards c WHERE c.deck_id = d.id) AS card_count FROM decks d WHERE d.id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(&ctx.db)
    .await?)
}

/// Các cặp (học sinh, bộ thẻ) đã ôn hết mọi thẻ ít nhất một lần.
pub async fn completed_pairs(ctx: &AppContext, deck_ids: &[Uuid], user: Option<Uuid>) -> AppResult<Vec<(Uuid, Uuid)>> {
    if deck_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT r.user_id, r.deck_id FROM card_reviews r WHERE r.deck_id = ANY($1) AND ($2::uuid IS NULL OR r.user_id = $2) \
         GROUP BY r.user_id, r.deck_id \
         HAVING count(*) >= (SELECT count(*) FROM cards c WHERE c.deck_id = r.deck_id) \
            AND (SELECT count(*) FROM cards c WHERE c.deck_id = r.deck_id) > 0",
    )
    .bind(deck_ids)
    .bind(user)
    .fetch_all(&ctx.db)
    .await?)
}

/// Bộ thẻ + lịch ôn của người học.
pub async fn study(ctx: &AppContext, user_id: Uuid, deck_id: Uuid) -> AppResult<StudyDeck> {
    let (title, description): (String, String) = sqlx::query_as("SELECT title, description FROM decks WHERE id = $1")
        .bind(deck_id)
        .fetch_optional(&ctx.db)
        .await?
        .ok_or_else(not_found)?;
    let cards = cards_of(&ctx.db, deck_id).await?;
    let reviews: HashMap<Uuid, ReviewRow> = sqlx::query_as::<_, ReviewRow>(
        "SELECT card_id, repetitions, interval_days, ease, lapses, review_count, due_at FROM card_reviews \
         WHERE user_id = $1 AND deck_id = $2",
    )
    .bind(user_id)
    .bind(deck_id)
    .fetch_all(&ctx.db)
    .await?
    .into_iter()
    .map(|r| (r.card_id, r))
    .collect();
    let now = Utc::now();
    let mut due_count = 0;
    let mut new_count = 0;
    let mut next_due_at: Option<DateTime<Utc>> = None;
    let out = cards
        .into_iter()
        .map(|c| {
            let r = reviews.get(&c.id);
            let state = match r {
                None => "new",
                Some(r) if r.repetitions == 0 => "learning",
                Some(_) => "review",
            };
            match r {
                None => new_count += 1,
                Some(r) if r.due_at <= now => due_count += 1,
                Some(r) => next_due_at = Some(next_due_at.map_or(r.due_at, |d| d.min(r.due_at))),
            }
            StudyCard {
                id: c.id,
                front: c.front,
                back: c.back,
                state,
                due_at: r.map(|r| r.due_at),
                interval_days: r.map_or(0, |r| r.interval_days),
                review_count: r.map_or(0, |r| r.review_count),
                next_minutes: srs::preview_minutes(r.map(ReviewRow::state)),
            }
        })
        .collect();
    Ok(StudyDeck { deck_id, title, description, cards: out, due_count, new_count, next_due_at })
}

/// Ghi một lần ôn (học sinh). `save = false` khi giáo viên xem thử: chỉ tính, không lưu.
pub async fn review(
    ctx: &AppContext,
    user_id: Uuid,
    deck_id: Uuid,
    input: ReviewInput,
    save: bool,
) -> AppResult<ReviewResult> {
    let in_deck: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM cards WHERE id = $1 AND deck_id = $2)")
        .bind(input.card_id)
        .bind(deck_id)
        .fetch_one(&ctx.db)
        .await?;
    if !in_deck {
        return Err(AppError::not_found("Không có thẻ này trong bộ thẻ."));
    }
    let prev = sqlx::query_as::<_, ReviewRow>(
        "SELECT card_id, repetitions, interval_days, ease, lapses, review_count, due_at FROM card_reviews \
         WHERE user_id = $1 AND card_id = $2",
    )
    .bind(user_id)
    .bind(input.card_id)
    .fetch_optional(&ctx.db)
    .await?;
    let now = Utc::now();
    let (next, due) = srs::schedule(prev.as_ref().map(ReviewRow::state), input.rating, now);
    if save {
        let mut tx = ctx.db.begin().await?;
        sqlx::query(
            "INSERT INTO card_reviews (user_id, card_id, deck_id, repetitions, interval_days, ease, lapses, review_count, \
             last_rating, due_at, reviewed_at) VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8, $9, $10) \
             ON CONFLICT (user_id, card_id) DO UPDATE SET repetitions = $4, interval_days = $5, ease = $6, lapses = $7, \
             review_count = card_reviews.review_count + 1, last_rating = $8, due_at = $9, reviewed_at = $10",
        )
        .bind(user_id)
        .bind(input.card_id)
        .bind(deck_id)
        .bind(next.repetitions)
        .bind(next.interval_days)
        .bind(next.ease)
        .bind(next.lapses)
        .bind(input.rating.as_str())
        .bind(due)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO card_review_log (user_id, deck_id, card_id, rating) VALUES ($1, $2, $3, $4)")
            .bind(user_id)
            .bind(deck_id)
            .bind(input.card_id)
            .bind(input.rating.as_str())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(ReviewResult { card_id: input.card_id, due_at: due, interval_days: next.interval_days, saved: save })
}

/// Số thẻ đến hạn + thẻ mới của người học trong các bộ thẻ.
pub async fn due_counts(ctx: &AppContext, user_id: Uuid, deck_ids: &[Uuid]) -> AppResult<HashMap<Uuid, (i64, i64)>> {
    if deck_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, (Uuid, i64, i64)>(
        "SELECT c.deck_id, \
           count(*) FILTER (WHERE r.card_id IS NOT NULL AND r.due_at <= now()) AS due, \
           count(*) FILTER (WHERE r.card_id IS NULL) AS fresh \
         FROM cards c LEFT JOIN card_reviews r ON r.card_id = c.id AND r.user_id = $1 \
         WHERE c.deck_id = ANY($2) GROUP BY c.deck_id",
    )
    .bind(user_id)
    .bind(deck_ids)
    .fetch_all(&ctx.db)
    .await?;
    Ok(rows.into_iter().map(|(d, due, fresh)| (d, (due, fresh))).collect())
}

/// Số lần ôn thẻ của học sinh trong khoảng thời gian (báo cáo phụ huynh).
pub async fn reviews_between(
    ctx: &AppContext,
    user_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM card_review_log WHERE user_id = $1 AND reviewed_at >= $2 AND reviewed_at < $3",
    )
    .bind(user_id)
    .bind(from)
    .bind(to)
    .fetch_one(&ctx.db)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::questions::model::QOption;

    fn body(options: &[&str], correct: &[usize], accepted: &[&str], explanation: &str) -> QuestionBody {
        let opts: Vec<QOption> =
            options.iter().map(|c| QOption { id: Uuid::now_v7(), content: (*c).to_string() }).collect();
        QuestionBody {
            content: "Câu hỏi?".into(),
            correct: correct.iter().map(|&i| opts[i].id).collect(),
            options: opts,
            accepted: accepted.iter().map(|s| (*s).to_string()).collect(),
            explanation: explanation.into(),
            shuffle_options: true,
        }
    }

    #[test]
    fn mat_sau_theo_dang_cau() {
        let b = body(&["2", "4"], &[1], &[], "Vì $4^2 = 16$.");
        assert_eq!(answer_text("single", &b), "Đáp án: **B.** 4\n\nVì $4^2 = 16$.");
        assert_eq!(question_text("single", &b), "Câu hỏi?\n\n**A.** 2\n**B.** 4");
        let tf = body(&["x > 0", "x < 0"], &[0], &[], "");
        assert_eq!(answer_text("true_false", &tf), "a) x > 0 — **Đúng**\nb) x < 0 — **Sai**");
        let short = body(&[], &[], &["0,5", "1/2"], "");
        assert_eq!(answer_text("short", &short), "Đáp án: 0,5; 1/2");
        assert_eq!(question_text("short", &short), "Câu hỏi?");
    }
}
