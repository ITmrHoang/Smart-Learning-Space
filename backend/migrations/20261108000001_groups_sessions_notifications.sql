-- M13–M16: nhóm học sinh, lịch & buổi học, thông báo.

-- ---------- M13: Nhóm học sinh ----------

-- Nhóm của giáo viên; class_id có giá trị khi nhóm nằm trong một lớp học (để lọc), NULL = gom từ nhiều lớp học.
CREATE TABLE student_groups (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id   uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    class_id   uuid REFERENCES classes (id) ON DELETE SET NULL,
    name       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX student_groups_owner_idx ON student_groups (owner_id);

CREATE TABLE student_group_members (
    group_id   uuid        NOT NULL REFERENCES student_groups (id) ON DELETE CASCADE,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, user_id)
);
CREATE INDEX student_group_members_user_idx ON student_group_members (user_id);

-- ---------- M14: Lịch & buổi học ----------

-- Luật lặp hằng tuần (để sửa "buổi này và các buổi sau"). Giờ tính theo UTC+7.
CREATE TABLE schedules (
    id               uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id         uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    weekdays         int[]       NOT NULL,   -- 1 = thứ Hai … 7 = Chủ nhật
    start_date       date        NOT NULL,
    end_date         date        NOT NULL,
    created_at       timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id              uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id        uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    schedule_id     uuid REFERENCES schedules (id) ON DELETE SET NULL,
    class_id        uuid REFERENCES classes (id) ON DELETE SET NULL,
    title           text        NOT NULL,
    description     text        NOT NULL DEFAULT '',
    starts_at       timestamptz NOT NULL,
    ends_at         timestamptz NOT NULL,
    status          text        NOT NULL DEFAULT 'scheduled',  -- scheduled | cancelled
    access          text        NOT NULL DEFAULT 'invited',    -- invited | members | logged_in
    meeting_url     text,
    remind_minutes  int         NOT NULL DEFAULT 60,           -- 0 = không nhắc
    reminded_at     timestamptz,
    ics_sequence    int         NOT NULL DEFAULT 0,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT sessions_time_check CHECK (ends_at > starts_at),
    CONSTRAINT sessions_status_check CHECK (status IN ('scheduled', 'cancelled')),
    CONSTRAINT sessions_access_check CHECK (access IN ('invited', 'members', 'logged_in'))
);
CREATE INDEX sessions_owner_time_idx ON sessions (owner_id, starts_at);
CREATE INDEX sessions_class_time_idx ON sessions (class_id, starts_at) WHERE class_id IS NOT NULL;
CREATE INDEX sessions_schedule_idx ON sessions (schedule_id, starts_at) WHERE schedule_id IS NOT NULL;
CREATE INDEX sessions_remind_idx ON sessions (starts_at) WHERE status = 'scheduled' AND reminded_at IS NULL AND remind_minutes > 0;

-- Mời theo lớp học / nhóm / từng học sinh (tính danh sách người được mời lúc xem).
CREATE TABLE session_invitees (
    session_id  uuid NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    target_type text NOT NULL,  -- class | group | user
    target_id   uuid NOT NULL,
    PRIMARY KEY (session_id, target_type, target_id),
    CONSTRAINT session_invitees_type_check CHECK (target_type IN ('class', 'group', 'user'))
);
CREATE INDEX session_invitees_target_idx ON session_invitees (target_type, target_id);

-- Bài sẽ dạy trong buổi.
CREATE TABLE session_lessons (
    session_id uuid NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    lesson_id  uuid NOT NULL REFERENCES lessons (id) ON DELETE CASCADE,
    position   int  NOT NULL,
    PRIMARY KEY (session_id, lesson_id)
);

-- Trả lời tham gia + điểm danh của từng học sinh.
CREATE TABLE session_attendees (
    session_id uuid        NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    rsvp       text,                   -- yes | no
    rsvp_at    timestamptz,
    joined_at  timestamptz,            -- lần bấm "Vào học" đầu tiên
    attended   boolean     NOT NULL DEFAULT false,
    PRIMARY KEY (session_id, user_id),
    CONSTRAINT session_attendees_rsvp_check CHECK (rsvp IS NULL OR rsvp IN ('yes', 'no'))
);

-- Bước "buổi học" trong lộ trình.
ALTER TABLE path_steps DROP CONSTRAINT path_steps_kind_check;
ALTER TABLE path_steps ADD CONSTRAINT path_steps_kind_check CHECK (kind IN ('milestone', 'lesson', 'exam', 'session'));
ALTER TABLE path_steps ADD COLUMN session_id uuid REFERENCES sessions (id) ON DELETE SET NULL;

-- ---------- M15: Thông báo ----------

CREATE TABLE notifications (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind       text        NOT NULL,   -- session_invite | session_updated | session_cancelled | session_reminder | enrollment_approved | class_added | path_published
    title      text        NOT NULL,
    body       text        NOT NULL DEFAULT '',
    link       text,                   -- đường dẫn trong app, ví dụ /buoi-hoc/{id}
    read_at    timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX notifications_user_idx ON notifications (user_id, created_at DESC);
CREATE INDEX notifications_unread_idx ON notifications (user_id) WHERE read_at IS NULL;
