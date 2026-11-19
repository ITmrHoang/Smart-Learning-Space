-- Đợt 9: câu tự luận + chấm tay (M38–M39), nhận xét mẫu, mã đề (M40), bộ đề (M41).

-- ---------- M38: câu tự luận ----------

-- Nội dung phiên bản (question_versions.body) thêm: rubric [{id, content, points}], tolerance, matchMode.
ALTER TABLE questions DROP CONSTRAINT questions_kind_check;
ALTER TABLE questions ADD CONSTRAINT questions_kind_check
    CHECK (kind IN ('single', 'multiple', 'true_false', 'short', 'essay'));

-- Điểm riêng của câu cố định trong đề (NULL = điểm mỗi câu của phần).
ALTER TABLE exam_section_questions ADD COLUMN points double precision;

-- ---------- M39: chấm tay ----------

ALTER TABLE attempt_items
    ADD COLUMN manual      boolean NOT NULL DEFAULT false,   -- câu tự luận: giáo viên chấm
    ADD COLUMN marks       jsonb,                            -- { stepId: điểm đạt } theo hướng dẫn chấm
    ADD COLUMN feedback    text    NOT NULL DEFAULT '',
    ADD COLUMN annotations jsonb,                            -- [{ fileId, annotatedFileId }] ảnh bài làm đã vẽ nhận xét
    ADD COLUMN graded_by   uuid REFERENCES users (id) ON DELETE SET NULL,
    ADD COLUMN graded_at   timestamptz;
CREATE INDEX attempt_items_pending_idx ON attempt_items (attempt_id) WHERE manual AND graded_at IS NULL;

-- Nhận xét mẫu của giáo viên (chấm đề tự luận, chấm bài tập).
CREATE TABLE comment_snippets (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id   uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    body       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX comment_snippets_owner_idx ON comment_snippets (owner_id, created_at);

-- ---------- M40: mã đề ----------

-- items: [{ sectionPosition, sectionTitle, scoring, points, questionId, versionId, optionOrder }]
CREATE TABLE exam_papers (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    exam_id    uuid        NOT NULL REFERENCES exams (id) ON DELETE CASCADE,
    position   int         NOT NULL,
    code       text        NOT NULL,
    items      jsonb       NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (exam_id, code)
);
CREATE INDEX exam_papers_exam_idx ON exam_papers (exam_id, position);

-- ---------- M41: bộ đề ----------

CREATE TABLE exam_sets (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id  uuid        NOT NULL REFERENCES subjects (id),
    title       text        NOT NULL,
    description text        NOT NULL DEFAULT '',
    archived_at timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX exam_sets_owner_idx ON exam_sets (owner_id, updated_at DESC) WHERE archived_at IS NULL;

CREATE TABLE exam_set_items (
    set_id   uuid NOT NULL REFERENCES exam_sets (id) ON DELETE CASCADE,
    exam_id  uuid NOT NULL REFERENCES exams (id) ON DELETE CASCADE,
    position int  NOT NULL,
    PRIMARY KEY (set_id, exam_id)
);
