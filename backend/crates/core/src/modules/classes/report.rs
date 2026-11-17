//! Báo cáo lớp học cho giáo viên (M17, đề xuất D11) + xuất bảng điểm Excel.
//!
//! - Bảng điểm: điểm cao nhất của từng học sinh ở từng đề trong lộ trình đã xuất bản.
//! - Tỷ lệ điểm theo danh mục / mức độ, câu sai nhiều: tính trên mọi lượt đã nộp trong lớp học (không tính làm thử).
//! - Học sinh cần chú ý: lâu không học (≥ 7 ngày), tiến độ thấp (< 30 % sau 7 ngày vào lớp), điểm thấp (TB < 50 %).

use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use rust_xlsxwriter::{Format, Workbook};
use serde::Serialize;
use uuid::Uuid;

use super::{dto::ProgressSummary, path, repository as repo, service::load_staff_class};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    modules::{
        exams::service as exams,
        sessions::{service as sessions, time::to_local},
    },
    security::Actor,
};

pub const INACTIVE_DAYS: i64 = 7;
pub const LOW_PROGRESS_PERCENT: i64 = 30;
pub const LOW_SCORE_PERCENT: f64 = 50.0;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportExam {
    pub step_id: Uuid,
    pub exam_id: Uuid,
    pub title: String,
    pub max_score: Option<f64>,
    /// Số học sinh đã nộp.
    pub submitted: i64,
    pub avg_score: Option<f64>,
    pub avg_percent: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportStudent {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub joined_at: DateTime<Utc>,
    pub progress: ProgressSummary,
    /// Điểm cao nhất từng đề, cùng thứ tự `exams` (`null` = chưa nộp).
    pub scores: Vec<Option<f64>>,
    pub avg_percent: Option<f64>,
    pub last_active_at: Option<DateTime<Utc>>,
    pub sessions_total: i64,
    pub sessions_attended: i64,
    /// Số phụ huynh đã xác nhận và đang nhận báo cáo.
    pub guardians: i64,
    /// inactive | low_progress | low_score
    pub flags: Vec<&'static str>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ScoreStat {
    /// Danh mục: id; mức độ: `null`.
    pub id: Option<Uuid>,
    /// Tên danh mục / mã mức độ.
    pub name: String,
    pub earned: f64,
    pub possible: f64,
    pub answers: i64,
    #[sqlx(skip)]
    pub percent: f64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HardQuestion {
    pub question_id: Uuid,
    pub content: String,
    pub level: String,
    pub kind: String,
    pub answers: i64,
    pub wrong: i64,
    pub earned: f64,
    pub possible: f64,
    #[sqlx(skip)]
    pub percent: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassReport {
    pub class_title: String,
    pub generated_at: DateTime<Utc>,
    pub exams: Vec<ReportExam>,
    pub students: Vec<ReportStudent>,
    pub topics: Vec<ScoreStat>,
    pub levels: Vec<ScoreStat>,
    pub hard_questions: Vec<HardQuestion>,
}

fn percent(earned: f64, possible: f64) -> f64 {
    if possible <= 0.0 { 0.0 } else { (earned / possible * 1000.0).round() / 10.0 }
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Cắt chuỗi tối đa `max` ký tự (theo ký tự, không cắt giữa chữ có dấu).
fn snippet(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

/// Các cờ cần chú ý của một học sinh (hàm thuần).
pub fn flags_for(
    progress: &ProgressSummary,
    avg_percent: Option<f64>,
    last_active_at: Option<DateTime<Utc>>,
    joined_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Vec<&'static str> {
    let week = TimeDelta::days(INACTIVE_DAYS);
    let settled = now - joined_at >= week;
    let mut out = Vec::new();
    if settled && last_active_at.is_none_or(|t| now - t >= week) {
        out.push("inactive");
    }
    if settled && progress.total > 0 && progress.percent < LOW_PROGRESS_PERCENT {
        out.push("low_progress");
    }
    if avg_percent.is_some_and(|p| p < LOW_SCORE_PERCENT) {
        out.push("low_score");
    }
    out
}

const SUBMITTED: &str = "a.class_id = $1 AND a.status = 'submitted' AND NOT a.is_preview";

pub async fn class_report(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<ClassReport> {
    let class = load_staff_class(ctx, actor, class_id).await?;
    let progress = path::class_progress(ctx, actor, class_id).await?;
    let members = repo::members(&ctx.db, class_id, Some("active")).await?;
    let joined: HashMap<Uuid, DateTime<Utc>> = members.iter().map(|m| (m.user_id, m.created_at)).collect();
    let user_ids: Vec<Uuid> = members.iter().map(|m| m.user_id).collect();
    let now = Utc::now();

    // Đề trong lộ trình đã xuất bản + điểm cao nhất.
    let exam_steps = path::published_exam_steps(ctx, class_id).await?;
    let exam_ids: Vec<Uuid> = exam_steps.iter().map(|(_, e, _)| *e).collect();
    let best = exams::best_scores(ctx, class_id, &exam_ids, None).await?;
    let best_of = |u: Uuid, e: Uuid| best.iter().find(|b| b.user_id == u && b.exam_id == e && b.submitted > 0);

    let last_active: HashMap<Uuid, DateTime<Utc>> = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
        "SELECT u, max(t) FROM ( \
           SELECT user_id AS u, completed_at AS t FROM step_progress WHERE class_id = $1 \
           UNION ALL SELECT user_id, COALESCE(submitted_at, started_at) FROM attempts WHERE class_id = $1 AND NOT is_preview \
           UNION ALL SELECT a.user_id, a.joined_at FROM session_attendees a JOIN sessions s ON s.id = a.session_id \
             WHERE s.class_id = $1 AND a.joined_at IS NOT NULL \
           UNION ALL SELECT l.user_id, l.reviewed_at FROM card_review_log l \
             WHERE l.deck_id IN (SELECT deck_id FROM path_steps WHERE class_id = $1 AND deck_id IS NOT NULL) \
         ) x GROUP BY u",
    )
    .bind(class_id)
    .fetch_all(&ctx.db)
    .await?
    .into_iter()
    .collect();
    let attendance: HashMap<Uuid, (i64, i64)> = sessions::attendance_stats(ctx, Some(class_id), &user_ids, None, now)
        .await?
        .into_iter()
        .map(|(u, total, attended)| (u, (total, attended)))
        .collect();
    let guardians: HashMap<Uuid, i64> = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT student_id, count(*) FROM student_guardians WHERE student_id = ANY($1) \
         AND consent_status = 'confirmed' AND receive_reports GROUP BY student_id",
    )
    .bind(&user_ids)
    .fetch_all(&ctx.db)
    .await?
    .into_iter()
    .collect();

    let students: Vec<ReportStudent> = progress
        .students
        .into_iter()
        .map(|p| {
            let scores: Vec<Option<f64>> =
                exam_ids.iter().map(|&e| best_of(p.user_id, e).and_then(|b| b.best_score)).collect();
            let pcts: Vec<f64> = exam_ids
                .iter()
                .filter_map(|&e| best_of(p.user_id, e))
                .filter_map(|b| Some(percent(b.best_score?, b.max_score?)))
                .collect();
            let avg_percent = (!pcts.is_empty()).then(|| round2(pcts.iter().sum::<f64>() / pcts.len() as f64));
            let last_active_at = last_active.get(&p.user_id).copied();
            let joined_at = joined.get(&p.user_id).copied().unwrap_or(now);
            let (sessions_total, sessions_attended) = attendance.get(&p.user_id).copied().unwrap_or((0, 0));
            ReportStudent {
                flags: flags_for(&p.progress, avg_percent, last_active_at, joined_at, now),
                user_id: p.user_id,
                username: p.username,
                full_name: p.full_name,
                joined_at,
                progress: p.progress,
                scores,
                avg_percent,
                last_active_at,
                sessions_total,
                sessions_attended,
                guardians: guardians.get(&p.user_id).copied().unwrap_or(0),
            }
        })
        .collect();

    let exams_out = exam_steps
        .iter()
        .enumerate()
        .map(|(i, (step_id, exam_id, title))| {
            let rows: Vec<_> = best.iter().filter(|b| b.exam_id == *exam_id && b.submitted > 0).collect();
            let max_score = rows.iter().find_map(|b| b.max_score);
            let scored: Vec<f64> = students.iter().filter_map(|s| s.scores[i]).collect();
            let avg_score = (!scored.is_empty()).then(|| round2(scored.iter().sum::<f64>() / scored.len() as f64));
            ReportExam {
                step_id: *step_id,
                exam_id: *exam_id,
                title: title.clone(),
                max_score,
                submitted: scored.len() as i64,
                avg_percent: avg_score.zip(max_score).map(|(a, m)| percent(a, m)),
                avg_score,
            }
        })
        .collect();

    let mut topics = sqlx::query_as::<_, ScoreStat>(crate::sql!(
        "SELECT t.id, t.name, sum(ai.score) AS earned, sum(ai.points) AS possible, count(*) AS answers \
         FROM attempt_items ai JOIN attempts a ON a.id = ai.attempt_id \
         JOIN question_tags qt ON qt.question_id = ai.question_id JOIN tags t ON t.id = qt.tag_id \
         WHERE {SUBMITTED} GROUP BY t.id, t.name"
    ))
    .bind(class_id)
    .fetch_all(&ctx.db)
    .await?;
    let mut levels = sqlx::query_as::<_, ScoreStat>(crate::sql!(
        "SELECT NULL::uuid AS id, q.level AS name, sum(ai.score) AS earned, sum(ai.points) AS possible, count(*) AS answers \
         FROM attempt_items ai JOIN attempts a ON a.id = ai.attempt_id JOIN questions q ON q.id = ai.question_id \
         WHERE {SUBMITTED} GROUP BY q.level"
    ))
    .bind(class_id)
    .fetch_all(&ctx.db)
    .await?;
    for s in topics.iter_mut().chain(levels.iter_mut()) {
        s.percent = percent(s.earned, s.possible);
        s.earned = round2(s.earned);
        s.possible = round2(s.possible);
    }
    // Danh mục yếu nhất lên đầu; mức độ theo thứ tự nhận biết → vận dụng cao.
    topics.sort_by(|a, b| a.percent.total_cmp(&b.percent).then(b.answers.cmp(&a.answers)));
    let order = ["nhan_biet", "thong_hieu", "van_dung", "van_dung_cao"];
    levels.sort_by_key(|l| order.iter().position(|o| *o == l.name).unwrap_or(9));

    let mut hard_questions = sqlx::query_as::<_, HardQuestion>(crate::sql!(
        "SELECT ai.question_id, qv.body->>'content' AS content, q.level, q.kind, count(*) AS answers, \
           count(*) FILTER (WHERE ai.score + 1e-9 < ai.points) AS wrong, sum(ai.score) AS earned, sum(ai.points) AS possible \
         FROM attempt_items ai JOIN attempts a ON a.id = ai.attempt_id JOIN questions q ON q.id = ai.question_id \
         JOIN question_versions qv ON qv.id = q.current_version_id \
         WHERE {SUBMITTED} GROUP BY ai.question_id, q.id, qv.id \
         HAVING count(*) FILTER (WHERE ai.score + 1e-9 < ai.points) > 0 \
         ORDER BY sum(ai.score) / NULLIF(sum(ai.points), 0) ASC NULLS LAST, count(*) DESC LIMIT 10"
    ))
    .bind(class_id)
    .fetch_all(&ctx.db)
    .await?;
    for q in &mut hard_questions {
        q.percent = percent(q.earned, q.possible);
        q.content = snippet(&q.content, 200);
        q.earned = round2(q.earned);
        q.possible = round2(q.possible);
    }

    Ok(ClassReport {
        class_title: class.title,
        generated_at: now,
        exams: exams_out,
        students,
        topics,
        levels,
        hard_questions,
    })
}

fn level_label(code: &str) -> &str {
    match code {
        "nhan_biet" => "Nhận biết",
        "thong_hieu" => "Thông hiểu",
        "van_dung" => "Vận dụng",
        "van_dung_cao" => "Vận dụng cao",
        other => other,
    }
}

fn flag_label(flag: &str) -> &str {
    match flag {
        "inactive" => "Lâu không học",
        "low_progress" => "Tiến độ thấp",
        "low_score" => "Điểm thấp",
        other => other,
    }
}

/// Tên file an toàn: bỏ dấu tiếng Việt, chỉ giữ chữ / số / gạch ngang.
fn file_slug(title: &str) -> String {
    let base: String = crate::slug::slugify(title);
    if base.is_empty() { "lop-hoc".into() } else { base }
}

/// File Excel: bảng điểm + theo danh mục / mức độ + câu sai nhiều. Trả (tên file, nội dung).
pub async fn report_xlsx(ctx: &AppContext, actor: &Actor, class_id: Uuid) -> AppResult<(String, Vec<u8>)> {
    let r = class_report(ctx, actor, class_id).await?;
    let fail = |e: rust_xlsxwriter::XlsxError| AppError::internal(e);
    let mut wb = Workbook::new();
    let head = Format::new().set_bold().set_background_color("#E8F0FE").set_text_wrap();
    let pct = Format::new().set_num_format("0.0");
    let num = Format::new().set_num_format("0.##");
    let wrap = Format::new().set_text_wrap();

    // Sheet 1: bảng điểm
    let ws = wb.add_worksheet();
    ws.set_name("Bảng điểm").map_err(fail)?;
    let mut headers: Vec<String> =
        ["STT", "Họ tên", "Tên đăng nhập", "Tiến độ (%)"].iter().map(|s| (*s).to_string()).collect();
    for e in &r.exams {
        headers.push(match e.max_score {
            Some(m) => format!("{} (/{})", e.title, round2(m)),
            None => e.title.clone(),
        });
    }
    headers.extend(
        ["Điểm TB (%)", "Buổi có mặt", "Hoạt động gần nhất", "Cần chú ý", "Phụ huynh nhận báo cáo"]
            .iter()
            .map(|s| (*s).to_string()),
    );
    for (c, h) in headers.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, h, &head).map_err(fail)?;
        ws.set_column_width(
            c as u16,
            match c {
                0 => 5.0,
                1 => 26.0,
                2 => 16.0,
                _ => 14.0,
            },
        )
        .map_err(fail)?;
    }
    for (i, s) in r.students.iter().enumerate() {
        let row = (i + 1) as u32;
        let mut c = 0u16;
        ws.write_number(row, c, (i + 1) as f64).map_err(fail)?;
        c += 1;
        ws.write_string(row, c, &s.full_name).map_err(fail)?;
        c += 1;
        ws.write_string(row, c, &s.username).map_err(fail)?;
        c += 1;
        ws.write_number(row, c, s.progress.percent as f64).map_err(fail)?;
        c += 1;
        for score in &s.scores {
            if let Some(v) = score {
                ws.write_number_with_format(row, c, *v, &num).map_err(fail)?;
            }
            c += 1;
        }
        if let Some(v) = s.avg_percent {
            ws.write_number_with_format(row, c, v, &pct).map_err(fail)?;
        }
        c += 1;
        ws.write_string(row, c, format!("{}/{}", s.sessions_attended, s.sessions_total)).map_err(fail)?;
        c += 1;
        if let Some(t) = s.last_active_at {
            ws.write_string(row, c, to_local(t).format("%d/%m/%Y %H:%M").to_string()).map_err(fail)?;
        }
        c += 1;
        let flags: Vec<&str> = s.flags.iter().map(|f| flag_label(f)).collect();
        ws.write_string(row, c, flags.join(", ")).map_err(fail)?;
        c += 1;
        ws.write_number(row, c, s.guardians as f64).map_err(fail)?;
    }
    ws.set_freeze_panes(1, 2).map_err(fail)?;

    // Sheet 2: danh mục + mức độ
    let ws = wb.add_worksheet();
    ws.set_name("Theo danh mục").map_err(fail)?;
    for (c, h) in ["Danh mục / mức độ", "Lượt trả lời", "Tỷ lệ điểm (%)"].iter().enumerate() {
        ws.write_string_with_format(0, c as u16, *h, &head).map_err(fail)?;
        ws.set_column_width(c as u16, if c == 0 { 36.0 } else { 16.0 }).map_err(fail)?;
    }
    let mut row = 1u32;
    for s in &r.topics {
        ws.write_string(row, 0, &s.name).map_err(fail)?;
        ws.write_number(row, 1, s.answers as f64).map_err(fail)?;
        ws.write_number_with_format(row, 2, s.percent, &pct).map_err(fail)?;
        row += 1;
    }
    row += 1;
    for s in &r.levels {
        ws.write_string(row, 0, format!("Mức độ: {}", level_label(&s.name))).map_err(fail)?;
        ws.write_number(row, 1, s.answers as f64).map_err(fail)?;
        ws.write_number_with_format(row, 2, s.percent, &pct).map_err(fail)?;
        row += 1;
    }

    // Sheet 3: câu sai nhiều
    let ws = wb.add_worksheet();
    ws.set_name("Câu sai nhiều").map_err(fail)?;
    for (c, h) in ["STT", "Nội dung câu hỏi", "Mức độ", "Lượt trả lời", "Lượt sai", "Tỷ lệ điểm (%)"].iter().enumerate()
    {
        ws.write_string_with_format(0, c as u16, *h, &head).map_err(fail)?;
        ws.set_column_width(c as u16, if c == 1 { 60.0 } else { 13.0 }).map_err(fail)?;
    }
    for (i, q) in r.hard_questions.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write_number(row, 0, (i + 1) as f64).map_err(fail)?;
        ws.write_string_with_format(row, 1, &q.content, &wrap).map_err(fail)?;
        ws.write_string(row, 2, level_label(&q.level)).map_err(fail)?;
        ws.write_number(row, 3, q.answers as f64).map_err(fail)?;
        ws.write_number(row, 4, q.wrong as f64).map_err(fail)?;
        ws.write_number_with_format(row, 5, q.percent, &pct).map_err(fail)?;
    }

    let bytes = wb.save_to_buffer().map_err(fail)?;
    let day = to_local(r.generated_at).format("%Y-%m-%d");
    Ok((format!("bang-diem-{}-{day}.xlsx", file_slug(&r.class_title)), bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn co_can_chu_y() {
        let now = t("2026-11-13T12:00:00Z");
        let old = t("2026-10-01T00:00:00Z");
        let low = ProgressSummary::new(1, 10);
        let ok = ProgressSummary::new(8, 10);
        // Lâu không học + tiến độ thấp + điểm thấp
        assert_eq!(
            flags_for(&low, Some(40.0), Some(t("2026-11-01T00:00:00Z")), old, now),
            vec!["inactive", "low_progress", "low_score"]
        );
        // Học đều, điểm ổn
        assert!(flags_for(&ok, Some(80.0), Some(t("2026-11-12T00:00:00Z")), old, now).is_empty());
        // Mới vào lớp 2 ngày: chưa xét lâu không học / tiến độ
        assert!(flags_for(&low, None, None, t("2026-11-11T00:00:00Z"), now).is_empty());
    }

    #[test]
    fn ti_le_va_cat_chuoi() {
        assert_eq!(percent(1.75, 2.25), 77.8);
        assert_eq!(percent(1.0, 0.0), 0.0);
        assert_eq!(snippet("  Giá trị của căn  ", 100), "Giá trị của căn");
        assert_eq!(snippet("Phương trình bậc hai", 9), "Phương tr…");
    }
}
