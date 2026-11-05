use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, Postgres, QueryBuilder, types::Json};
use uuid::Uuid;

use super::model::{
    AttemptItemRow, AttemptListRow, AttemptRow, BestScoreRow, ExamRef, ExamRow, RegradeItemRow, RuleRow,
    SectionQuestionRow, SectionRow,
};
use crate::modules::questions::grading::Answer;

const EXAM_COLS: &str = "e.id, e.owner_id, e.subject_id, e.grade_id, e.title, e.description, e.kind, e.time_limit_minutes, \
     e.max_attempts, e.opens_at, e.closes_at, e.shuffle_questions, e.shuffle_options, e.show_answers, e.archived_at, \
     e.created_at, e.updated_at, \
     (SELECT count(*) FROM attempts a WHERE a.exam_id = e.id AND NOT a.is_preview) AS attempt_count";

/// Số câu mỗi lượt làm = câu cố định + tổng số câu rút theo ma trận.
const ITEM_COUNT: &str = "(SELECT count(*) FROM exam_section_questions q JOIN exam_sections s ON s.id = q.section_id WHERE s.exam_id = e.id) \
     + (SELECT COALESCE(sum(r.count), 0) FROM exam_section_rules r JOIN exam_sections s ON s.id = r.section_id WHERE s.exam_id = e.id)";

pub struct ExamFields<'a> {
    pub subject_id: Uuid,
    pub grade_id: Option<Uuid>,
    pub title: &'a str,
    pub description: &'a str,
    pub kind: &'a str,
    pub time_limit_minutes: Option<i32>,
    pub max_attempts: Option<i32>,
    pub opens_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    pub shuffle_questions: bool,
    pub shuffle_options: bool,
    pub show_answers: &'a str,
}

pub struct NewSection<'a> {
    pub title: &'a str,
    pub scoring: &'a str,
    pub points_per_question: f64,
    pub question_ids: &'a [Uuid],
    pub rules: Vec<NewRule>,
}

pub struct NewRule {
    pub tag_id: Option<Uuid>,
    pub level: Option<&'static str>,
    pub kind: Option<&'static str>,
    pub folder_id: Option<Uuid>,
    pub count: i32,
}

pub struct ExamFilter<'a> {
    pub owner_id: Uuid,
    pub q: Option<&'a str>,
    pub subject_id: Option<Uuid>,
    pub kind: Option<&'a str>,
}

fn push_filter(qb: &mut QueryBuilder<Postgres>, f: &ExamFilter<'_>) {
    qb.push(" WHERE e.archived_at IS NULL AND e.owner_id = ").push_bind(f.owner_id);
    if let Some(q) = f.q {
        qb.push(" AND unaccent(lower(e.title)) LIKE unaccent(lower(")
            .push_bind(format!("%{}%", crate::db::escape_like(q)))
            .push("))");
    }
    if let Some(s) = f.subject_id {
        qb.push(" AND e.subject_id = ").push_bind(s);
    }
    if let Some(k) = f.kind {
        qb.push(" AND e.kind = ").push_bind(k.to_string());
    }
}

pub async fn list<'e, E: PgExecutor<'e>>(
    db: E,
    f: &ExamFilter<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<(ExamRow, i64)>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        #[sqlx(flatten)]
        exam: ExamRow,
        item_count: i64,
    }
    let mut qb = QueryBuilder::new(format!("SELECT {EXAM_COLS}, {ITEM_COUNT} AS item_count FROM exams e"));
    push_filter(&mut qb, f);
    qb.push(" ORDER BY e.updated_at DESC LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);
    Ok(qb.build_query_as::<Row>().fetch_all(db).await?.into_iter().map(|r| (r.exam, r.item_count)).collect())
}

pub async fn count<'e, E: PgExecutor<'e>>(db: E, f: &ExamFilter<'_>) -> sqlx::Result<i64> {
    let mut qb = QueryBuilder::new("SELECT count(*) FROM exams e");
    push_filter(&mut qb, f);
    qb.build_query_scalar::<i64>().fetch_one(db).await
}

pub async fn find<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<ExamRow>> {
    sqlx::query_as::<_, ExamRow>(crate::sql!("SELECT {EXAM_COLS} FROM exams e WHERE e.id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn insert(conn: &mut sqlx::PgConnection, owner_id: Uuid, f: &ExamFields<'_>) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO exams (owner_id, subject_id, grade_id, title, description, kind, time_limit_minutes, max_attempts, \
            opens_at, closes_at, shuffle_questions, shuffle_options, show_answers) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id",
    )
    .bind(owner_id)
    .bind(f.subject_id)
    .bind(f.grade_id)
    .bind(f.title)
    .bind(f.description)
    .bind(f.kind)
    .bind(f.time_limit_minutes)
    .bind(f.max_attempts)
    .bind(f.opens_at)
    .bind(f.closes_at)
    .bind(f.shuffle_questions)
    .bind(f.shuffle_options)
    .bind(f.show_answers)
    .fetch_one(&mut *conn)
    .await
}

pub async fn update(conn: &mut sqlx::PgConnection, id: Uuid, f: &ExamFields<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE exams SET subject_id = $2, grade_id = $3, title = $4, description = $5, kind = $6, time_limit_minutes = $7, \
            max_attempts = $8, opens_at = $9, closes_at = $10, shuffle_questions = $11, shuffle_options = $12, \
            show_answers = $13, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(f.subject_id)
    .bind(f.grade_id)
    .bind(f.title)
    .bind(f.description)
    .bind(f.kind)
    .bind(f.time_limit_minutes)
    .bind(f.max_attempts)
    .bind(f.opens_at)
    .bind(f.closes_at)
    .bind(f.shuffle_questions)
    .bind(f.shuffle_options)
    .bind(f.show_answers)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Ghi lại toàn bộ cấu trúc phần (xóa cũ, thêm mới). Lượt làm cũ không bị ảnh hưởng vì đã chép câu ra `attempt_items`.
pub async fn replace_sections(
    conn: &mut sqlx::PgConnection,
    exam_id: Uuid,
    sections: &[NewSection<'_>],
) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM exam_sections WHERE exam_id = $1").bind(exam_id).execute(&mut *conn).await?;
    for (i, s) in sections.iter().enumerate() {
        let section_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO exam_sections (exam_id, position, title, scoring, points_per_question) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(exam_id)
        .bind(i as i32)
        .bind(s.title)
        .bind(s.scoring)
        .bind(s.points_per_question)
        .fetch_one(&mut *conn)
        .await?;
        sqlx::query(
            "INSERT INTO exam_section_questions (section_id, position, question_id) \
             SELECT $1, (ord - 1)::int, qid FROM unnest($2::uuid[]) WITH ORDINALITY AS t(qid, ord)",
        )
        .bind(section_id)
        .bind(s.question_ids)
        .execute(&mut *conn)
        .await?;
        for (j, r) in s.rules.iter().enumerate() {
            sqlx::query(
                "INSERT INTO exam_section_rules (section_id, position, tag_id, level, kind, folder_id, count) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(section_id)
            .bind(j as i32)
            .bind(r.tag_id)
            .bind(r.level)
            .bind(r.kind)
            .bind(r.folder_id)
            .bind(r.count)
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

pub async fn sections<'e, E: PgExecutor<'e>>(db: E, exam_id: Uuid) -> sqlx::Result<Vec<SectionRow>> {
    sqlx::query_as::<_, SectionRow>(
        "SELECT id, exam_id, position, title, scoring, points_per_question FROM exam_sections \
         WHERE exam_id = $1 ORDER BY position",
    )
    .bind(exam_id)
    .fetch_all(db)
    .await
}

pub async fn section_questions<'e, E: PgExecutor<'e>>(
    db: E,
    section_ids: &[Uuid],
) -> sqlx::Result<Vec<SectionQuestionRow>> {
    sqlx::query_as::<_, SectionQuestionRow>(
        "SELECT section_id, position, question_id FROM exam_section_questions WHERE section_id = ANY($1) ORDER BY position",
    )
    .bind(section_ids)
    .fetch_all(db)
    .await
}

pub async fn rules<'e, E: PgExecutor<'e>>(db: E, section_ids: &[Uuid]) -> sqlx::Result<Vec<RuleRow>> {
    sqlx::query_as::<_, RuleRow>(
        "SELECT section_id, position, tag_id, level, kind, folder_id, count FROM exam_section_rules \
         WHERE section_id = ANY($1) ORDER BY position",
    )
    .bind(section_ids)
    .fetch_all(db)
    .await
}

pub async fn archive<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE exams SET archived_at = now(), updated_at = now() WHERE id = $1").bind(id).execute(db).await?;
    Ok(())
}

pub async fn refs<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid]) -> sqlx::Result<Vec<ExamRef>> {
    sqlx::query_as::<_, ExamRef>(crate::sql!(
        "SELECT e.id, e.owner_id, e.title, e.kind, e.time_limit_minutes, e.archived_at IS NOT NULL AS archived, \
                {ITEM_COUNT} AS item_count FROM exams e WHERE e.id = ANY($1)"
    ))
    .bind(ids)
    .fetch_all(db)
    .await
}

// ---------- Lượt làm bài ----------

const ATTEMPT_COLS: &str = "id, exam_id, user_id, class_id, step_id, is_preview, status, started_at, deadline_at, \
     submitted_at, score, max_score";

pub struct NewAttempt {
    pub exam_id: Uuid,
    pub user_id: Uuid,
    pub class_id: Option<Uuid>,
    pub step_id: Option<Uuid>,
    pub is_preview: bool,
    pub deadline_at: Option<DateTime<Utc>>,
    pub max_score: f64,
}

pub struct NewItem<'a> {
    pub position: i32,
    pub section_position: i32,
    pub section_title: &'a str,
    pub scoring: &'a str,
    pub points: f64,
    pub question_id: Uuid,
    pub question_version_id: Uuid,
    pub option_order: Vec<Uuid>,
}

pub async fn insert_attempt(
    conn: &mut sqlx::PgConnection,
    a: &NewAttempt,
    items: &[NewItem<'_>],
) -> sqlx::Result<Uuid> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO attempts (exam_id, user_id, class_id, step_id, is_preview, deadline_at, max_score) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(a.exam_id)
    .bind(a.user_id)
    .bind(a.class_id)
    .bind(a.step_id)
    .bind(a.is_preview)
    .bind(a.deadline_at)
    .bind(a.max_score)
    .fetch_one(&mut *conn)
    .await?;
    for it in items {
        sqlx::query(
            "INSERT INTO attempt_items (attempt_id, position, section_position, section_title, scoring, points, \
                question_id, question_version_id, option_order) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(it.position)
        .bind(it.section_position)
        .bind(it.section_title)
        .bind(it.scoring)
        .bind(it.points)
        .bind(it.question_id)
        .bind(it.question_version_id)
        .bind(&it.option_order)
        .execute(&mut *conn)
        .await?;
    }
    Ok(id)
}

pub async fn find_attempt<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<AttemptRow>> {
    sqlx::query_as::<_, AttemptRow>(crate::sql!("SELECT {ATTEMPT_COLS} FROM attempts WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

/// Lượt đang làm dở của user với đề (trong lớp học / làm thử).
pub async fn find_in_progress<'e, E: PgExecutor<'e>>(
    db: E,
    user_id: Uuid,
    exam_id: Uuid,
    class_id: Option<Uuid>,
    is_preview: bool,
) -> sqlx::Result<Option<AttemptRow>> {
    sqlx::query_as::<_, AttemptRow>(crate::sql!(
        "SELECT {ATTEMPT_COLS} FROM attempts WHERE user_id = $1 AND exam_id = $2 AND class_id IS NOT DISTINCT FROM $3 \
         AND is_preview = $4 AND status = 'in_progress' ORDER BY started_at DESC LIMIT 1"
    ))
    .bind(user_id)
    .bind(exam_id)
    .bind(class_id)
    .bind(is_preview)
    .fetch_optional(db)
    .await
}

pub async fn count_attempts<'e, E: PgExecutor<'e>>(
    db: E,
    user_id: Uuid,
    exam_id: Uuid,
    class_id: Option<Uuid>,
) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM attempts WHERE user_id = $1 AND exam_id = $2 AND class_id IS NOT DISTINCT FROM $3 \
         AND NOT is_preview",
    )
    .bind(user_id)
    .bind(exam_id)
    .bind(class_id)
    .fetch_one(db)
    .await
}

pub async fn items<'e, E: PgExecutor<'e>>(db: E, attempt_id: Uuid) -> sqlx::Result<Vec<AttemptItemRow>> {
    sqlx::query_as::<_, AttemptItemRow>(
        "SELECT position, section_position, section_title, scoring, points, question_id, question_version_id, option_order, \
                answer, checked, is_correct, score FROM attempt_items WHERE attempt_id = $1 ORDER BY position",
    )
    .bind(attempt_id)
    .fetch_all(db)
    .await
}

pub async fn save_answer<'e, E: PgExecutor<'e>>(
    db: E,
    attempt_id: Uuid,
    position: i32,
    answer: &Answer,
) -> sqlx::Result<bool> {
    let res =
        sqlx::query("UPDATE attempt_items SET answer = $3 WHERE attempt_id = $1 AND position = $2 AND NOT checked")
            .bind(attempt_id)
            .bind(position)
            .bind(Json(answer))
            .execute(db)
            .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn set_item_grade<'e, E: PgExecutor<'e>>(
    db: E,
    attempt_id: Uuid,
    position: i32,
    is_correct: bool,
    score: f64,
    checked: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE attempt_items SET is_correct = $3, score = $4, checked = checked OR $5 WHERE attempt_id = $1 AND position = $2",
    )
    .bind(attempt_id)
    .bind(position)
    .bind(is_correct)
    .bind(score)
    .bind(checked)
    .execute(db)
    .await?;
    Ok(())
}

/// Đánh dấu đã nộp (chỉ khi đang làm). Trả `false` nếu đã nộp trước đó.
pub async fn mark_submitted<'e, E: PgExecutor<'e>>(db: E, id: Uuid, score: f64) -> sqlx::Result<bool> {
    let res = sqlx::query(
        "UPDATE attempts SET status = 'submitted', submitted_at = now(), score = $2 WHERE id = $1 AND status = 'in_progress'",
    )
    .bind(id)
    .bind(score)
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn list_for_exam<'e, E: PgExecutor<'e>>(
    db: E,
    exam_id: Uuid,
    class_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> sqlx::Result<Vec<AttemptListRow>> {
    sqlx::query_as::<_, AttemptListRow>(
        "SELECT a.id, a.exam_id, a.user_id, u.username, u.full_name, a.class_id, a.is_preview, a.status, a.started_at, \
                a.submitted_at, a.score, a.max_score \
         FROM attempts a JOIN users u ON u.id = a.user_id \
         WHERE a.exam_id = $1 AND ($2::uuid IS NULL OR a.class_id = $2) AND ($3::uuid IS NULL OR a.user_id = $3) \
         ORDER BY a.started_at DESC",
    )
    .bind(exam_id)
    .bind(class_id)
    .bind(user_id)
    .fetch_all(db)
    .await
}

/// Điểm cao nhất (bài đã nộp), số lần nộp, lượt đang làm dở — theo học sinh × đề trong một lớp học.
pub async fn best_scores<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Uuid,
    exam_ids: &[Uuid],
    user_id: Option<Uuid>,
) -> sqlx::Result<Vec<BestScoreRow>> {
    sqlx::query_as::<_, BestScoreRow>(
        "SELECT a.user_id, a.exam_id, \
                max(a.score) FILTER (WHERE a.status = 'submitted') AS best_score, \
                max(a.max_score) AS max_score, \
                count(*) FILTER (WHERE a.status = 'submitted') AS submitted, \
                (array_agg(a.id ORDER BY a.started_at DESC) FILTER (WHERE a.status = 'in_progress'))[1] AS in_progress \
         FROM attempts a WHERE a.class_id = $1 AND a.exam_id = ANY($2) AND NOT a.is_preview \
           AND ($3::uuid IS NULL OR a.user_id = $3) \
         GROUP BY a.user_id, a.exam_id",
    )
    .bind(class_id)
    .bind(exam_ids)
    .bind(user_id)
    .fetch_all(db)
    .await
}

/// Lượt làm đang dở đã quá hạn (để tự nộp).
pub async fn overdue_in_progress<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Uuid,
    user_id: Option<Uuid>,
    grace_secs: i64,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM attempts WHERE class_id = $1 AND status = 'in_progress' AND deadline_at IS NOT NULL \
         AND deadline_at + make_interval(secs => $3) < now() AND ($2::uuid IS NULL OR user_id = $2)",
    )
    .bind(class_id)
    .bind(user_id)
    .bind(grace_secs as f64)
    .fetch_all(db)
    .await
}

// ---------- Chấm lại ----------

pub async fn submitted_items_for_question<'e, E: PgExecutor<'e>>(
    db: E,
    question_id: Uuid,
) -> sqlx::Result<Vec<RegradeItemRow>> {
    sqlx::query_as::<_, RegradeItemRow>(
        "SELECT ai.attempt_id, ai.position, ai.scoring, ai.points, ai.answer, ai.score \
         FROM attempt_items ai JOIN attempts a ON a.id = ai.attempt_id \
         WHERE ai.question_id = $1 AND a.status = 'submitted'",
    )
    .bind(question_id)
    .fetch_all(db)
    .await
}

pub async fn set_item_version<'e, E: PgExecutor<'e>>(
    db: E,
    attempt_id: Uuid,
    position: i32,
    version_id: Uuid,
    is_correct: bool,
    score: f64,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE attempt_items SET question_version_id = $3, is_correct = $4, score = $5 WHERE attempt_id = $1 AND position = $2",
    )
    .bind(attempt_id)
    .bind(position)
    .bind(version_id)
    .bind(is_correct)
    .bind(score)
    .execute(db)
    .await?;
    Ok(())
}

/// Tính lại tổng điểm các lượt làm từ điểm từng câu.
pub async fn recompute_scores<'e, E: PgExecutor<'e>>(db: E, attempt_ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE attempts a SET score = round((SELECT COALESCE(sum(score), 0) FROM attempt_items i WHERE i.attempt_id = a.id)::numeric, 2)::float8 \
         WHERE a.id = ANY($1)",
    )
    .bind(attempt_ids)
    .execute(db)
    .await?;
    Ok(())
}
