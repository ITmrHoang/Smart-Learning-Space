-- Đợt 8: giáo án (M34) + tài liệu đính kèm bài học / giáo án (M37).

-- ---------- M34: Giáo án theo mẫu ----------
-- Giáo án là tài liệu của giáo viên (khác bài học học sinh xem): danh sách khối theo mẫu
-- CV 5512 (THCS / THPT), CV 2345 (tiểu học), dạy thêm, hoặc tự do. Khối lưu jsonb, kiểm tra ở api.
CREATE TABLE lesson_plans (
    id           uuid PRIMARY KEY DEFAULT uuidv7(),
    owner_id     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id   uuid        NOT NULL REFERENCES subjects (id),
    grade_id     uuid REFERENCES grades (id),
    title        text        NOT NULL,
    template     text        NOT NULL,
    info         jsonb       NOT NULL DEFAULT '{}',   -- trường, tổ, số tiết, ngày soạn, ngày dạy, lớp dạy…
    blocks       jsonb       NOT NULL DEFAULT '[]',
    lesson_id    uuid REFERENCES lessons (id) ON DELETE SET NULL,  -- bài học cho học sinh đi kèm
    attachments  jsonb       NOT NULL DEFAULT '[]',   -- [{ fileId, name }] (M37)
    copied_from  uuid REFERENCES lesson_plans (id) ON DELETE SET NULL,
    archived_at  timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT lesson_plans_template_check CHECK (template IN ('cv5512', 'cv2345', 'day_them', 'blank'))
);
CREATE INDEX lesson_plans_owner_idx ON lesson_plans (owner_id, updated_at DESC) WHERE archived_at IS NULL;

-- ---------- M37: Tài liệu đính kèm (PDF / Word / PowerPoint / Excel) ----------
ALTER TABLE files DROP CONSTRAINT files_purpose_check;
ALTER TABLE files ADD CONSTRAINT files_purpose_check
    CHECK (purpose IN ('content', 'homework', 'import', 'attachment'));

-- Tài liệu đi theo lần xuất bản: học sinh thấy tài liệu của bản đã xuất bản.
ALTER TABLE lessons ADD COLUMN attachments jsonb NOT NULL DEFAULT '[]';
ALTER TABLE lesson_revisions ADD COLUMN attachments jsonb NOT NULL DEFAULT '[]';
