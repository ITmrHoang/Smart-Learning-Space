use sqlx::{PgExecutor, Postgres, QueryBuilder, types::Json};
use uuid::Uuid;

use super::model::{ClassRow, MemberRow, Membership, PathVersionRow, ProgressRow, Step, StepRow};

const CLASS_COLS: &str = "c.id, c.owner_id, c.subject_id, c.title, c.description, c.mode, c.visibility, c.enrollment, \
     c.invite_code, c.copied_from, c.path_updated_at, c.archived_at, c.created_at, c.updated_at, \
     array(SELECT grade_id FROM class_grades g WHERE g.class_id = c.id) AS grade_ids, \
     array(SELECT tag_id FROM class_tags t WHERE t.class_id = c.id) AS tag_ids, \
     (SELECT full_name FROM users u WHERE u.id = c.owner_id) AS owner_name, \
     (SELECT count(*) FROM class_members m WHERE m.class_id = c.id AND m.role = 'student' AND m.status = 'active') AS student_count, \
     (SELECT count(*) FROM class_members m WHERE m.class_id = c.id AND m.role = 'student' AND m.status = 'pending') AS pending_count, \
     pv.version AS path_version, pv.published_at AS path_published_at, \
     COALESCE((SELECT count(*) FROM jsonb_array_elements(pv.steps) s WHERE s->>'kind' <> 'milestone'), 0) AS step_count";

const CLASS_FROM: &str = " FROM classes c LEFT JOIN LATERAL (SELECT version, published_at, steps FROM path_versions p \
     WHERE p.class_id = c.id ORDER BY version DESC LIMIT 1) pv ON true";

pub struct ClassFields<'a> {
    pub subject_id: Uuid,
    pub title: &'a str,
    pub description: &'a str,
    pub mode: &'a str,
    pub visibility: &'a str,
    pub enrollment: &'a str,
}

pub enum ClassScope {
    /// Lớp học của giáo viên.
    Owner(Uuid, bool),
    /// Danh sách công khai.
    Public,
}

pub struct ClassFilter<'a> {
    pub scope: ClassScope,
    pub q: Option<&'a str>,
    pub subject_id: Option<Uuid>,
    pub grade_id: Option<Uuid>,
}

fn push_filter(qb: &mut QueryBuilder<Postgres>, f: &ClassFilter<'_>) {
    match f.scope {
        ClassScope::Owner(owner, archived) => {
            qb.push(" WHERE c.owner_id = ").push_bind(owner);
            qb.push(if archived { " AND c.archived_at IS NOT NULL" } else { " AND c.archived_at IS NULL" });
        }
        ClassScope::Public => {
            qb.push(" WHERE c.visibility = 'public' AND c.archived_at IS NULL");
        }
    }
    if let Some(q) = f.q {
        qb.push(" AND unaccent(lower(c.title)) LIKE unaccent(lower(")
            .push_bind(format!("%{}%", crate::db::escape_like(q)))
            .push("))");
    }
    if let Some(s) = f.subject_id {
        qb.push(" AND c.subject_id = ").push_bind(s);
    }
    if let Some(g) = f.grade_id {
        qb.push(" AND EXISTS (SELECT 1 FROM class_grades cg WHERE cg.class_id = c.id AND cg.grade_id = ")
            .push_bind(g)
            .push(")");
    }
}

pub async fn list<'e, E: PgExecutor<'e>>(
    db: E,
    f: &ClassFilter<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<ClassRow>> {
    let mut qb = QueryBuilder::new(format!("SELECT {CLASS_COLS}{CLASS_FROM}"));
    push_filter(&mut qb, f);
    qb.push(" ORDER BY c.updated_at DESC LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);
    qb.build_query_as::<ClassRow>().fetch_all(db).await
}

pub async fn count<'e, E: PgExecutor<'e>>(db: E, f: &ClassFilter<'_>) -> sqlx::Result<i64> {
    let mut qb = QueryBuilder::new("SELECT count(*) FROM classes c");
    push_filter(&mut qb, f);
    qb.build_query_scalar::<i64>().fetch_one(db).await
}

pub async fn find<'e, E: PgExecutor<'e>>(db: E, id: Uuid) -> sqlx::Result<Option<ClassRow>> {
    sqlx::query_as::<_, ClassRow>(crate::sql!("SELECT {CLASS_COLS}{CLASS_FROM} WHERE c.id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await
}

pub async fn find_by_code<'e, E: PgExecutor<'e>>(db: E, code: &str) -> sqlx::Result<Option<ClassRow>> {
    sqlx::query_as::<_, ClassRow>(crate::sql!("SELECT {CLASS_COLS}{CLASS_FROM} WHERE c.invite_code = $1"))
        .bind(code)
        .fetch_optional(db)
        .await
}

/// Lớp học mà `user_id` là học sinh (đang học hoặc chờ duyệt).
pub async fn list_for_student<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid) -> sqlx::Result<Vec<(ClassRow, String)>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        #[sqlx(flatten)]
        class: ClassRow,
        member_status: String,
    }
    let rows = sqlx::query_as::<_, Row>(crate::sql!(
        "SELECT {CLASS_COLS}, m.status AS member_status{CLASS_FROM} \
         JOIN class_members m ON m.class_id = c.id AND m.user_id = $1 AND m.role = 'student' \
         WHERE m.status IN ('active', 'pending') AND c.archived_at IS NULL ORDER BY m.updated_at DESC"
    ))
    .bind(user_id)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|r| (r.class, r.member_status)).collect())
}

pub async fn insert(
    conn: &mut sqlx::PgConnection,
    owner_id: Uuid,
    f: &ClassFields<'_>,
    invite_code: &str,
    copied_from: Option<Uuid>,
) -> sqlx::Result<Uuid> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO classes (owner_id, subject_id, title, description, mode, visibility, enrollment, invite_code, copied_from) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(owner_id)
    .bind(f.subject_id)
    .bind(f.title)
    .bind(f.description)
    .bind(f.mode)
    .bind(f.visibility)
    .bind(f.enrollment)
    .bind(invite_code)
    .bind(copied_from)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query("INSERT INTO class_members (class_id, user_id, role, status) VALUES ($1, $2, 'owner', 'active')")
        .bind(id)
        .bind(owner_id)
        .execute(&mut *conn)
        .await?;
    Ok(id)
}

pub async fn update<'e, E: PgExecutor<'e>>(db: E, id: Uuid, f: &ClassFields<'_>) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE classes SET subject_id = $2, title = $3, description = $4, mode = $5, visibility = $6, enrollment = $7, \
         updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(f.subject_id)
    .bind(f.title)
    .bind(f.description)
    .bind(f.mode)
    .bind(f.visibility)
    .bind(f.enrollment)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_grades_tags(
    conn: &mut sqlx::PgConnection,
    id: Uuid,
    grade_ids: &[Uuid],
    tag_ids: &[Uuid],
) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM class_grades WHERE class_id = $1").bind(id).execute(&mut *conn).await?;
    sqlx::query("INSERT INTO class_grades (class_id, grade_id) SELECT $1, unnest($2::uuid[]) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(grade_ids)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM class_tags WHERE class_id = $1").bind(id).execute(&mut *conn).await?;
    sqlx::query("INSERT INTO class_tags (class_id, tag_id) SELECT $1, unnest($2::uuid[]) ON CONFLICT DO NOTHING")
        .bind(id)
        .bind(tag_ids)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn set_archived<'e, E: PgExecutor<'e>>(db: E, id: Uuid, archived: bool) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE classes SET archived_at = CASE WHEN $2 THEN now() ELSE NULL END, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(archived)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_invite_code<'e, E: PgExecutor<'e>>(db: E, id: Uuid, code: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE classes SET invite_code = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(code)
        .execute(db)
        .await?;
    Ok(())
}

// ---------- Thành viên ----------

pub async fn membership<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Uuid,
    user_id: Uuid,
) -> sqlx::Result<Option<Membership>> {
    sqlx::query_as::<_, Membership>("SELECT role, status FROM class_members WHERE class_id = $1 AND user_id = $2")
        .bind(class_id)
        .bind(user_id)
        .fetch_optional(db)
        .await
}

/// Thêm / cập nhật học sinh với trạng thái `status`.
pub async fn upsert_student<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Uuid,
    user_id: Uuid,
    status: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO class_members (class_id, user_id, role, status) VALUES ($1, $2, 'student', $3) \
         ON CONFLICT (class_id, user_id) DO UPDATE SET status = EXCLUDED.status, updated_at = now() \
         WHERE class_members.role = 'student'",
    )
    .bind(class_id)
    .bind(user_id)
    .bind(status)
    .execute(db)
    .await?;
    Ok(())
}

/// Đổi trạng thái khi trạng thái hiện tại nằm trong `from`. Trả `false` nếu không có dòng nào đổi.
pub async fn transition<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Uuid,
    user_id: Uuid,
    from: &[&str],
    to: &str,
) -> sqlx::Result<bool> {
    let from: Vec<String> = from.iter().map(|s| s.to_string()).collect();
    let res = sqlx::query(
        "UPDATE class_members SET status = $4, updated_at = now() \
         WHERE class_id = $1 AND user_id = $2 AND role = 'student' AND status = ANY($3)",
    )
    .bind(class_id)
    .bind(user_id)
    .bind(from)
    .bind(to)
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn members<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Uuid,
    status: Option<&str>,
) -> sqlx::Result<Vec<MemberRow>> {
    sqlx::query_as::<_, MemberRow>(
        "SELECT m.user_id, u.username, u.full_name, u.email, m.role, m.status, m.created_at, m.updated_at \
         FROM class_members m JOIN users u ON u.id = m.user_id \
         WHERE m.class_id = $1 AND m.role = 'student' AND ($2::text IS NULL OR m.status = $2) \
         ORDER BY m.status = 'pending' DESC, u.full_name",
    )
    .bind(class_id)
    .bind(status)
    .fetch_all(db)
    .await
}

// ---------- Lộ trình ----------

pub async fn draft_steps<'e, E: PgExecutor<'e>>(db: E, class_id: Uuid) -> sqlx::Result<Vec<Step>> {
    Ok(sqlx::query_as::<_, StepRow>(
        "SELECT id, kind, title, lesson_id, exam_id, unlock, pass_score FROM path_steps WHERE class_id = $1 ORDER BY position",
    )
    .bind(class_id)
    .fetch_all(db)
    .await?
    .into_iter()
    .map(Into::into)
    .collect())
}

/// Ghi lại toàn bộ bản nháp lộ trình theo thứ tự.
pub async fn replace_steps(conn: &mut sqlx::PgConnection, class_id: Uuid, steps: &[Step]) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM path_steps WHERE class_id = $1").bind(class_id).execute(&mut *conn).await?;
    for (i, s) in steps.iter().enumerate() {
        sqlx::query(
            "INSERT INTO path_steps (id, class_id, position, kind, title, lesson_id, exam_id, unlock, pass_score) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(s.id)
        .bind(class_id)
        .bind(i as i32)
        .bind(s.kind.as_str())
        .bind(&s.title)
        .bind(s.lesson_id)
        .bind(s.exam_id)
        .bind(s.unlock.as_str())
        .bind(s.pass_score)
        .execute(&mut *conn)
        .await?;
    }
    sqlx::query("UPDATE classes SET path_updated_at = now() WHERE id = $1").bind(class_id).execute(&mut *conn).await?;
    Ok(())
}

pub async fn latest_version<'e, E: PgExecutor<'e>>(db: E, class_id: Uuid) -> sqlx::Result<Option<PathVersionRow>> {
    sqlx::query_as::<_, PathVersionRow>(
        "SELECT version, steps, published_at FROM path_versions WHERE class_id = $1 ORDER BY version DESC LIMIT 1",
    )
    .bind(class_id)
    .fetch_optional(db)
    .await
}

pub async fn insert_version<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Uuid,
    steps: &[Step],
    by: Uuid,
) -> sqlx::Result<i32> {
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO path_versions (class_id, version, steps, published_by) \
         SELECT $1, COALESCE(max(version), 0) + 1, $2, $3 FROM path_versions WHERE class_id = $1 RETURNING version",
    )
    .bind(class_id)
    .bind(Json(steps))
    .bind(by)
    .fetch_one(db)
    .await
}

// ---------- Tiến độ ----------

pub async fn progress<'e, E: PgExecutor<'e>>(
    db: E,
    class_id: Uuid,
    user_id: Option<Uuid>,
) -> sqlx::Result<Vec<ProgressRow>> {
    sqlx::query_as::<_, ProgressRow>(
        "SELECT user_id, step_id, completed_at FROM step_progress WHERE class_id = $1 AND ($2::uuid IS NULL OR user_id = $2)",
    )
    .bind(class_id)
    .bind(user_id)
    .fetch_all(db)
    .await
}

pub async fn mark_done<'e, E: PgExecutor<'e>>(db: E, user_id: Uuid, class_id: Uuid, step_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO step_progress (user_id, class_id, step_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .bind(class_id)
        .bind(step_id)
        .execute(db)
        .await?;
    Ok(())
}
