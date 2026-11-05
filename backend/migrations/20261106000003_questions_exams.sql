-- M10–M11: ngân hàng câu hỏi, đề ôn tập / kiểm tra, lượt làm bài.

-- ---------- M10: Ngân hàng câu hỏi ----------

-- Thư mục chỉ để giáo viên tự sắp xếp câu hỏi (cây nhiều cấp).
CREATE TABLE question_folders (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id   uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    parent_id  uuid REFERENCES question_folders (id) ON DELETE CASCADE,
    name       text        NOT NULL,
    sort_order int         NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX question_folders_owner_idx ON question_folders (owner_id);

CREATE TABLE questions (
    id                 uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id           uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id         uuid        NOT NULL REFERENCES subjects (id),
    grade_id           uuid REFERENCES grades (id),
    folder_id          uuid REFERENCES question_folders (id) ON DELETE SET NULL,
    kind               text        NOT NULL,  -- single | multiple | true_false | short
    level              text        NOT NULL,  -- nhan_biet | thong_hieu | van_dung | van_dung_cao
    current_version_id uuid,
    search_text        text        NOT NULL DEFAULT '',  -- nội dung không dấu, chữ thường (tìm kiếm)
    archived_at        timestamptz,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT questions_kind_check CHECK (kind IN ('single', 'multiple', 'true_false', 'short')),
    CONSTRAINT questions_level_check CHECK (level IN ('nhan_biet', 'thong_hieu', 'van_dung', 'van_dung_cao'))
);
CREATE INDEX questions_owner_idx ON questions (owner_id, subject_id) WHERE archived_at IS NULL;

CREATE TABLE question_tags (
    question_id uuid NOT NULL REFERENCES questions (id) ON DELETE CASCADE,
    tag_id      uuid NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (question_id, tag_id)
);
CREATE INDEX question_tags_tag_idx ON question_tags (tag_id);

-- Mỗi lần sửa nội dung / đáp án tạo một phiên bản (lượt làm bài trỏ tới đúng phiên bản đã làm).
-- body: { content, options: [{id, content}], correct: [optionId], accepted: [string], explanation, shuffleOptions }
CREATE TABLE question_versions (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    question_id uuid        NOT NULL REFERENCES questions (id) ON DELETE CASCADE,
    version     int         NOT NULL,
    body        jsonb       NOT NULL,
    created_by  uuid        NOT NULL REFERENCES users (id),
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (question_id, version)
);
ALTER TABLE questions
    ADD CONSTRAINT questions_current_version_fkey FOREIGN KEY (current_version_id) REFERENCES question_versions (id);

-- ---------- M11: Đề ôn tập / kiểm tra ----------

CREATE TABLE exams (
    id                 uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id           uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id         uuid        NOT NULL REFERENCES subjects (id),
    grade_id           uuid REFERENCES grades (id),
    title              text        NOT NULL,
    description        text        NOT NULL DEFAULT '',
    kind               text        NOT NULL DEFAULT 'practice',  -- practice (ôn tập) | test (kiểm tra)
    time_limit_minutes int,
    max_attempts       int,
    opens_at           timestamptz,
    closes_at          timestamptz,
    shuffle_questions  boolean     NOT NULL DEFAULT true,
    shuffle_options    boolean     NOT NULL DEFAULT true,
    show_answers       text        NOT NULL DEFAULT 'after_submit', -- after_submit | after_close | never
    archived_at        timestamptz,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT exams_kind_check CHECK (kind IN ('practice', 'test')),
    CONSTRAINT exams_show_answers_check CHECK (show_answers IN ('after_submit', 'after_close', 'never'))
);
CREATE INDEX exams_owner_idx ON exams (owner_id) WHERE archived_at IS NULL;

-- Phần I / II / III. scoring: per_question (đúng hết mới có điểm) | thpt_true_false (0,1 / 0,25 / 0,5 / 1).
CREATE TABLE exam_sections (
    id                  uuid PRIMARY KEY DEFAULT uuidv7(),
    exam_id             uuid          NOT NULL REFERENCES exams (id) ON DELETE CASCADE,
    position            int           NOT NULL,
    title               text          NOT NULL,
    scoring             text          NOT NULL DEFAULT 'per_question',
    points_per_question double precision NOT NULL DEFAULT 1,
    CONSTRAINT exam_sections_scoring_check CHECK (scoring IN ('per_question', 'thpt_true_false'))
);
CREATE INDEX exam_sections_exam_idx ON exam_sections (exam_id, position);

-- Câu cố định trong phần.
CREATE TABLE exam_section_questions (
    section_id  uuid NOT NULL REFERENCES exam_sections (id) ON DELETE CASCADE,
    position    int  NOT NULL,
    question_id uuid NOT NULL REFERENCES questions (id),
    PRIMARY KEY (section_id, position)
);

-- Ma trận: rút ngẫu nhiên `count` câu theo danh mục × mức độ (× dạng câu, × thư mục) cho mỗi lượt làm.
CREATE TABLE exam_section_rules (
    section_id uuid NOT NULL REFERENCES exam_sections (id) ON DELETE CASCADE,
    position   int  NOT NULL,
    tag_id     uuid REFERENCES tags (id),
    level      text,
    kind       text,
    folder_id  uuid REFERENCES question_folders (id) ON DELETE SET NULL,
    count      int  NOT NULL,
    PRIMARY KEY (section_id, position)
);

ALTER TABLE path_steps ADD CONSTRAINT path_steps_exam_fkey FOREIGN KEY (exam_id) REFERENCES exams (id);

-- Lượt làm bài. class_id NULL + is_preview = giáo viên làm thử.
CREATE TABLE attempts (
    id           uuid PRIMARY KEY DEFAULT uuidv7(),
    exam_id      uuid          NOT NULL REFERENCES exams (id),
    user_id      uuid          NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    class_id     uuid REFERENCES classes (id) ON DELETE CASCADE,
    step_id      uuid,
    is_preview   boolean       NOT NULL DEFAULT false,
    status       text          NOT NULL DEFAULT 'in_progress',  -- in_progress | submitted
    started_at   timestamptz   NOT NULL DEFAULT now(),
    deadline_at  timestamptz,
    submitted_at timestamptz,
    score        double precision NOT NULL DEFAULT 0,
    max_score    double precision NOT NULL DEFAULT 0,
    CONSTRAINT attempts_status_check CHECK (status IN ('in_progress', 'submitted'))
);
CREATE INDEX attempts_user_idx ON attempts (user_id, exam_id);
CREATE INDEX attempts_class_idx ON attempts (class_id, exam_id) WHERE class_id IS NOT NULL;

-- Từng câu của lượt làm: câu, phiên bản, thứ tự đáp án đã trộn, câu trả lời, điểm.
CREATE TABLE attempt_items (
    attempt_id          uuid          NOT NULL REFERENCES attempts (id) ON DELETE CASCADE,
    position            int           NOT NULL,
    section_position    int           NOT NULL,
    section_title       text          NOT NULL,
    scoring             text          NOT NULL,
    points              double precision NOT NULL,
    question_id         uuid          NOT NULL REFERENCES questions (id),
    question_version_id uuid          NOT NULL REFERENCES question_versions (id),
    option_order        uuid[]        NOT NULL DEFAULT '{}',
    answer              jsonb,
    checked             boolean       NOT NULL DEFAULT false,  -- ôn tập: đã bấm "Kiểm tra" (khóa câu trả lời)
    is_correct          boolean,
    score               double precision NOT NULL DEFAULT 0,
    PRIMARY KEY (attempt_id, position)
);
CREATE INDEX attempt_items_question_idx ON attempt_items (question_id);
