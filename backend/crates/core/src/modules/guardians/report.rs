//! Nội dung báo cáo học tập gửi phụ huynh (email chữ thường; bản HTML làm sau khi worker gửi được HTML).

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    context::AppContext,
    error::AppResult,
    modules::{
        classes::path,
        flashcards,
        sessions::{service as sessions, time::to_local},
    },
};

/// Một bài đã nộp trong kỳ báo cáo.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExamLine {
    pub title: String,
    pub score: f64,
    pub max_score: f64,
    pub submitted_at: DateTime<Utc>,
}

/// Số liệu một lớp học trong kỳ báo cáo.
#[derive(Debug, Clone)]
pub struct ClassLine {
    pub title: String,
    pub teacher: String,
    pub done: i64,
    pub total: i64,
    pub percent: i64,
    pub steps_this_period: i64,
    pub exams: Vec<ExamLine>,
    pub sessions_total: i64,
    pub sessions_attended: i64,
}

#[derive(Debug, Clone)]
pub struct StudentReport {
    pub student_name: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub classes: Vec<ClassLine>,
    pub cards_reviewed: i64,
}

/// "1,75" — số kiểu Việt Nam, tối đa 2 chữ số thập phân.
pub fn vn_number(x: f64) -> String {
    let s = format!("{:.2}", (x * 100.0).round() / 100.0);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.replace('.', ",")
}

/// Gom số liệu của học sinh trong [from, to); `only_class` = chỉ một lớp học (giáo viên gửi).
pub async fn collect(
    ctx: &AppContext,
    student_id: Uuid,
    student_name: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    only_class: Option<Uuid>,
) -> AppResult<StudentReport> {
    let classes = sqlx::query_as::<_, (Uuid, String, String)>(
        "SELECT c.id, c.title, u.full_name FROM class_members m JOIN classes c ON c.id = m.class_id \
         JOIN users u ON u.id = c.owner_id \
         WHERE m.user_id = $1 AND m.status = 'active' AND c.archived_at IS NULL AND ($2::uuid IS NULL OR c.id = $2) \
         ORDER BY c.title",
    )
    .bind(student_id)
    .bind(only_class)
    .fetch_all(&ctx.db)
    .await?;
    let mut lines = Vec::with_capacity(classes.len());
    for (class_id, title, teacher) in classes {
        let progress = path::progress_summary(ctx, class_id, student_id).await?;
        let steps_this_period: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM step_progress WHERE class_id = $1 AND user_id = $2 AND completed_at >= $3 AND completed_at < $4",
        )
        .bind(class_id)
        .bind(student_id)
        .bind(from)
        .bind(to)
        .fetch_one(&ctx.db)
        .await?;
        let exams = sqlx::query_as::<_, ExamLine>(
            "SELECT e.title, a.score, a.max_score, a.submitted_at FROM attempts a JOIN exams e ON e.id = a.exam_id \
             WHERE a.class_id = $1 AND a.user_id = $2 AND a.status = 'submitted' AND NOT a.is_preview \
               AND a.submitted_at >= $3 AND a.submitted_at < $4 ORDER BY a.submitted_at",
        )
        .bind(class_id)
        .bind(student_id)
        .bind(from)
        .bind(to)
        .fetch_all(&ctx.db)
        .await?;
        let (sessions_total, sessions_attended) =
            sessions::attendance_stats(ctx, Some(class_id), &[student_id], Some(from), to)
                .await?
                .first()
                .map_or((0, 0), |(_, t, a)| (*t, *a));
        lines.push(ClassLine {
            title,
            teacher,
            done: progress.done,
            total: progress.total,
            percent: progress.percent,
            steps_this_period,
            exams,
            sessions_total,
            sessions_attended,
        });
    }
    Ok(StudentReport {
        student_name: student_name.to_string(),
        from,
        to,
        classes: lines,
        cards_reviewed: flashcards::reviews_between(ctx, student_id, from, to).await?,
    })
}

/// Nội dung email (chữ thường). Hàm thuần để test.
pub fn render_text(
    r: &StudentReport,
    guardian_name: &str,
    note: Option<&str>,
    app_url: &str,
    unsubscribe_url: &str,
) -> String {
    let from = to_local(r.from).format("%d/%m");
    let to = to_local(r.to - chrono::TimeDelta::seconds(1)).format("%d/%m/%Y");
    let mut s = format!(
        "Kính gửi {guardian_name},\n\nSmart Learning Space gửi báo cáo học tập của {} từ {from} đến {to}.\n",
        r.student_name
    );
    if let Some(note) = note.map(str::trim).filter(|n| !n.is_empty()) {
        s.push_str(&format!("\nNhận xét của giáo viên:\n{note}\n"));
    }
    if r.classes.is_empty() {
        s.push_str("\nHọc sinh hiện chưa tham gia lớp học nào.\n");
    }
    for c in &r.classes {
        s.push_str(&format!("\nLớp học \"{}\" (giáo viên {})\n", c.title, c.teacher));
        s.push_str(&format!("- Tiến độ lộ trình: {}/{} bước ({}%)", c.done, c.total, c.percent));
        if c.steps_this_period > 0 {
            s.push_str(&format!(", trong kỳ học xong {} bài", c.steps_this_period));
        }
        s.push('\n');
        if c.exams.is_empty() {
            s.push_str("- Chưa nộp bài kiểm tra / ôn tập nào trong kỳ\n");
        } else {
            s.push_str("- Bài đã nộp trong kỳ:\n");
            for e in &c.exams {
                let pct = if e.max_score > 0.0 { (e.score / e.max_score * 100.0).round() as i64 } else { 0 };
                s.push_str(&format!(
                    "  • {}: {}/{} điểm ({pct}%) — {}\n",
                    e.title,
                    vn_number(e.score),
                    vn_number(e.max_score),
                    to_local(e.submitted_at).format("%d/%m %H:%M")
                ));
            }
        }
        if c.sessions_total > 0 {
            s.push_str(&format!(
                "- Buổi học: có mặt {}/{} buổi{}\n",
                c.sessions_attended,
                c.sessions_total,
                if c.sessions_attended < c.sessions_total { " (có buổi vắng)" } else { "" }
            ));
        }
    }
    if r.cards_reviewed > 0 {
        s.push_str(&format!("\nThẻ ghi nhớ đã ôn trong kỳ: {} lượt.\n", r.cards_reviewed));
    }
    s.push_str(&format!(
        "\nHọc sinh đăng nhập tại {app_url} để xem chi tiết.\n\n\
         Bạn nhận email này vì đã xác nhận nhận báo cáo học tập của {}. Không muốn nhận nữa: {unsubscribe_url}\n",
        r.student_name
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn so_kieu_viet_nam() {
        assert_eq!(vn_number(1.75), "1,75");
        assert_eq!(vn_number(2.0), "2");
        assert_eq!(vn_number(2.5), "2,5");
        assert_eq!(vn_number(1.0 / 3.0), "0,33");
    }

    #[test]
    fn noi_dung_bao_cao() {
        let r = StudentReport {
            student_name: "Trần Thu".into(),
            // Thứ Hai 9/11 0h → Chủ nhật 15/11 20h (giờ VN)
            from: t("2026-11-08T17:00:00Z"),
            to: t("2026-11-15T13:00:00Z"),
            classes: vec![ClassLine {
                title: "Toán 9".into(),
                teacher: "Cô Lan".into(),
                done: 3,
                total: 4,
                percent: 75,
                steps_this_period: 2,
                exams: vec![ExamLine {
                    title: "Kiểm tra 15 phút".into(),
                    score: 1.75,
                    max_score: 2.25,
                    submitted_at: t("2026-11-10T13:05:00Z"),
                }],
                sessions_total: 2,
                sessions_attended: 1,
            }],
            cards_reviewed: 24,
        };
        let s =
            render_text(&r, "Anh Minh", Some("Em cần ôn lại hệ phương trình."), "https://sls.vn", "https://sls.vn/huy");
        assert!(s.starts_with("Kính gửi Anh Minh,"));
        assert!(s.contains("từ 09/11 đến 15/11/2026"));
        assert!(s.contains("Nhận xét của giáo viên:\nEm cần ôn lại hệ phương trình."));
        assert!(s.contains("- Tiến độ lộ trình: 3/4 bước (75%), trong kỳ học xong 2 bài"));
        assert!(s.contains("  • Kiểm tra 15 phút: 1,75/2,25 điểm (78%) — 10/11 20:05"));
        assert!(s.contains("- Buổi học: có mặt 1/2 buổi (có buổi vắng)"));
        assert!(s.contains("Thẻ ghi nhớ đã ôn trong kỳ: 24 lượt."));
        assert!(s.contains("Không muốn nhận nữa: https://sls.vn/huy"));
    }
}
