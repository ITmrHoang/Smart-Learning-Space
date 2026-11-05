-- M7–M9: thư viện bài, lớp học + ghi danh, lộ trình + tiến độ.

-- ---------- M7: Thư viện bài ----------

-- Bản nháp của bài học (giáo viên sửa ở đây). Học sinh chỉ đọc bản đã xuất bản (lesson_revisions).
CREATE TABLE lessons (
    id                    uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id              uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id            uuid        NOT NULL REFERENCES subjects (id),
    grade_id              uuid REFERENCES grades (id),
    title                 text        NOT NULL,
    summary               text        NOT NULL DEFAULT '',
    content               text        NOT NULL DEFAULT '',   -- Markdown + LaTeX $...$
    video_url             text,                              -- YouTube hoặc link mp4 / m3u8
    duration_minutes      int,
    published_revision_id uuid,                              -- NULL = chưa xuất bản lần nào
    has_changes           boolean     NOT NULL DEFAULT true, -- bản nháp khác bản xuất bản mới nhất
    archived_at           timestamptz,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX lessons_owner_idx ON lessons (owner_id) WHERE archived_at IS NULL;

CREATE TABLE lesson_tags (
    lesson_id uuid NOT NULL REFERENCES lessons (id) ON DELETE CASCADE,
    tag_id    uuid NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (lesson_id, tag_id)
);

-- Mỗi lần xuất bản lưu một phiên bản (xem lại / quay về bản cũ).
CREATE TABLE lesson_revisions (
    id               uuid PRIMARY KEY DEFAULT uuidv7(),
    lesson_id        uuid        NOT NULL REFERENCES lessons (id) ON DELETE CASCADE,
    version          int         NOT NULL,
    title            text        NOT NULL,
    summary          text        NOT NULL,
    content          text        NOT NULL,
    video_url        text,
    duration_minutes int,
    published_by     uuid        NOT NULL REFERENCES users (id),
    published_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (lesson_id, version)
);
ALTER TABLE lessons
    ADD CONSTRAINT lessons_published_revision_fkey FOREIGN KEY (published_revision_id) REFERENCES lesson_revisions (id);

-- ---------- M8: Lớp học + ghi danh ----------

CREATE TABLE classes (
    id             uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id       uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id     uuid        NOT NULL REFERENCES subjects (id),
    title          text        NOT NULL,
    description    text        NOT NULL DEFAULT '',
    mode           text        NOT NULL DEFAULT 'self_paced',  -- self_paced (tự học) | live (có buổi live)
    visibility     text        NOT NULL DEFAULT 'private',     -- public | unlisted | private
    enrollment     text        NOT NULL DEFAULT 'approval',    -- open | approval | invite
    invite_code    text        NOT NULL,
    copied_from    uuid REFERENCES classes (id) ON DELETE SET NULL,
    path_updated_at timestamptz,                           -- lần sửa bản nháp lộ trình gần nhất
    archived_at    timestamptz,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT classes_mode_check CHECK (mode IN ('self_paced', 'live')),
    CONSTRAINT classes_visibility_check CHECK (visibility IN ('public', 'unlisted', 'private')),
    CONSTRAINT classes_enrollment_check CHECK (enrollment IN ('open', 'approval', 'invite'))
);
CREATE UNIQUE INDEX classes_invite_code_key ON classes (invite_code);
CREATE INDEX classes_owner_idx ON classes (owner_id);
CREATE INDEX classes_public_idx ON classes (subject_id) WHERE visibility = 'public' AND archived_at IS NULL;

CREATE TABLE class_grades (
    class_id uuid NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    grade_id uuid NOT NULL REFERENCES grades (id) ON DELETE CASCADE,
    PRIMARY KEY (class_id, grade_id)
);

CREATE TABLE class_tags (
    class_id uuid NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    tag_id   uuid NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (class_id, tag_id)
);

-- Thành viên + ghi danh: giáo viên chủ lớp (owner), trợ giảng (sau), học sinh.
CREATE TABLE class_members (
    class_id   uuid        NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role       text        NOT NULL DEFAULT 'student',  -- owner | assistant | student
    status     text        NOT NULL DEFAULT 'active',   -- pending | active | rejected | left | removed
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (class_id, user_id),
    CONSTRAINT class_members_role_check CHECK (role IN ('owner', 'assistant', 'student')),
    CONSTRAINT class_members_status_check CHECK (status IN ('pending', 'active', 'rejected', 'left', 'removed'))
);
CREATE INDEX class_members_user_idx ON class_members (user_id) WHERE status = 'active';

-- ---------- M9: Lộ trình + tiến độ ----------

-- Bản nháp lộ trình chính của lớp học.
CREATE TABLE path_steps (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    class_id   uuid        NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    position   int         NOT NULL,
    kind       text        NOT NULL,                  -- milestone | lesson | exam
    title      text        NOT NULL DEFAULT '',       -- tên mốc; với bài / đề để trống = dùng tên bài / đề
    lesson_id  uuid REFERENCES lessons (id),
    exam_id    uuid,                                  -- FK thêm ở migration đề
    unlock     text        NOT NULL DEFAULT 'free',   -- free | previous
    pass_score double precision,                         -- bước đề: điểm tối thiểu để tính là xong
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT path_steps_kind_check CHECK (kind IN ('milestone', 'lesson', 'exam')),
    CONSTRAINT path_steps_unlock_check CHECK (unlock IN ('free', 'previous'))
);
CREATE INDEX path_steps_class_idx ON path_steps (class_id, position);

-- Ảnh chụp lộ trình mỗi lần xuất bản; học sinh đọc phiên bản mới nhất.
CREATE TABLE path_versions (
    class_id     uuid        NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    version      int         NOT NULL,
    steps        jsonb       NOT NULL,
    published_by uuid        NOT NULL REFERENCES users (id),
    published_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (class_id, version)
);

-- Học sinh đánh dấu đã học xong một bước bài học (bước đề tính từ lượt làm bài).
CREATE TABLE step_progress (
    user_id      uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    class_id     uuid        NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    step_id      uuid        NOT NULL,   -- không FK: bước bị xóa khỏi lộ trình vẫn giữ lịch sử học
    completed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, class_id, step_id)
);
