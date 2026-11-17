use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use super::model::{AttendeeRow, InviteeRow, Recipient, SessionRef, SessionRow};

const SESSION_COLS: &str = "s.id, s.owner_id, s.schedule_id, s.class_id, s.title, s.description, s.starts_at, s.ends_at, \
     s.status, s.access, s.meeting_url, s.room, s.remind_minutes, s.ics_sequence, c.title AS class_title, u.full_name AS owner_name";
const SESSION_FROM: &str =
    " FROM sessions s LEFT JOIN classes c ON c.id = s.class_id JOIN users u ON u.id = s.owner_id";

/// Điều kiện "học sinh `$u` được mời vào buổi `s`" (theo học sinh, lớp học đang học hoặc làm trợ giảng, nhóm).
macro_rules! invited_cond {
    ($u:literal) => {
        concat!(
            "EXISTS (SELECT 1 FROM session_invitees i WHERE i.session_id = s.id AND (",
            "(i.target_type = 'user' AND i.target_id = ", $u, ") OR ",
            "(i.target_type = 'class' AND EXISTS (SELECT 1 FROM class_members m WHERE m.class_id = i.target_id ",
            "AND m.user_id = ", $u, " AND m.role IN ('student', 'assistant') AND m.status = 'active')) OR ",
            "(i.target_type = 'group' AND EXISTS (SELECT 1 FROM student_group_members g WHERE g.group_id = i.target_id ",
            "AND g.user_id = ", $u, "))))"
        )
    };
}

pub struct NewSession<'a> {
    pub owner_id: Uuid,
    pub schedule_id: Option<Uuid>,
    pub class_id: Option<Uuid>,
    pub title: &'a str,
    pub description: &'a str,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub access: &'a str,
    pub meeting_url: Option<&'a str>,
    pub room: &'a str,
    pub remind_minutes: i32,
}

pub async fn insert_schedule(
    conn: &mut sqlx::PgConnection,
    owner_id: Uuid,
    weekdays: &[i32],
    start: NaiveDate,
    end: NaiveDate,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO schedules (owner_id, weekdays, start_date, end_date) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(owner_id)
    .bind(weekdays)
    .bind(start)
    .bind(end)
    .fetch_one(&mut *conn)
    .await
}

pub async fn insert(conn: &mut sqlx::PgConnection, s: &NewSession<'_>) -> sqlx::Result<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO sessions (owner_id, schedule_id, class_id, title, description, starts_at, ends_at, access, meeting_url, remind_minutes, \
         room) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id",
    )
    .bind(s.owner_id)
    .bind(s.schedule_id)
    .bind(s.class_id)
    .bind(s.title)
    .bind(s.description)
    .bind(s.starts_at)
    .bind(s.ends_at)
    .bind(s.access)
    .bind(s.meeting_url)
    .bind(s.remind_minutes)
    .bind(s.room)
    .fetch_one(&mut *conn)
    .await
}

/// Sửa nội dung + giờ; dời giờ thì nhắc lại từ đầu. Tăng SEQUENCE của `.ics`.
pub async fn update(conn: &mut sqlx::PgConnection, id: Uuid, s: &NewSession<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE sessions SET class_id = $2, title = $3, description = $4, \
            reminded_at = CASE WHEN starts_at = $5 AND remind_minutes = $9 THEN reminded_at ELSE NULL END, \
            starts_at = $5, ends_at = $6, access = $7, meeting_url = $8, remind_minutes = $9, room = $10, \
            ics_sequence = ics_sequence + 1, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(s.class_id)
    .bind(s.title)
    .bind(s.description)
    .bind(s.starts_at)
    .bind(s.ends_at)
    .bind(s.access)
    .bind(s.meeting_url)
    .bind(s.remind_minutes)
    .bind(s.room)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn cancel<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE sessions SET status = 'cancelled', ics_sequence = ics_sequence + 1, updated_at = now() WHERE id = ANY($1)",
    )
    .bind(ids)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_invitees(conn: &mut sqlx::PgConnection, id: Uuid, targets: &[(&str, Uuid)]) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM session_invitees WHERE session_id = $1").bind(id).execute(&mut *conn).await?;
    let types: Vec<String> = targets.iter().map(|(t, _)| t.to_string()).collect();
    let ids: Vec<Uuid> = targets.iter().map(|(_, i)| *i).collect();
    sqlx::query(
        "INSERT INTO session_invitees (session_id, target_type, target_id) \
         SELECT $1, t, i FROM unnest($2::text[], $3::uuid[]) AS x(t, i) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(&types)
    .bind(&ids)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn set_lessons(conn: &mut sqlx::PgConnection, id: Uuid, lesson_ids: &[Uuid]) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM session_lessons WHERE session_id = $1").bind(id).execute(&mut *conn).await?;
    sqlx::query(
        "INSERT INTO session_lessons (session_id, lesson_id, position) \
         SELECT $1, l, (ord - 1)::int FROM unnest($2::uuid[]) WITH ORDINALITY AS t(l, ord) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(lesson_ids)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn find<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<SessionRow>> {
    sqlx::query_as::<_, SessionRow>(crate::sql!("SELECT {SESSION_COLS}{SESSION_FROM} WHERE s.id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn find_many<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid]) -> sqlx::Result<Vec<SessionRow>> {
    sqlx::query_as::<_, SessionRow>(crate::sql!(
        "SELECT {SESSION_COLS}{SESSION_FROM} WHERE s.id = ANY($1) ORDER BY s.starts_at"
    ))
    .bind(ids)
    .fetch_all(db)
    .await
}

/// Các buổi cùng lịch lặp, từ `from` trở đi, chưa hủy.
pub async fn following<'e, E: PgExecutor<'e>>(
    db: E,
    schedule_id: Uuid,
    from: DateTime<Utc>,
) -> sqlx::Result<Vec<SessionRow>> {
    sqlx::query_as::<_, SessionRow>(crate::sql!(
        "SELECT {SESSION_COLS}{SESSION_FROM} WHERE s.schedule_id = $1 AND s.starts_at >= $2 AND s.status = 'scheduled' \
         ORDER BY s.starts_at"
    ))
    .bind(schedule_id)
    .bind(from)
    .fetch_all(db)
    .await
}

/// Buổi của giáo viên trong khoảng thời gian.
pub async fn list_owned<'e, E: PgExecutor<'e>>(
    db: E,
    owner: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    class_id: Option<Uuid>,
) -> sqlx::Result<Vec<SessionRow>> {
    sqlx::query_as::<_, SessionRow>(crate::sql!(
        "SELECT {SESSION_COLS}{SESSION_FROM} WHERE s.owner_id = $1 AND s.starts_at < $3 AND s.ends_at > $2 \
         AND ($4::uuid IS NULL OR s.class_id = $4) ORDER BY s.starts_at"
    ))
    .bind(owner)
    .bind(from)
    .bind(to)
    .bind(class_id)
    .fetch_all(db)
    .await
}

/// Buổi học sinh được mời trong khoảng thời gian, kèm trả lời của học sinh.
pub async fn list_invited<'e, E: PgExecutor<'e>>(
    db: E,
    user: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    class_id: Option<Uuid>,
) -> sqlx::Result<Vec<(SessionRow, Option<String>)>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        #[sqlx(flatten)]
        s: SessionRow,
        my_rsvp: Option<String>,
    }
    let rows = sqlx::query_as::<_, Row>(crate::sql!(
        "SELECT {SESSION_COLS}, (SELECT a.rsvp FROM session_attendees a WHERE a.session_id = s.id AND a.user_id = $1) AS my_rsvp \
         {SESSION_FROM} WHERE s.starts_at < $3 AND s.ends_at > $2 AND ($4::uuid IS NULL OR s.class_id = $4) AND {} \
         ORDER BY s.starts_at",
        invited_cond!("$1")
    ))
    .bind(user)
    .bind(from)
    .bind(to)
    .bind(class_id)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|r| (r.s, r.my_rsvp)).collect())
}

pub async fn is_invited<'e, E: PgExecutor<'e>>(db: E, session: Uuid, user: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar::<_, bool>(crate::sql!("SELECT {} FROM sessions s WHERE s.id = $2", invited_cond!("$1")))
        .bind(user)
        .bind(session)
        .fetch_optional(db)
        .await
        .map(|v| v.unwrap_or(false))
}

pub async fn is_class_member<'e, E: PgExecutor<'e>>(db: E, class_id: Uuid, user: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM class_members WHERE class_id = $1 AND user_id = $2 AND status = 'active')",
    )
    .bind(class_id)
    .bind(user)
    .fetch_one(db)
    .await
}

/// Học sinh được mời (đang hoạt động) của các buổi.
pub async fn recipients<'e, E: PgExecutor<'e>>(db: E, session_ids: &[Uuid]) -> sqlx::Result<Vec<Recipient>> {
    sqlx::query_as::<_, Recipient>(
        "SELECT u.id, u.username, u.full_name, u.email FROM users u WHERE u.status = 'active' AND u.id IN ( \
           SELECT target_id FROM session_invitees WHERE session_id = ANY($1) AND target_type = 'user' \
           UNION SELECT m.user_id FROM session_invitees i JOIN class_members m ON m.class_id = i.target_id \
             AND m.role = 'student' AND m.status = 'active' WHERE i.session_id = ANY($1) AND i.target_type = 'class' \
           UNION SELECT g.user_id FROM session_invitees i JOIN student_group_members g ON g.group_id = i.target_id \
             WHERE i.session_id = ANY($1) AND i.target_type = 'group') ORDER BY u.full_name",
    )
    .bind(session_ids)
    .fetch_all(db)
    .await
}

pub async fn invitees<'e, E: PgExecutor<'e>>(db: E, session_id: Uuid) -> sqlx::Result<Vec<InviteeRow>> {
    sqlx::query_as::<_, InviteeRow>(
        "SELECT i.target_type, i.target_id, COALESCE(c.title, g.name, u.full_name, '') AS name FROM session_invitees i \
         LEFT JOIN classes c ON i.target_type = 'class' AND c.id = i.target_id \
         LEFT JOIN student_groups g ON i.target_type = 'group' AND g.id = i.target_id \
         LEFT JOIN users u ON i.target_type = 'user' AND u.id = i.target_id \
         WHERE i.session_id = $1 ORDER BY i.target_type, name",
    )
    .bind(session_id)
    .fetch_all(db)
    .await
}

pub async fn lessons<'e, E: PgExecutor<'e>>(db: E, session_id: Uuid) -> sqlx::Result<Vec<(Uuid, String)>> {
    sqlx::query_as::<_, (Uuid, String)>(
        "SELECT l.id, l.title FROM session_lessons sl JOIN lessons l ON l.id = sl.lesson_id WHERE sl.session_id = $1 ORDER BY sl.position",
    )
    .bind(session_id)
    .fetch_all(db)
    .await
}

pub async fn attendees<'e, E: PgExecutor<'e>>(db: E, session_id: Uuid) -> sqlx::Result<Vec<AttendeeRow>> {
    sqlx::query_as::<_, AttendeeRow>(
        "SELECT user_id, rsvp, joined_at, attended FROM session_attendees WHERE session_id = $1",
    )
    .bind(session_id)
    .fetch_all(db)
    .await
}

pub async fn set_rsvp<'e, E: PgExecutor<'e>>(db: E, session_id: Uuid, user: Uuid, rsvp: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO session_attendees (session_id, user_id, rsvp, rsvp_at) VALUES ($1, $2, $3, now()) \
         ON CONFLICT (session_id, user_id) DO UPDATE SET rsvp = EXCLUDED.rsvp, rsvp_at = now()",
    )
    .bind(session_id)
    .bind(user)
    .bind(rsvp)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn mark_joined<'e, E: PgExecutor<'e>>(db: E, session_id: Uuid, user: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO session_attendees (session_id, user_id, joined_at, attended) VALUES ($1, $2, now(), true) \
         ON CONFLICT (session_id, user_id) DO UPDATE SET joined_at = COALESCE(session_attendees.joined_at, now()), attended = true",
    )
    .bind(session_id)
    .bind(user)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_attended<'e, E: PgExecutor<'e>>(
    db: E,
    session_id: Uuid,
    user: Uuid,
    attended: bool,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO session_attendees (session_id, user_id, attended) VALUES ($1, $2, $3) \
         ON CONFLICT (session_id, user_id) DO UPDATE SET attended = EXCLUDED.attended",
    )
    .bind(session_id)
    .bind(user)
    .bind(attended)
    .execute(db)
    .await?;
    Ok(())
}

/// Buổi đã lên lịch của giáo viên trùng một trong các khoảng giờ (bỏ qua `exclude`).
pub async fn owner_conflict<'e, E: PgExecutor<'e>>(
    db: E,
    owner: Uuid,
    starts: &[DateTime<Utc>],
    ends: &[DateTime<Utc>],
    exclude: &[Uuid],
) -> sqlx::Result<Option<(String, DateTime<Utc>)>> {
    sqlx::query_as::<_, (String, DateTime<Utc>)>(
        "SELECT s.title, s.starts_at FROM sessions s, unnest($2::timestamptz[], $3::timestamptz[]) AS t(st, en) \
         WHERE s.owner_id = $1 AND s.status = 'scheduled' AND s.starts_at < t.en AND s.ends_at > t.st \
           AND NOT (s.id = ANY($4)) ORDER BY s.starts_at LIMIT 1",
    )
    .bind(owner)
    .bind(starts)
    .bind(ends)
    .bind(exclude)
    .fetch_optional(db)
    .await
}

/// Số học sinh (trong `users`) có buổi khác trùng giờ.
pub async fn student_conflicts<'e, E: PgExecutor<'e>>(
    db: E,
    users: &[Uuid],
    starts: &[DateTime<Utc>],
    ends: &[DateTime<Utc>],
    exclude: &[Uuid],
) -> sqlx::Result<i64> {
    sqlx::query_scalar::<_, i64>(crate::sql!(
        "SELECT count(*) FROM unnest($1::uuid[]) AS x(uid) WHERE EXISTS ( \
           SELECT 1 FROM sessions s, unnest($2::timestamptz[], $3::timestamptz[]) AS t(st, en) \
           WHERE s.status = 'scheduled' AND s.starts_at < t.en AND s.ends_at > t.st AND NOT (s.id = ANY($4)) AND {})",
        invited_cond!("x.uid")
    ))
    .bind(users)
    .bind(starts)
    .bind(ends)
    .bind(exclude)
    .fetch_one(db)
    .await
}

/// "Nhận" các buổi đến giờ nhắc (đánh dấu đã nhắc ngay trong câu lệnh → không gửi trùng).
pub async fn claim_due_reminders<'e, E: PgExecutor<'e>>(db: E) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar::<_, Uuid>(
        "UPDATE sessions SET reminded_at = now() WHERE status = 'scheduled' AND reminded_at IS NULL AND remind_minutes > 0 \
           AND starts_at > now() AND starts_at - make_interval(mins => remind_minutes) <= now() RETURNING id",
    )
    .fetch_all(db)
    .await
}

pub async fn refs<'e, E: PgExecutor<'e>>(db: E, ids: &[Uuid]) -> sqlx::Result<Vec<SessionRef>> {
    sqlx::query_as::<_, SessionRef>(
        "SELECT id, owner_id, class_id, title, starts_at, ends_at, status FROM sessions WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(db)
    .await
}

/// (học sinh, buổi) đã điểm danh trong các buổi.
pub async fn attended_pairs<'e, E: PgExecutor<'e>>(
    db: E,
    session_ids: &[Uuid],
    user: Option<Uuid>,
) -> sqlx::Result<Vec<(Uuid, Uuid)>> {
    sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT user_id, session_id FROM session_attendees WHERE session_id = ANY($1) AND attended \
         AND ($2::uuid IS NULL OR user_id = $2)",
    )
    .bind(session_ids)
    .bind(user)
    .fetch_all(db)
    .await
}

/// Với từng học sinh: số buổi (không hủy, đã kết thúc trong khoảng) mà học sinh được mời, và số buổi có mặt.
/// `class_id = None` → mọi lớp học.
pub async fn attendance_stats<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Option<Uuid>,
    user_ids: &[Uuid],
    from: Option<DateTime<Utc>>,
    to: DateTime<Utc>,
) -> sqlx::Result<Vec<(Uuid, i64, i64)>> {
    sqlx::query_as::<_, (Uuid, i64, i64)>(crate::sql!(
        "SELECT u.id, count(s.id), count(s.id) FILTER (WHERE a.attended) \
         FROM unnest($2::uuid[]) AS u(id) \
         LEFT JOIN sessions s ON s.status = 'scheduled' AND s.ends_at <= $4 \
           AND ($3::timestamptz IS NULL OR s.starts_at >= $3) AND ($1::uuid IS NULL OR s.class_id = $1) AND {} \
         LEFT JOIN session_attendees a ON a.session_id = s.id AND a.user_id = u.id \
         GROUP BY u.id",
        invited_cond!("u.id")
    ))
    .bind(class_id)
    .bind(user_ids)
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
}
