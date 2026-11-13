-- Đợt 4: hỏi đáp theo bài (M18), thẻ ghi nhớ (M19), phụ huynh + báo cáo (M20).
-- Báo cáo lớp học (M17) chỉ đọc dữ liệu có sẵn, không cần bảng mới.

-- ---------- M18: Hỏi đáp ----------

-- Câu hỏi của học sinh trong lớp học, có thể gắn với một bước (bài / đề / thẻ) của lộ trình.
CREATE TABLE class_threads (
    id               uuid PRIMARY KEY DEFAULT uuidv7(),
    class_id         uuid        NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    step_id          uuid,       -- không có FK: bước có thể bị xóa khỏi lộ trình, câu hỏi vẫn giữ
    author_id        uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    title            text        NOT NULL,
    body             text        NOT NULL,
    pinned           boolean     NOT NULL DEFAULT false,
    accepted_post_id uuid,
    hidden_at        timestamptz,
    hidden_by        uuid REFERENCES users (id) ON DELETE SET NULL,
    post_count       int         NOT NULL DEFAULT 0,
    last_post_at     timestamptz NOT NULL DEFAULT now(),
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX class_threads_class_idx ON class_threads (class_id, pinned DESC, last_post_at DESC);

CREATE TABLE thread_posts (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    thread_id  uuid        NOT NULL REFERENCES class_threads (id) ON DELETE CASCADE,
    author_id  uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    body       text        NOT NULL,
    hidden_at  timestamptz,
    hidden_by  uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX thread_posts_thread_idx ON thread_posts (thread_id, created_at);
ALTER TABLE class_threads
    ADD CONSTRAINT class_threads_accepted_fkey FOREIGN KEY (accepted_post_id) REFERENCES thread_posts (id) ON DELETE SET NULL;

-- Báo vi phạm (D7): mỗi người báo một nội dung một lần; giáo viên chủ lớp xử lý.
CREATE TABLE content_reports (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    class_id    uuid        NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    target_type text        NOT NULL,   -- thread | post
    target_id   uuid        NOT NULL,
    reporter_id uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    reason      text        NOT NULL,
    status      text        NOT NULL DEFAULT 'open',   -- open | hidden | dismissed
    resolved_by uuid REFERENCES users (id) ON DELETE SET NULL,
    resolved_at timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT content_reports_target_check CHECK (target_type IN ('thread', 'post')),
    CONSTRAINT content_reports_status_check CHECK (status IN ('open', 'hidden', 'dismissed')),
    UNIQUE (target_type, target_id, reporter_id)
);
CREATE INDEX content_reports_class_idx ON content_reports (class_id, status, created_at DESC);

-- ---------- M19: Thẻ ghi nhớ ----------

CREATE TABLE decks (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id  uuid        NOT NULL REFERENCES subjects (id),
    title       text        NOT NULL,
    description text        NOT NULL DEFAULT '',
    archived_at timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX decks_owner_idx ON decks (owner_id) WHERE archived_at IS NULL;

-- Mặt trước / mặt sau: Markdown + LaTeX. ID thẻ giữ nguyên khi sửa → không mất lịch ôn của học sinh.
CREATE TABLE cards (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    deck_id    uuid        NOT NULL REFERENCES decks (id) ON DELETE CASCADE,
    position   int         NOT NULL,
    front      text        NOT NULL,
    back       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX cards_deck_idx ON cards (deck_id, position);

-- Lịch ôn của từng học sinh với từng thẻ (lặp lại ngắt quãng, kiểu SM-2).
CREATE TABLE card_reviews (
    user_id       uuid             NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    card_id       uuid             NOT NULL REFERENCES cards (id) ON DELETE CASCADE,
    deck_id       uuid             NOT NULL REFERENCES decks (id) ON DELETE CASCADE,
    repetitions   int              NOT NULL DEFAULT 0,
    interval_days int              NOT NULL DEFAULT 0,
    ease          double precision NOT NULL DEFAULT 2.5,
    lapses        int              NOT NULL DEFAULT 0,
    review_count  int              NOT NULL DEFAULT 0,
    last_rating   text             NOT NULL,
    due_at        timestamptz      NOT NULL,
    reviewed_at   timestamptz      NOT NULL,
    PRIMARY KEY (user_id, card_id)
);
CREATE INDEX card_reviews_due_idx ON card_reviews (user_id, deck_id, due_at);

-- Nhật ký từng lần ôn (thống kê, báo cáo phụ huynh).
CREATE TABLE card_review_log (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    user_id     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    deck_id     uuid        NOT NULL REFERENCES decks (id) ON DELETE CASCADE,
    card_id     uuid        NOT NULL REFERENCES cards (id) ON DELETE CASCADE,
    rating      text        NOT NULL,
    reviewed_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX card_review_log_user_idx ON card_review_log (user_id, reviewed_at);

ALTER TABLE path_steps DROP CONSTRAINT path_steps_kind_check;
ALTER TABLE path_steps ADD CONSTRAINT path_steps_kind_check CHECK (kind IN ('milestone', 'lesson', 'exam', 'session', 'deck'));
ALTER TABLE path_steps ADD COLUMN deck_id uuid REFERENCES decks (id) ON DELETE SET NULL;

-- ---------- M20: Phụ huynh + báo cáo ----------

-- Không có tài khoản phụ huynh: chỉ lưu liên hệ để gửi báo cáo. Phụ huynh phải bấm link xác nhận
-- (xác minh email + bằng chứng đồng ý xử lý dữ liệu của học sinh chưa thành niên).
CREATE TABLE student_guardians (
    id                 uuid PRIMARY KEY DEFAULT uuidv7(),
    student_id         uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    full_name          text        NOT NULL,
    relation           text        NOT NULL DEFAULT '',
    email              text        NOT NULL,   -- chữ thường
    phone              text        NOT NULL DEFAULT '',
    consent_status     text        NOT NULL DEFAULT 'pending',   -- pending | confirmed | declined
    consent_token_hash text,
    consent_sent_at    timestamptz,
    responded_at       timestamptz,
    receive_reports    boolean     NOT NULL DEFAULT true,
    unsubscribe_token  text        NOT NULL UNIQUE,   -- chỉ dùng để hủy nhận báo cáo
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT student_guardians_status_check CHECK (consent_status IN ('pending', 'confirmed', 'declined')),
    UNIQUE (student_id, email)
);
CREATE UNIQUE INDEX student_guardians_consent_idx ON student_guardians (consent_token_hash) WHERE consent_token_hash IS NOT NULL;

-- Lịch sử gửi báo cáo. Báo cáo tuần: mỗi phụ huynh một lần mỗi tuần (chỉ mục duy nhất → chạy nhiều instance không gửi trùng).
CREATE TABLE report_deliveries (
    id           uuid PRIMARY KEY DEFAULT uuidv7(),
    guardian_id  uuid        NOT NULL REFERENCES student_guardians (id) ON DELETE CASCADE,
    student_id   uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind         text        NOT NULL,   -- weekly | manual
    period_start date        NOT NULL,
    period_end   date        NOT NULL,
    class_id     uuid REFERENCES classes (id) ON DELETE SET NULL,
    sent_by      uuid REFERENCES users (id) ON DELETE SET NULL,
    note         text        NOT NULL DEFAULT '',
    created_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT report_deliveries_kind_check CHECK (kind IN ('weekly', 'manual'))
);
CREATE UNIQUE INDEX report_deliveries_weekly_uniq ON report_deliveries (guardian_id, period_start) WHERE kind = 'weekly';
CREATE INDEX report_deliveries_student_idx ON report_deliveries (student_id, created_at DESC);
