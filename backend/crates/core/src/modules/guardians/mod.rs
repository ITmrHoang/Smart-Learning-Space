//! Phụ huynh + báo cáo học tập (M20, analyst.md "Thông tin phụ huynh").
//!
//! - Không có tài khoản phụ huynh. Học sinh khai liên hệ (tối đa 3); hệ thống gửi email để phụ huynh bấm xác nhận
//!   đồng ý (xác minh email + bằng chứng đồng ý xử lý dữ liệu của học sinh chưa thành niên). Link có hạn 14 ngày.
//! - Báo cáo tuần: tối Chủ nhật (từ 19h giờ Việt Nam) gửi cho phụ huynh đã xác nhận và đang nhận báo cáo, kỳ báo cáo là
//!   từ 0h thứ Hai. Mỗi phụ huynh một lần / tuần nhờ chỉ mục duy nhất của `report_deliveries` (nhiều instance không gửi trùng).
//! - Giáo viên gửi ngay báo cáo của một lớp học (7 ngày gần nhất) kèm nhận xét.
//! - Mỗi email có link hủy nhận (`unsubscribe_token` riêng của từng phụ huynh, chỉ dùng được để hủy / nhận lại).

pub mod report;

use chrono::{DateTime, Datelike, Duration, NaiveTime, TimeDelta, Utc, Weekday};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    jobs::{Email, Job},
    modules::{
        classes::repository as classes,
        sessions::time::{from_local, to_local},
    },
    security::{
        Actor, Permission,
        token::{random_token, sha256_hex},
    },
};

pub const MAX_GUARDIANS: i64 = 3;
pub const CONSENT_TTL_DAYS: i64 = 14;
pub const RESEND_MINUTES: i64 = 10;
/// Báo cáo tuần gửi từ giờ này tối Chủ nhật (giờ Việt Nam).
pub const WEEKLY_HOUR: u32 = 19;

// ---------- DTO ----------

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GuardianInput {
    #[validate(length(min = 1, max = 100, message = "Họ tên từ 1 đến 100 ký tự."))]
    pub full_name: String,
    #[serde(default)]
    #[validate(length(max = 50, message = "Quan hệ tối đa 50 ký tự."))]
    pub relation: String,
    #[validate(email(message = "Email không hợp lệ."))]
    pub email: String,
    #[serde(default)]
    #[validate(length(max = 20, message = "Số điện thoại tối đa 20 ký tự."))]
    pub phone: String,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GuardianUpdate {
    #[validate(length(min = 1, max = 100, message = "Họ tên từ 1 đến 100 ký tự."))]
    pub full_name: String,
    #[serde(default)]
    #[validate(length(max = 50, message = "Quan hệ tối đa 50 ký tự."))]
    pub relation: String,
    #[serde(default)]
    #[validate(length(max = 20, message = "Số điện thoại tối đa 20 ký tự."))]
    pub phone: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct GuardianView {
    pub id: Uuid,
    pub full_name: String,
    pub relation: String,
    pub email: String,
    pub phone: String,
    /// pending | confirmed | declined
    pub consent_status: String,
    pub consent_sent_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
    pub receive_reports: bool,
    pub last_report_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenQuery {
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentInput {
    pub token: String,
    pub accept: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentInfo {
    pub student_name: String,
    pub guardian_name: String,
    pub email: String,
    pub relation: String,
    pub consent_status: String,
    pub receive_reports: bool,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ManualReportInput {
    #[serde(default)]
    #[validate(length(max = 2000, message = "Nhận xét tối đa 2.000 ký tự."))]
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SentReport {
    pub sent: usize,
}

#[derive(Debug, sqlx::FromRow)]
struct GuardianRow {
    id: Uuid,
    student_id: Uuid,
    full_name: String,
    relation: String,
    email: String,
    consent_status: String,
    consent_sent_at: Option<DateTime<Utc>>,
    receive_reports: bool,
    unsubscribe_token: String,
    student_name: String,
    student_username: String,
}

const ROW_COLS: &str = "g.id, g.student_id, g.full_name, g.relation, g.email, g.consent_status, g.consent_sent_at, \
     g.receive_reports, g.unsubscribe_token, u.full_name AS student_name, u.username AS student_username";

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy phụ huynh.")
}

fn invalid_link() -> AppError {
    AppError::bad_request("INVALID_LINK", "Link không hợp lệ hoặc đã hết hạn.")
}

async fn send(ctx: &AppContext, email: Email) {
    if let Err(e) = ctx.jobs.publish(Job::SendEmail(email)).await {
        tracing::error!(error = %e, "không đẩy được email phụ huynh vào hàng đợi");
    }
}

fn unsubscribe_url(ctx: &AppContext, token: &str) -> String {
    format!("{}/phu-huynh/huy-nhan?token={token}", ctx.config.app_base_url)
}

// ---------- Học sinh khai phụ huynh ----------

pub async fn list_mine(ctx: &AppContext, actor: &Actor) -> AppResult<Vec<GuardianView>> {
    Ok(sqlx::query_as::<_, GuardianView>(
        "SELECT g.id, g.full_name, g.relation, g.email, g.phone, g.consent_status, g.consent_sent_at, g.responded_at, \
           g.receive_reports, (SELECT max(d.created_at) FROM report_deliveries d WHERE d.guardian_id = g.id) AS last_report_at \
         FROM student_guardians g WHERE g.student_id = $1 ORDER BY g.created_at",
    )
    .bind(actor.id)
    .fetch_all(&ctx.db)
    .await?)
}

async fn load_row(ctx: &AppContext, cond: &str, value: &str) -> AppResult<Option<GuardianRow>> {
    Ok(sqlx::query_as::<_, GuardianRow>(crate::sql!(
        "SELECT {ROW_COLS} FROM student_guardians g JOIN users u ON u.id = g.student_id WHERE {cond}"
    ))
    .bind(value)
    .fetch_optional(&ctx.db)
    .await?)
}

async fn own_row(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<GuardianRow> {
    load_row(ctx, "g.id = $1::uuid", &id.to_string()).await?.filter(|g| g.student_id == actor.id).ok_or_else(not_found)
}

/// Tạo link xác nhận mới và gửi email cho phụ huynh.
async fn send_consent(ctx: &AppContext, g: &GuardianRow) -> AppResult<()> {
    let token = random_token();
    sqlx::query(
        "UPDATE student_guardians SET consent_token_hash = $2, consent_sent_at = now(), updated_at = now() WHERE id = $1",
    )
    .bind(g.id)
    .bind(sha256_hex(&token))
    .execute(&ctx.db)
    .await?;
    let link = format!("{}/phu-huynh/xac-nhan?token={token}", ctx.config.app_base_url);
    let relation =
        if g.relation.trim().is_empty() { "phụ huynh".to_string() } else { g.relation.trim().to_lowercase() };
    let text = format!(
        "Kính gửi {},\n\n\
         Học sinh {} (tài khoản @{}) đã khai bạn là {relation} trên Smart Learning Space và muốn bạn nhận báo cáo học tập \
         hằng tuần qua email (tiến độ học, điểm các bài kiểm tra, điểm danh buổi học).\n\n\
         Bấm link sau để xác nhận đồng ý hoặc từ chối:\n{link}\n\n\
         Khi đồng ý, bạn cho phép Smart Learning Space gửi cho bạn thông tin học tập của học sinh. Bạn có thể hủy nhận \
         bất cứ lúc nào bằng link ở cuối mỗi báo cáo. Link có hiệu lực {CONSENT_TTL_DAYS} ngày.\n\n\
         Nếu bạn không biết học sinh này, hãy bỏ qua email.\n",
        g.full_name, g.student_name, g.student_username
    );
    send(
        ctx,
        Email {
            to: g.email.clone(),
            subject: format!("Xác nhận nhận báo cáo học tập của {}", g.student_name),
            text,
            ..Default::default()
        },
    )
    .await;
    Ok(())
}

pub async fn add(ctx: &AppContext, actor: &Actor, input: GuardianInput) -> AppResult<GuardianView> {
    input.validate()?;
    let email = input.email.trim().to_lowercase();
    let own_email: String =
        sqlx::query_scalar("SELECT lower(email) FROM users WHERE id = $1").bind(actor.id).fetch_one(&ctx.db).await?;
    if email == own_email {
        return Err(AppError::field("email", "Hãy nhập email của phụ huynh, không phải email của bạn."));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM student_guardians WHERE student_id = $1")
        .bind(actor.id)
        .fetch_one(&ctx.db)
        .await?;
    if count >= MAX_GUARDIANS {
        return Err(AppError::conflict("TOO_MANY_GUARDIANS", format!("Tối đa {MAX_GUARDIANS} phụ huynh.")));
    }
    let id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO student_guardians (student_id, full_name, relation, email, phone, unsubscribe_token) \
         VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (student_id, email) DO NOTHING RETURNING id",
    )
    .bind(actor.id)
    .bind(input.full_name.trim())
    .bind(input.relation.trim())
    .bind(&email)
    .bind(input.phone.trim())
    .bind(random_token())
    .fetch_optional(&ctx.db)
    .await?;
    let id = id.ok_or_else(|| AppError::field("email", "Email này đã có trong danh sách phụ huynh."))?;
    let row = own_row(ctx, actor, id).await?;
    send_consent(ctx, &row).await?;
    view(ctx, actor, id).await
}

async fn view(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<GuardianView> {
    list_mine(ctx, actor).await?.into_iter().find(|g| g.id == id).ok_or_else(not_found)
}

pub async fn update(ctx: &AppContext, actor: &Actor, id: Uuid, input: GuardianUpdate) -> AppResult<GuardianView> {
    input.validate()?;
    own_row(ctx, actor, id).await?;
    sqlx::query(
        "UPDATE student_guardians SET full_name = $2, relation = $3, phone = $4, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(input.full_name.trim())
    .bind(input.relation.trim())
    .bind(input.phone.trim())
    .execute(&ctx.db)
    .await?;
    view(ctx, actor, id).await
}

pub async fn remove(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<()> {
    own_row(ctx, actor, id).await?;
    sqlx::query("DELETE FROM student_guardians WHERE id = $1").bind(id).execute(&ctx.db).await?;
    Ok(())
}

/// Gửi lại email xác nhận (chưa xác nhận / đã từ chối), tối đa một lần mỗi 10 phút.
pub async fn resend(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<GuardianView> {
    let row = own_row(ctx, actor, id).await?;
    if row.consent_status == "confirmed" {
        return Err(AppError::conflict("ALREADY_CONFIRMED", "Phụ huynh đã xác nhận."));
    }
    if row.consent_sent_at.is_some_and(|t| Utc::now() - t < TimeDelta::minutes(RESEND_MINUTES)) {
        return Err(AppError::conflict(
            "RESEND_TOO_SOON",
            format!("Vừa gửi email xác nhận. Hãy thử lại sau {RESEND_MINUTES} phút."),
        ));
    }
    sqlx::query("UPDATE student_guardians SET consent_status = 'pending' WHERE id = $1")
        .bind(id)
        .execute(&ctx.db)
        .await?;
    send_consent(ctx, &row).await?;
    view(ctx, actor, id).await
}

// ---------- Phụ huynh bấm link (không cần đăng nhập) ----------

async fn by_consent_token(ctx: &AppContext, token: &str) -> AppResult<GuardianRow> {
    let row = load_row(ctx, "g.consent_token_hash = $1", &sha256_hex(token.trim())).await?.ok_or_else(invalid_link)?;
    if row.consent_sent_at.is_none_or(|t| Utc::now() - t > TimeDelta::days(CONSENT_TTL_DAYS)) {
        return Err(invalid_link());
    }
    Ok(row)
}

fn consent_info(g: &GuardianRow, status: &str, receive_reports: bool) -> ConsentInfo {
    ConsentInfo {
        student_name: g.student_name.clone(),
        guardian_name: g.full_name.clone(),
        email: g.email.clone(),
        relation: g.relation.clone(),
        consent_status: status.to_string(),
        receive_reports,
    }
}

pub async fn consent_status(ctx: &AppContext, q: TokenQuery) -> AppResult<ConsentInfo> {
    let g = by_consent_token(ctx, &q.token).await?;
    Ok(consent_info(&g, &g.consent_status, g.receive_reports))
}

pub async fn respond(ctx: &AppContext, input: ConsentInput) -> AppResult<ConsentInfo> {
    let g = by_consent_token(ctx, &input.token).await?;
    let status = if input.accept { "confirmed" } else { "declined" };
    sqlx::query(
        "UPDATE student_guardians SET consent_status = $2, responded_at = now(), receive_reports = $3, updated_at = now() \
         WHERE id = $1",
    )
    .bind(g.id)
    .bind(status)
    .bind(input.accept)
    .execute(&ctx.db)
    .await?;
    Ok(consent_info(&g, status, input.accept))
}

/// Hủy nhận (`subscribe = false`) hoặc nhận lại (`true`, chỉ khi đã xác nhận) bằng link ở cuối báo cáo.
pub async fn set_subscription(ctx: &AppContext, q: TokenQuery, subscribe: bool) -> AppResult<ConsentInfo> {
    let g = load_row(ctx, "g.unsubscribe_token = $1", q.token.trim()).await?.ok_or_else(invalid_link)?;
    if subscribe && g.consent_status != "confirmed" {
        return Err(AppError::conflict("NOT_CONFIRMED", "Phụ huynh chưa xác nhận nhận báo cáo."));
    }
    sqlx::query("UPDATE student_guardians SET receive_reports = $2, updated_at = now() WHERE id = $1")
        .bind(g.id)
        .bind(subscribe)
        .execute(&ctx.db)
        .await?;
    Ok(consent_info(&g, &g.consent_status, subscribe))
}

pub async fn subscription_info(ctx: &AppContext, q: TokenQuery) -> AppResult<ConsentInfo> {
    let g = load_row(ctx, "g.unsubscribe_token = $1", q.token.trim()).await?.ok_or_else(invalid_link)?;
    Ok(consent_info(&g, &g.consent_status, g.receive_reports))
}

// ---------- Gửi báo cáo ----------

async fn deliver(
    ctx: &AppContext,
    g: &GuardianRow,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    only_class: Option<Uuid>,
    note: Option<&str>,
) -> AppResult<()> {
    let r = report::collect(ctx, g.student_id, &g.student_name, from, to, only_class).await?;
    let text = report::render_text(
        &r,
        &g.full_name,
        note,
        &ctx.config.app_base_url,
        &unsubscribe_url(ctx, &g.unsubscribe_token),
    );
    let subject = match note {
        Some(_) => format!("Giáo viên gửi báo cáo học tập của {}", g.student_name),
        None => format!("Báo cáo học tập tuần của {} ({})", g.student_name, to_local(from).format("%d/%m")),
    };
    send(ctx, Email { to: g.email.clone(), subject, text, ..Default::default() }).await;
    Ok(())
}

/// Kỳ báo cáo tuần chứa `now` nếu đã tới giờ gửi (tối Chủ nhật ≥ 19h giờ VN): (0h thứ Hai, ngày thứ Hai, ngày Chủ nhật).
pub fn weekly_period(now: DateTime<Utc>) -> Option<(DateTime<Utc>, chrono::NaiveDate, chrono::NaiveDate)> {
    let local = to_local(now);
    if local.weekday() != Weekday::Sun || local.time() < NaiveTime::from_hms_opt(WEEKLY_HOUR, 0, 0)? {
        return None;
    }
    let sunday = local.date();
    let monday = sunday - Duration::days(6);
    Some((from_local(monday.and_time(NaiveTime::MIN)), monday, sunday))
}

/// Gửi báo cáo tuần cho mọi phụ huynh chưa nhận báo cáo của tuần này. Trả số email đã gửi.
pub async fn send_weekly_reports(ctx: &AppContext, now: DateTime<Utc>) -> AppResult<usize> {
    let Some((from, monday, sunday)) = weekly_period(now) else { return Ok(0) };
    // "Nhận" lượt gửi trước khi gửi: chỉ instance chèn được dòng mới mới gửi email.
    let claimed: Vec<Uuid> = sqlx::query_scalar(
        "INSERT INTO report_deliveries (guardian_id, student_id, kind, period_start, period_end) \
         SELECT g.id, g.student_id, 'weekly', $1, $2 FROM student_guardians g JOIN users u ON u.id = g.student_id \
         WHERE g.consent_status = 'confirmed' AND g.receive_reports AND u.status = 'active' \
         ON CONFLICT (guardian_id, period_start) WHERE kind = 'weekly' DO NOTHING RETURNING guardian_id",
    )
    .bind(monday)
    .bind(sunday)
    .fetch_all(&ctx.db)
    .await?;
    for id in &claimed {
        if let Some(g) = load_row(ctx, "g.id = $1::uuid", &id.to_string()).await? {
            deliver(ctx, &g, from, now, None, None).await?;
        }
    }
    Ok(claimed.len())
}

/// Gọi mỗi phút từ `api`: tới giờ thì gửi báo cáo tuần. `done` = tuần (thứ Hai) đã gửi xong trong instance này,
/// để khỏi quét lại cả buổi tối Chủ nhật.
pub async fn weekly_tick(ctx: &AppContext, done: &mut Option<chrono::NaiveDate>) -> AppResult<usize> {
    let now = Utc::now();
    let Some((_, monday, _)) = weekly_period(now) else { return Ok(0) };
    if *done == Some(monday) {
        return Ok(0);
    }
    let n = send_weekly_reports(ctx, now).await?;
    *done = Some(monday);
    Ok(n)
}

/// Giáo viên gửi ngay báo cáo của một học sinh trong lớp học (7 ngày gần nhất) kèm nhận xét.
pub async fn send_manual(
    ctx: &AppContext,
    actor: &Actor,
    class_id: Uuid,
    student_id: Uuid,
    input: ManualReportInput,
) -> AppResult<SentReport> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    let class =
        classes::find(&ctx.db, class_id).await?.ok_or_else(|| AppError::not_found("Không tìm thấy lớp học."))?;
    if class.owner_id != actor.id {
        return Err(AppError::not_found("Không tìm thấy lớp học."));
    }
    if !classes::membership(&ctx.db, class_id, student_id).await?.is_some_and(|m| m.status == "active") {
        return Err(AppError::not_found("Học sinh không học lớp học này."));
    }
    let rows = sqlx::query_as::<_, GuardianRow>(crate::sql!(
        "SELECT {ROW_COLS} FROM student_guardians g JOIN users u ON u.id = g.student_id \
         WHERE g.student_id = $1 AND g.consent_status = 'confirmed' AND g.receive_reports"
    ))
    .bind(student_id)
    .fetch_all(&ctx.db)
    .await?;
    if rows.is_empty() {
        return Err(AppError::conflict("NO_GUARDIAN", "Học sinh chưa có phụ huynh xác nhận nhận báo cáo."));
    }
    let now = Utc::now();
    let from = now - TimeDelta::days(7);
    for g in &rows {
        sqlx::query(
            "INSERT INTO report_deliveries (guardian_id, student_id, kind, period_start, period_end, class_id, sent_by, note) \
             VALUES ($1, $2, 'manual', $3, $4, $5, $6, $7)",
        )
        .bind(g.id)
        .bind(student_id)
        .bind(to_local(from).date())
        .bind(to_local(now).date())
        .bind(class_id)
        .bind(actor.id)
        .bind(input.note.trim())
        .execute(&ctx.db)
        .await?;
        deliver(ctx, g, from, now, Some(class_id), Some(input.note.trim())).await?;
    }
    Ok(SentReport { sent: rows.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn ky_bao_cao_tuan() {
        // Chủ nhật 15/11/2026 19:30 giờ VN = 12:30 UTC
        let (from, monday, sunday) = weekly_period(t("2026-11-15T12:30:00Z")).unwrap();
        assert_eq!(from, t("2026-11-08T17:00:00Z")); // 0h thứ Hai 9/11 giờ VN
        assert_eq!((monday.to_string(), sunday.to_string()), ("2026-11-09".into(), "2026-11-15".into()));
        // Chủ nhật 18:59 giờ VN: chưa tới giờ; thứ Hai: không gửi
        assert!(weekly_period(t("2026-11-15T11:59:00Z")).is_none());
        assert!(weekly_period(t("2026-11-16T12:30:00Z")).is_none());
        // 23:59 Chủ nhật giờ VN vẫn trong kỳ
        assert!(weekly_period(t("2026-11-15T16:59:00Z")).is_some());
    }
}
