//! Nghiệp vụ lịch & buổi học. Xem mô tả ở `mod.rs`.

use chrono::{DateTime, TimeDelta, Utc};
use uuid::Uuid;
use validator::Validate;

use super::{
    dto::{
        AttendanceInput, AttendeeView, CancelInput, CreateSessionInput, CreateSessionResponse, EditScope,
        InviteesInput, InviteesView, JoinResponse, NamedRef, RangeQuery, Room, RsvpInput, SessionDetail, SessionFields,
        SessionSummary, UpdateSessionInput, UpdateSessionResponse,
    },
    ics::{self, IcsEvent, Method},
    model::{Access, Recipient, SessionRef, SessionRow},
    repository::{self as repo, NewSession},
    time,
};
use crate::{
    context::AppContext,
    error::{AppError, AppResult},
    jobs::{Attachment, Email, Job},
    modules::{
        classes::service as classes,
        groups,
        lessons::service as lessons,
        notifications::{self, NotificationKind},
    },
    security::{Actor, Permission},
};

/// Nút "Vào học" mở trước giờ học bao lâu.
const JOIN_EARLY: TimeDelta = TimeDelta::minutes(10);
/// Khoảng tối đa của một lần xem lịch.
const MAX_RANGE: TimeDelta = TimeDelta::days(93);

fn not_found() -> AppError {
    AppError::not_found("Không tìm thấy buổi học.")
}

fn session_link(ctx: &AppContext, id: Uuid) -> String {
    format!("{}/buoi-hoc/{id}", ctx.config.app_base_url)
}

async fn load_own(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<SessionRow> {
    actor.require(Permission::TeachingManage)?;
    let s = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if s.owner_id != actor.id {
        return Err(not_found());
    }
    Ok(s)
}

fn targets(inv: &InviteesInput) -> Vec<(&'static str, Uuid)> {
    let mut out: Vec<(&'static str, Uuid)> = Vec::new();
    out.extend(inv.class_ids.iter().map(|id| ("class", *id)));
    out.extend(inv.group_ids.iter().map(|id| ("group", *id)));
    out.extend(inv.user_ids.iter().map(|id| ("user", *id)));
    out
}

async fn ensure_own_classes(ctx: &AppContext, owner: Uuid, ids: &[Uuid], field: &str) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let mut unique = ids.to_vec();
    unique.sort();
    unique.dedup();
    let count = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM classes WHERE owner_id = $1 AND id = ANY($2)")
        .bind(owner)
        .bind(&unique)
        .fetch_one(&ctx.db)
        .await?;
    if count != unique.len() as i64 {
        return Err(AppError::field(field, "Lớp học không tồn tại."));
    }
    Ok(())
}

async fn validate_fields(ctx: &AppContext, actor: &Actor, f: &SessionFields) -> AppResult<()> {
    f.validate()?;
    if let Some(class_id) = f.class_id {
        ensure_own_classes(ctx, actor.id, &[class_id], "classId").await?;
    }
    if f.access == Access::Members && f.class_id.is_none() {
        return Err(AppError::field("access", "Chọn lớp học của buổi để cho thành viên lớp học vào."));
    }
    if f.room == Room::Live && ctx.config.live.is_none() {
        return Err(AppError::field("room", "Chưa cấu hình phòng học trực tuyến (LiveKit). Hãy dùng link họp ngoài."));
    }
    if f.access == Access::PublicLink && f.room != Room::Live {
        return Err(AppError::field("access", "Buổi mở cho khách chỉ dùng được với phòng học của hệ thống."));
    }
    if f.access == Access::Invited && f.invitees.is_empty() {
        return Err(AppError::field("invitees", "Mời ít nhất một lớp học, nhóm hoặc học sinh."));
    }
    ensure_own_classes(ctx, actor.id, &f.invitees.class_ids, "invitees").await?;
    groups::ensure_own_groups(ctx, actor.id, &f.invitees.group_ids).await?;
    groups::ensure_own_students(ctx, actor.id, &f.invitees.user_ids)
        .await
        .map_err(|_| AppError::field("invitees", "Chỉ mời được học sinh đang học các lớp học của bạn."))?;
    let refs = lessons::refs(ctx, &f.lesson_ids).await?;
    if refs.len() != f.lesson_ids.len() || refs.iter().any(|l| l.owner_id != actor.id || l.archived) {
        return Err(AppError::field("lessonIds", "Có bài không tồn tại hoặc đã bị ẩn."));
    }
    Ok(())
}

fn new_session<'a>(owner: Uuid, schedule: Option<Uuid>, f: &'a SessionFields, start: DateTime<Utc>) -> NewSession<'a> {
    NewSession {
        owner_id: owner,
        schedule_id: schedule,
        class_id: f.class_id,
        title: f.title.trim(),
        description: f.description.trim(),
        starts_at: start,
        ends_at: start + TimeDelta::minutes(f.duration_minutes),
        access: f.access.as_str(),
        meeting_url: f.meeting_url.as_deref().map(str::trim).filter(|u| !u.is_empty()),
        room: f.room.as_str(),
        remind_minutes: f.remind_minutes,
    }
}

fn conflict_error(title: &str, at: DateTime<Utc>) -> AppError {
    AppError::conflict(
        "SESSION_CONFLICT",
        format!("Trùng lịch với buổi \"{title}\" lúc {} của bạn.", time::format_vn(at)),
    )
}

// ---------- Email + thông báo ----------

#[derive(Clone, Copy)]
enum Mail {
    Invite,
    Updated,
    Cancelled,
    Reminder,
}

fn times_text(sessions: &[SessionRow]) -> String {
    let mut lines: Vec<String> = sessions
        .iter()
        .take(12)
        .map(|s| format!("- {} ({} phút)", time::format_vn(s.starts_at), (s.ends_at - s.starts_at).num_minutes()))
        .collect();
    if sessions.len() > 12 {
        lines.push(format!("- … và {} buổi nữa", sessions.len() - 12));
    }
    lines.join("\n")
}

/// Gửi thông báo trong app + email (kèm `.ics` trừ khi nhắc lịch) cho người được mời.
async fn announce(ctx: &AppContext, sessions: &[SessionRow], recipients: &[Recipient], mail: Mail) -> AppResult<()> {
    let Some(first) = sessions.first() else { return Ok(()) };
    let n = sessions.len();
    let when = if n > 1 {
        format!("{} buổi, buổi đầu {}", n, time::format_vn(first.starts_at))
    } else {
        time::format_vn(first.starts_at)
    };
    let (kind, title, subject) = match mail {
        Mail::Invite => {
            (NotificationKind::SessionInvite, format!("Lời mời buổi học: {}", first.title), "Lời mời buổi học")
        }
        Mail::Updated => {
            (NotificationKind::SessionUpdated, format!("Buổi học đã thay đổi: {}", first.title), "Buổi học đã thay đổi")
        }
        Mail::Cancelled => {
            (NotificationKind::SessionCancelled, format!("Buổi học đã hủy: {}", first.title), "Buổi học đã hủy")
        }
        Mail::Reminder => {
            (NotificationKind::SessionReminder, format!("Sắp đến giờ học: {}", first.title), "Nhắc lịch học")
        }
    };
    let link = format!("/buoi-hoc/{}", first.id);
    let ids: Vec<Uuid> = recipients.iter().map(|r| r.id).collect();
    notifications::notify(&ctx.db, &ids, kind, &title, &when, Some(&link)).await?;

    let method = if matches!(mail, Mail::Cancelled) { Method::Cancel } else { Method::Request };
    let attachment = (!matches!(mail, Mail::Reminder)).then(|| {
        let events: Vec<IcsEvent> = sessions
            .iter()
            .map(|s| IcsEvent {
                uid: format!("{}@{}", s.id, ics::UID_DOMAIN),
                sequence: s.ics_sequence,
                start: s.starts_at,
                end: s.ends_at,
                summary: s.title.clone(),
                description: format!("{}\n\nChi tiết: {}", s.description, session_link(ctx, s.id)).trim().to_string(),
                url: s.meeting_url.clone(),
                organizer: s.owner_name.clone(),
            })
            .collect();
        let method_name = if method == Method::Cancel { "CANCEL" } else { "REQUEST" };
        Attachment {
            filename: "buoi-hoc.ics".into(),
            content_type: format!("text/calendar; charset=utf-8; method={method_name}"),
            content: ics::calendar(method, &events, Utc::now()),
        }
    });
    let class = first.class_title.as_deref().map(|c| format!(" (lớp học {c})")).unwrap_or_default();
    let verb = match mail {
        Mail::Invite => format!("{} mời bạn tham gia buổi học \"{}\"{class}.", first.owner_name, first.title),
        Mail::Updated => format!("Buổi học \"{}\"{class} vừa được thay đổi. Lịch mới:", first.title),
        Mail::Cancelled => format!("Buổi học \"{}\"{class} đã bị hủy.", first.title),
        Mail::Reminder => format!("Buổi học \"{}\"{class} sắp bắt đầu.", first.title),
    };
    let join = first
        .meeting_url
        .as_deref()
        .filter(|_| !matches!(mail, Mail::Cancelled))
        .map(|u| format!("\nVào học: {u} (mở từ 10 phút trước giờ học)\n"))
        .unwrap_or_default();
    for r in recipients {
        let text = format!(
            "Chào {},\n\n{verb}\n\nThời gian:\n{}\n{join}\nXem chi tiết{}: {}\n\nSmart Learning Space",
            r.full_name,
            times_text(sessions),
            if matches!(mail, Mail::Invite) { " và trả lời tham gia" } else { "" },
            session_link(ctx, first.id),
        );
        let email = Email {
            to: r.email.clone(),
            subject: format!("{subject}: {} — {}", first.title, time::format_vn(first.starts_at)),
            text,
            attachments: attachment.clone().into_iter().collect(),
        };
        if let Err(e) = ctx.jobs.publish(Job::SendEmail(email)).await {
            tracing::error!(error = %e, "không đẩy được email buổi học vào hàng đợi");
        }
    }
    Ok(())
}

// ---------- Giáo viên ----------

pub async fn create(ctx: &AppContext, actor: &Actor, input: CreateSessionInput) -> AppResult<CreateSessionResponse> {
    actor.require(Permission::TeachingManage)?;
    input.validate()?;
    let f = &input.fields;
    validate_fields(ctx, actor, f).await?;
    let starts = match &input.repeat {
        Some(r) => {
            if r.until < time::to_local(f.starts_at).date() {
                return Err(AppError::field("repeat", "Ngày kết thúc phải sau ngày bắt đầu."));
            }
            let list = time::weekly(f.starts_at, &r.weekdays, r.until).map_err(|_| {
                AppError::field(
                    "repeat",
                    format!("Lịch lặp tối đa {} buổi. Hãy chọn ngày kết thúc sớm hơn.", time::MAX_OCCURRENCES),
                )
            })?;
            if list.is_empty() {
                return Err(AppError::field("repeat", "Không có buổi nào trong khoảng ngày đã chọn."));
            }
            list
        }
        None => vec![f.starts_at],
    };
    let ends: Vec<DateTime<Utc>> = starts.iter().map(|s| *s + TimeDelta::minutes(f.duration_minutes)).collect();
    if let Some((title, at)) = repo::owner_conflict(&ctx.db, actor.id, &starts, &ends, &[]).await? {
        return Err(conflict_error(&title, at));
    }

    let targets = targets(&f.invitees);
    let mut tx = ctx.db.begin().await?;
    let schedule = match &input.repeat {
        Some(r) => {
            let days: Vec<i32> = r.weekdays.iter().map(|d| *d as i32).collect();
            Some(repo::insert_schedule(&mut tx, actor.id, &days, time::to_local(f.starts_at).date(), r.until).await?)
        }
        None => None,
    };
    let mut ids = Vec::with_capacity(starts.len());
    for start in &starts {
        let id = repo::insert(&mut tx, &new_session(actor.id, schedule, f, *start)).await?;
        repo::set_invitees(&mut tx, id, &targets).await?;
        repo::set_lessons(&mut tx, id, &f.lesson_ids).await?;
        ids.push(id);
    }
    tx.commit().await?;

    let rows = repo::find_many(&ctx.db, &ids).await?;
    let recipients = repo::recipients(&ctx.db, &ids).await?;
    let user_ids: Vec<Uuid> = recipients.iter().map(|r| r.id).collect();
    let student_conflicts = repo::student_conflicts(&ctx.db, &user_ids, &starts, &ends, &ids).await?;
    announce(ctx, &rows, &recipients, Mail::Invite).await?;
    Ok(CreateSessionResponse {
        sessions: rows.iter().map(|s| SessionSummary::new(s, actor.id, None)).collect(),
        student_conflicts,
    })
}

fn range(q: &RangeQuery) -> AppResult<(DateTime<Utc>, DateTime<Utc>)> {
    let from = q.from.unwrap_or_else(|| Utc::now() - TimeDelta::days(7));
    let to = q.to.unwrap_or(from + TimeDelta::days(37));
    if to <= from || to - from > MAX_RANGE {
        return Err(AppError::bad_request("INVALID_RANGE", "Khoảng thời gian không hợp lệ (tối đa 93 ngày)."));
    }
    Ok((from, to))
}

pub async fn list_owned(ctx: &AppContext, actor: &Actor, q: RangeQuery) -> AppResult<Vec<SessionSummary>> {
    actor.require(Permission::TeachingManage)?;
    let (from, to) = range(&q)?;
    Ok(repo::list_owned(&ctx.db, actor.id, from, to, q.class_id)
        .await?
        .iter()
        .map(|s| SessionSummary::new(s, actor.id, None))
        .collect())
}

/// Sửa buổi: chỉ buổi này, hoặc buổi này và các buổi sau của cùng lịch lặp (dời cùng một khoảng thời gian).
pub async fn update(
    ctx: &AppContext,
    actor: &Actor,
    id: Uuid,
    input: UpdateSessionInput,
) -> AppResult<UpdateSessionResponse> {
    let current = load_own(ctx, actor, id).await?;
    if current.status == "cancelled" {
        return Err(AppError::conflict("SESSION_CANCELLED", "Buổi học đã hủy, không sửa được."));
    }
    input.validate()?;
    let f = &input.fields;
    validate_fields(ctx, actor, f).await?;
    let affected = match (input.scope, current.schedule_id) {
        (EditScope::Following, Some(schedule)) => repo::following(&ctx.db, schedule, current.starts_at).await?,
        _ => vec![current.clone()],
    };
    let delta = f.starts_at - current.starts_at;
    let ids: Vec<Uuid> = affected.iter().map(|s| s.id).collect();
    let starts: Vec<DateTime<Utc>> = affected.iter().map(|s| s.starts_at + delta).collect();
    let ends: Vec<DateTime<Utc>> = starts.iter().map(|s| *s + TimeDelta::minutes(f.duration_minutes)).collect();
    if let Some((title, at)) = repo::owner_conflict(&ctx.db, actor.id, &starts, &ends, &ids).await? {
        return Err(conflict_error(&title, at));
    }

    let targets = targets(&f.invitees);
    let mut tx = ctx.db.begin().await?;
    for (s, start) in affected.iter().zip(&starts) {
        repo::update(&mut tx, s.id, &new_session(actor.id, s.schedule_id, f, *start)).await?;
        repo::set_invitees(&mut tx, s.id, &targets).await?;
        repo::set_lessons(&mut tx, s.id, &f.lesson_ids).await?;
    }
    tx.commit().await?;

    let rows = repo::find_many(&ctx.db, &ids).await?;
    let recipients = repo::recipients(&ctx.db, &ids).await?;
    let user_ids: Vec<Uuid> = recipients.iter().map(|r| r.id).collect();
    let student_conflicts = repo::student_conflicts(&ctx.db, &user_ids, &starts, &ends, &ids).await?;
    announce(ctx, &rows, &recipients, Mail::Updated).await?;
    Ok(UpdateSessionResponse { updated: rows.len(), student_conflicts })
}

pub async fn cancel(ctx: &AppContext, actor: &Actor, id: Uuid, input: CancelInput) -> AppResult<UpdateSessionResponse> {
    let current = load_own(ctx, actor, id).await?;
    if current.status == "cancelled" {
        return Err(AppError::conflict("SESSION_CANCELLED", "Buổi học đã hủy."));
    }
    let affected = match (input.scope, current.schedule_id) {
        (EditScope::Following, Some(schedule)) => repo::following(&ctx.db, schedule, current.starts_at).await?,
        _ => vec![current],
    };
    let ids: Vec<Uuid> = affected.iter().map(|s| s.id).collect();
    let recipients = repo::recipients(&ctx.db, &ids).await?;
    repo::cancel(&ctx.db, &ids).await?;
    let rows = repo::find_many(&ctx.db, &ids).await?;
    announce(ctx, &rows, &recipients, Mail::Cancelled).await?;
    Ok(UpdateSessionResponse { updated: rows.len(), student_conflicts: 0 })
}

pub async fn set_attendance(ctx: &AppContext, actor: &Actor, id: Uuid, input: AttendanceInput) -> AppResult<()> {
    load_own(ctx, actor, id).await?;
    if !repo::is_invited(&ctx.db, id, input.user_id).await? {
        return Err(AppError::field("userId", "Học sinh không được mời vào buổi này."));
    }
    repo::set_attended(&ctx.db, id, input.user_id, input.attended).await?;
    Ok(())
}

// ---------- Học sinh ----------

pub async fn list_invited(ctx: &AppContext, actor: &Actor, q: RangeQuery) -> AppResult<Vec<SessionSummary>> {
    let (from, to) = range(&q)?;
    Ok(repo::list_invited(&ctx.db, actor.id, from, to, q.class_id)
        .await?
        .iter()
        .map(|(s, rsvp)| SessionSummary::new(s, actor.id, rsvp.clone()))
        .collect())
}

/// Người xem được vào buổi (xem chi tiết, bấm "Vào học").
async fn can_access(ctx: &AppContext, actor: &Actor, s: &SessionRow) -> AppResult<(bool, bool)> {
    let invited = repo::is_invited(&ctx.db, s.id, actor.id).await?;
    let allowed = invited
        || s.owner_id == actor.id
        || match s.access.as_str() {
            "logged_in" | "public_link" => true,
            "members" => match s.class_id {
                Some(class) => repo::is_class_member(&ctx.db, class, actor.id).await?,
                None => false,
            },
            _ => false,
        };
    Ok((allowed, invited))
}

pub async fn detail(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<SessionDetail> {
    let s = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    let (allowed, invited) = can_access(ctx, actor, &s).await?;
    if !allowed {
        return Err(not_found());
    }
    let is_owner = s.owner_id == actor.id;
    let attendees = repo::attendees(&ctx.db, id).await?;
    let mine = attendees.iter().find(|a| a.user_id == actor.id);
    let lessons = repo::lessons(&ctx.db, id).await?.into_iter().map(|(id, name)| NamedRef { id, name }).collect();
    let (invitees, attendee_views) = if is_owner {
        let mut view = InviteesView::default();
        for i in repo::invitees(&ctx.db, id).await? {
            let r = NamedRef { id: i.target_id, name: i.name };
            match i.target_type.as_str() {
                "class" => view.classes.push(r),
                "group" => view.groups.push(r),
                _ => view.users.push(r),
            }
        }
        let list = repo::recipients(&ctx.db, &[id])
            .await?
            .into_iter()
            .map(|r| {
                let a = attendees.iter().find(|a| a.user_id == r.id);
                AttendeeView {
                    user_id: r.id,
                    username: r.username,
                    full_name: r.full_name,
                    rsvp: a.and_then(|a| a.rsvp.clone()),
                    joined_at: a.and_then(|a| a.joined_at),
                    attended: a.is_some_and(|a| a.attended),
                }
            })
            .collect();
        (Some(view), Some(list))
    } else {
        (None, None)
    };
    let now = Utc::now();
    let join_opens_at = s.starts_at - JOIN_EARLY;
    let can_join = s.status == "scheduled" && now >= join_opens_at && now <= s.ends_at;
    Ok(SessionDetail {
        summary: SessionSummary::new(&s, actor.id, mine.and_then(|a| a.rsvp.clone())),
        description: s.description.clone(),
        duration_minutes: (s.ends_at - s.starts_at).num_minutes(),
        remind_minutes: s.remind_minutes,
        meeting_url: s.meeting_url.clone(),
        lessons,
        invitees,
        attendees: attendee_views,
        join_opens_at,
        can_join,
        attended: mine.is_some_and(|a| a.attended),
        invited,
    })
}

pub async fn rsvp(ctx: &AppContext, actor: &Actor, id: Uuid, input: RsvpInput) -> AppResult<()> {
    let s = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if !repo::is_invited(&ctx.db, id, actor.id).await? {
        return Err(not_found());
    }
    if s.status == "cancelled" || s.ends_at < Utc::now() {
        return Err(AppError::conflict("SESSION_CLOSED", "Buổi học đã kết thúc hoặc đã hủy."));
    }
    let value = match input.response {
        super::dto::Rsvp::Yes => "yes",
        super::dto::Rsvp::No => "no",
    };
    repo::set_rsvp(&ctx.db, id, actor.id, value).await?;
    Ok(())
}

/// "Vào học": mở từ 10 phút trước giờ học tới hết giờ; ghi điểm danh cho học sinh.
pub async fn join(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<JoinResponse> {
    let s = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    let (allowed, _) = can_access(ctx, actor, &s).await?;
    if !allowed {
        return Err(not_found());
    }
    if s.status == "cancelled" {
        return Err(AppError::conflict("SESSION_CANCELLED", "Buổi học đã hủy."));
    }
    let now = Utc::now();
    if now < s.starts_at - JOIN_EARLY {
        return Err(AppError::conflict(
            "SESSION_NOT_OPEN",
            format!("Chưa tới giờ vào học. Phòng mở lúc {}.", time::format_vn(s.starts_at - JOIN_EARLY)),
        ));
    }
    if now > s.ends_at {
        return Err(AppError::conflict("SESSION_ENDED", "Buổi học đã kết thúc."));
    }
    if s.owner_id != actor.id {
        repo::mark_joined(&ctx.db, id, actor.id).await?;
    }
    Ok(JoinResponse { meeting_url: s.meeting_url, room: s.room })
}

// ---------- Nhắc lịch (chạy nền) ----------

/// Gửi nhắc lịch cho các buổi đến giờ nhắc. Trả về số buổi đã nhắc.
pub async fn send_due_reminders(ctx: &AppContext) -> AppResult<usize> {
    let ids = repo::claim_due_reminders(&ctx.db).await?;
    for id in &ids {
        let rows = repo::find_many(&ctx.db, &[*id]).await?;
        let recipients = repo::recipients(&ctx.db, &[*id]).await?;
        announce(ctx, &rows, &recipients, Mail::Reminder).await?;
    }
    Ok(ids.len())
}

// ---------- Dùng cho lộ trình ----------

pub async fn refs(ctx: &AppContext, ids: &[Uuid]) -> AppResult<Vec<SessionRef>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(repo::refs(&ctx.db, ids).await?)
}

/// (học sinh, buổi) đã điểm danh.
pub async fn attended_pairs(
    ctx: &AppContext,
    session_ids: &[Uuid],
    user: Option<Uuid>,
) -> AppResult<Vec<(Uuid, Uuid)>> {
    if session_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(repo::attended_pairs(&ctx.db, session_ids, user).await?)
}

/// Số buổi được mời / có mặt của từng học sinh (báo cáo lớp học, báo cáo phụ huynh).
pub async fn attendance_stats(
    ctx: &AppContext,
    class_id: Option<Uuid>,
    user_ids: &[Uuid],
    from: Option<DateTime<Utc>>,
    to: DateTime<Utc>,
) -> AppResult<Vec<(Uuid, i64, i64)>> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(repo::attendance_stats(&ctx.db, class_id, user_ids, from, to).await?)
}

// ---------- Học kèm 1-1 (M23) ----------

/// Buổi học kèm 1-1 cần tạo (xem `create_booked`).
pub struct BookedSession<'a> {
    pub teacher_id: Uuid,
    pub student_id: Uuid,
    pub title: &'a str,
    pub description: &'a str,
    pub starts_at: DateTime<Utc>,
    pub duration_minutes: i64,
    pub meeting_url: Option<String>,
}

/// Tạo buổi học kèm cho học sinh đã đặt lịch (giáo viên vừa đồng ý). Khác `create`: học sinh không cần đang học
/// lớp học của giáo viên (khung giờ có thể mở cho mọi tài khoản). Vẫn chặn trùng lịch giáo viên, gửi lời mời
/// kèm `.ics` như buổi thường.
pub async fn create_booked(ctx: &AppContext, b: BookedSession<'_>) -> AppResult<Uuid> {
    let BookedSession { teacher_id, student_id, title, description, starts_at, duration_minutes, meeting_url } = b;
    let fields = SessionFields {
        title: title.chars().take(200).collect(),
        description: description.chars().take(5000).collect(),
        class_id: None,
        starts_at,
        duration_minutes,
        invitees: InviteesInput { user_ids: vec![student_id], ..Default::default() },
        access: Access::Invited,
        // Không có link họp + có phòng của hệ thống → học kèm trong phòng của hệ thống.
        room: if meeting_url.is_none() && ctx.config.live.is_some() { Room::Live } else { Room::External },
        meeting_url,
        remind_minutes: 60,
        lesson_ids: Vec::new(),
    };
    fields.validate()?;
    let ends = starts_at + TimeDelta::minutes(duration_minutes);
    if let Some((title, at)) = repo::owner_conflict(&ctx.db, teacher_id, &[starts_at], &[ends], &[]).await? {
        return Err(conflict_error(&title, at));
    }
    let targets = targets(&fields.invitees);
    let mut tx = ctx.db.begin().await?;
    let id = repo::insert(&mut tx, &new_session(teacher_id, None, &fields, starts_at)).await?;
    repo::set_invitees(&mut tx, id, &targets).await?;
    tx.commit().await?;
    let rows = repo::find_many(&ctx.db, &[id]).await?;
    let recipients = repo::recipients(&ctx.db, &[id]).await?;
    announce(ctx, &rows, &recipients, Mail::Invite).await?;
    Ok(id)
}

/// Hủy một buổi (học sinh hủy lịch học kèm đã được duyệt). Buổi đã hủy thì bỏ qua.
pub async fn cancel_by_id(ctx: &AppContext, id: Uuid) -> AppResult<()> {
    let rows = repo::find_many(&ctx.db, &[id]).await?;
    if rows.iter().all(|s| s.status == "cancelled") {
        return Ok(());
    }
    let recipients = repo::recipients(&ctx.db, &[id]).await?;
    repo::cancel(&ctx.db, &[id]).await?;
    let rows = repo::find_many(&ctx.db, &[id]).await?;
    announce(ctx, &rows, &recipients, Mail::Cancelled).await
}

/// Khoảng giờ giáo viên đã có buổi (chưa hủy) — để tính khung giờ còn trống cho học kèm.
pub async fn busy(
    ctx: &AppContext,
    owner: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
    Ok(repo::list_owned(&ctx.db, owner, from, to, None)
        .await?
        .into_iter()
        .filter(|s| s.status != "cancelled")
        .map(|s| (s.starts_at, s.ends_at))
        .collect())
}

// ---------- Phòng học trực tuyến (đợt 6) ----------

/// Giáo viên được mở phòng sớm bấy nhiêu trước giờ học; ai cũng ở lại được bấy nhiêu sau giờ kết thúc.
const LIVE_TEACHER_EARLY: TimeDelta = TimeDelta::minutes(60);
const LIVE_OVERTIME: TimeDelta = TimeDelta::minutes(30);

/// Vào phòng học trực tuyến của buổi: kiểm tra quyền + giờ như "Vào học", ghi điểm danh học sinh.
/// Trả về buổi học và người vào có phải giáo viên chủ buổi không.
/// Vai trong phòng học trực tuyến.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveRole {
    /// Giáo viên chủ buổi
    Teacher,
    /// Trợ giảng của lớp gắn với buổi: điều khiển phòng như giáo viên (trừ kết thúc buổi, link khách)
    Assistant,
    Student,
}

impl LiveRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Teacher => "teacher",
            Self::Assistant => "assistant",
            Self::Student => "student",
        }
    }
}

/// Trợ giảng (đang hoạt động) của lớp gắn với buổi học?
async fn class_assistant(ctx: &AppContext, s: &SessionRow, user: Uuid) -> AppResult<bool> {
    match s.class_id {
        Some(class) => classes::is_assistant(&ctx.db, class, user).await,
        None => Ok(false),
    }
}

pub async fn enter_live(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<(SessionRow, LiveRole)> {
    let s = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    let (allowed, _) = can_access(ctx, actor, &s).await?;
    if !allowed {
        return Err(not_found());
    }
    let role = if s.owner_id == actor.id {
        LiveRole::Teacher
    } else if class_assistant(ctx, &s, actor.id).await? {
        LiveRole::Assistant
    } else {
        LiveRole::Student
    };
    live_window(&s, role != LiveRole::Student)?;
    if role == LiveRole::Student {
        repo::mark_joined(&ctx.db, id, actor.id).await?;
    }
    Ok((s, role))
}

/// Buổi học có phòng của hệ thống, đang trong giờ mở phòng (khách dùng chung quy tắc với học sinh).
pub fn live_window(s: &SessionRow, is_owner: bool) -> AppResult<()> {
    if s.room != "live" {
        return Err(AppError::conflict("NOT_LIVE_ROOM", "Buổi học này dùng link họp ngoài."));
    }
    if s.status == "cancelled" {
        return Err(AppError::conflict("SESSION_CANCELLED", "Buổi học đã hủy."));
    }
    let now = Utc::now();
    let opens = s.starts_at - if is_owner { LIVE_TEACHER_EARLY } else { JOIN_EARLY };
    if now < opens {
        return Err(AppError::conflict(
            "SESSION_NOT_OPEN",
            format!("Chưa tới giờ vào học. Phòng mở lúc {}.", time::format_vn(opens)),
        ));
    }
    if now > s.ends_at + LIVE_OVERTIME {
        return Err(AppError::conflict("SESSION_ENDED", "Buổi học đã kết thúc."));
    }
    Ok(())
}

/// Buổi học (có phòng của hệ thống) của chính giáo viên.
pub async fn own_live(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<SessionRow> {
    let s = load_own(ctx, actor, id).await?;
    if s.room != "live" {
        return Err(AppError::conflict("NOT_LIVE_ROOM", "Buổi học này dùng link họp ngoài."));
    }
    Ok(s)
}

/// Buổi có phòng của hệ thống mà người dùng điều khiển được: giáo viên chủ buổi hoặc trợ giảng của lớp.
pub async fn live_staff(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<SessionRow> {
    actor.require(Permission::TeachingManage)?;
    let s = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if s.owner_id != actor.id && !class_assistant(ctx, &s, actor.id).await? {
        return Err(not_found());
    }
    if s.room != "live" {
        return Err(AppError::conflict("NOT_LIVE_ROOM", "Buổi học này dùng link họp ngoài."));
    }
    Ok(s)
}

/// Buổi học người xem được vào (xem lại bảng / bình chọn sau buổi).
pub async fn viewable(ctx: &AppContext, actor: &Actor, id: Uuid) -> AppResult<SessionRow> {
    let s = repo::find(&ctx.db, id).await?.ok_or_else(not_found)?;
    if !can_access(ctx, actor, &s).await?.0 {
        return Err(not_found());
    }
    Ok(s)
}

/// Buổi học theo ID (khách vào bằng link — kiểm tra khóa ở module `live`).
pub async fn find_row(ctx: &AppContext, id: Uuid) -> AppResult<SessionRow> {
    repo::find(&ctx.db, id).await?.ok_or_else(not_found)
}
