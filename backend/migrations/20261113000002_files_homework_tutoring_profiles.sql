-- Đợt 5: lưu file trên Garage (M21), bài tập về nhà (M22), đặt lịch học kèm 1-1 (M23),
-- lộ trình riêng cho nhóm / học sinh (M24), hồ sơ giáo viên công khai (M25).

-- ---------- M21: File ----------

-- File người dùng upload thẳng lên Garage. ID ngẫu nhiên (v4) vì link ảnh nội dung không cần đăng nhập.
CREATE TABLE files (
    id           uuid PRIMARY KEY,
    owner_id     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    purpose      text        NOT NULL,              -- content (ảnh trong nội dung) | homework (bài làm, link có chữ ký)
    object_key   text        NOT NULL,
    name         text        NOT NULL,              -- tên file gốc (hiện khi tải về)
    content_type text        NOT NULL,
    size_bytes   bigint      NOT NULL,
    status       text        NOT NULL DEFAULT 'pending',  -- pending (đã cấp link upload) | ready
    created_at   timestamptz NOT NULL DEFAULT now(),
    ready_at     timestamptz,
    CONSTRAINT files_purpose_check CHECK (purpose IN ('content', 'homework')),
    CONSTRAINT files_status_check CHECK (status IN ('pending', 'ready'))
);
CREATE UNIQUE INDEX files_object_key_key ON files (object_key);
CREATE INDEX files_owner_idx ON files (owner_id, created_at DESC);

-- ---------- M22: Bài tập về nhà ----------

-- Bài tập trong thư viện của giáo viên (dùng lại ở nhiều lớp học, như bộ thẻ).
CREATE TABLE assignments (
    id           uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id   uuid        NOT NULL REFERENCES subjects (id),
    title        text        NOT NULL,
    instructions text        NOT NULL DEFAULT '',   -- Markdown + công thức + ảnh
    max_score    double precision,                  -- NULL = chỉ nhận xét, không chấm điểm
    archived_at  timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX assignments_owner_idx ON assignments (owner_id, updated_at DESC);

ALTER TABLE path_steps DROP CONSTRAINT path_steps_kind_check;
ALTER TABLE path_steps ADD CONSTRAINT path_steps_kind_check
    CHECK (kind IN ('milestone', 'lesson', 'exam', 'session', 'deck', 'homework'));
ALTER TABLE path_steps ADD COLUMN assignment_id uuid REFERENCES assignments (id) ON DELETE SET NULL;
-- Hạn nộp của bước bài tập trong lớp học này (NULL = không có hạn).
ALTER TABLE path_steps ADD COLUMN due_at timestamptz;

-- Bài nộp: mỗi học sinh một bài cho mỗi bước bài tập; nộp lại được khi chưa chấm hoặc giáo viên trả lại.
CREATE TABLE homework_submissions (
    id            uuid PRIMARY KEY DEFAULT uuidv7(),
    class_id      uuid        NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    step_id       uuid        NOT NULL,
    assignment_id uuid        NOT NULL REFERENCES assignments (id) ON DELETE CASCADE,
    student_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status        text        NOT NULL DEFAULT 'submitted',   -- submitted | returned (trả lại để làm lại) | graded
    note          text        NOT NULL DEFAULT '',            -- lời nhắn của học sinh
    submitted_at  timestamptz NOT NULL DEFAULT now(),
    late          boolean     NOT NULL DEFAULT false,
    submit_count  int         NOT NULL DEFAULT 1,
    score         double precision,
    feedback      text        NOT NULL DEFAULT '',            -- nhận xét của giáo viên (Markdown)
    graded_by     uuid REFERENCES users (id) ON DELETE SET NULL,
    graded_at     timestamptz,
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT homework_submissions_status_check CHECK (status IN ('submitted', 'returned', 'graded'))
);
CREATE UNIQUE INDEX homework_submissions_key ON homework_submissions (class_id, step_id, student_id);
CREATE INDEX homework_submissions_assignment_idx ON homework_submissions (assignment_id);

-- Ảnh / PDF của bài nộp; giáo viên vẽ nhận xét lên ảnh → lưu thành ảnh mới (`annotated_file_id`).
CREATE TABLE submission_files (
    submission_id     uuid NOT NULL REFERENCES homework_submissions (id) ON DELETE CASCADE,
    position          int  NOT NULL,
    file_id           uuid NOT NULL REFERENCES files (id),
    annotated_file_id uuid REFERENCES files (id),
    PRIMARY KEY (submission_id, position)
);

-- Đã nhắc hạn nộp (một lần cho mỗi bước + hạn nộp; đổi hạn thì nhắc lại). Chạy nhiều instance không nhắc trùng.
CREATE TABLE homework_due_reminders (
    class_id uuid        NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    step_id  uuid        NOT NULL,
    due_at   timestamptz NOT NULL,
    sent_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (class_id, step_id, due_at)
);

-- ---------- M23: Đặt lịch học kèm 1-1 ----------

-- Giáo viên mở khung giờ rảnh hằng tuần (giờ Việt Nam); học sinh chọn một khoảng trong đó để đặt.
CREATE TABLE tutoring_offers (
    id               uuid PRIMARY KEY DEFAULT uuidv7(),
    teacher_id       uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id       uuid REFERENCES subjects (id),
    title            text        NOT NULL,
    description      text        NOT NULL DEFAULT '',
    duration_minutes int         NOT NULL,
    windows          jsonb       NOT NULL,          -- [{ "weekday": 1–7, "start": "HH:MM", "end": "HH:MM" }]
    meeting_url      text        NOT NULL DEFAULT '',
    audience         text        NOT NULL DEFAULT 'my_students',  -- my_students | everyone
    min_notice_hours int         NOT NULL DEFAULT 12,
    horizon_days     int         NOT NULL DEFAULT 14,
    active           boolean     NOT NULL DEFAULT true,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tutoring_offers_duration_check CHECK (duration_minutes BETWEEN 15 AND 240),
    CONSTRAINT tutoring_offers_audience_check CHECK (audience IN ('my_students', 'everyone'))
);
CREATE INDEX tutoring_offers_teacher_idx ON tutoring_offers (teacher_id);

CREATE TABLE tutoring_bookings (
    id           uuid PRIMARY KEY DEFAULT uuidv7(),
    offer_id     uuid        NOT NULL REFERENCES tutoring_offers (id) ON DELETE CASCADE,
    teacher_id   uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    student_id   uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    starts_at    timestamptz NOT NULL,
    ends_at      timestamptz NOT NULL,
    status       text        NOT NULL DEFAULT 'pending',  -- pending | approved | declined | cancelled
    message      text        NOT NULL DEFAULT '',         -- học sinh muốn học gì
    reply        text        NOT NULL DEFAULT '',         -- lý do từ chối / hủy
    session_id   uuid REFERENCES sessions (id) ON DELETE SET NULL,
    cancelled_by uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT tutoring_bookings_time_check CHECK (ends_at > starts_at),
    CONSTRAINT tutoring_bookings_status_check CHECK (status IN ('pending', 'approved', 'declined', 'cancelled'))
);
-- Một khung giờ của giáo viên chỉ có một yêu cầu đang chờ / đã duyệt (hai học sinh bấm cùng lúc → một người thắng).
CREATE UNIQUE INDEX tutoring_bookings_slot_key ON tutoring_bookings (teacher_id, starts_at)
    WHERE status IN ('pending', 'approved');
CREATE INDEX tutoring_bookings_student_idx ON tutoring_bookings (student_id, starts_at DESC);

-- ---------- M24: Lộ trình riêng ----------

-- Bước chỉ dành cho một số nhóm / học sinh. Cả hai mảng rỗng = cả lớp.
ALTER TABLE path_steps ADD COLUMN audience_group_ids uuid[] NOT NULL DEFAULT '{}';
ALTER TABLE path_steps ADD COLUMN audience_user_ids uuid[] NOT NULL DEFAULT '{}';

-- ---------- M25: Hồ sơ giáo viên công khai ----------

CREATE TABLE teacher_profiles (
    user_id        uuid PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    headline       text        NOT NULL DEFAULT '',  -- một dòng giới thiệu
    bio            text        NOT NULL DEFAULT '',  -- Markdown
    avatar_file_id uuid REFERENCES files (id) ON DELETE SET NULL,
    is_public      boolean     NOT NULL DEFAULT true,
    updated_at     timestamptz NOT NULL DEFAULT now()
);
