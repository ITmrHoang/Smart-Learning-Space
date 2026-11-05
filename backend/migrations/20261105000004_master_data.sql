-- M4: master data — môn học, lớp (Lớp 1–12 + lớp riêng), danh mục (nhãn trong môn).
CREATE TABLE subjects (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    name       text        NOT NULL,
    slug       text        NOT NULL,
    sort_order int         NOT NULL DEFAULT 0,
    is_active  boolean     NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX subjects_slug_key ON subjects (slug);

-- owner_id NULL = dùng chung; khác NULL = lớp riêng của một giáo viên (chỉ người đó thấy).
-- merged_into khác NULL = đã gộp vào mục khác, không còn hiện.
CREATE TABLE grades (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    name        text        NOT NULL,
    slug        text        NOT NULL,
    sort_order  int         NOT NULL DEFAULT 0,
    is_system   boolean     NOT NULL DEFAULT false,   -- Lớp 1–12: không sửa, không ẩn
    owner_id    uuid REFERENCES users (id) ON DELETE CASCADE,
    merged_into uuid REFERENCES grades (id),
    is_active   boolean     NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX grades_shared_slug_key ON grades (slug) WHERE owner_id IS NULL AND merged_into IS NULL;
CREATE UNIQUE INDEX grades_owner_slug_key ON grades (owner_id, slug) WHERE owner_id IS NOT NULL AND merged_into IS NULL;

CREATE TABLE tags (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    subject_id  uuid        NOT NULL REFERENCES subjects (id) ON DELETE CASCADE,
    name        text        NOT NULL,
    slug        text        NOT NULL,
    sort_order  int         NOT NULL DEFAULT 0,
    owner_id    uuid REFERENCES users (id) ON DELETE CASCADE,
    merged_into uuid REFERENCES tags (id),
    is_active   boolean     NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX tags_shared_slug_key ON tags (subject_id, slug) WHERE owner_id IS NULL AND merged_into IS NULL;
CREATE UNIQUE INDEX tags_owner_slug_key ON tags (subject_id, owner_id, slug) WHERE owner_id IS NOT NULL AND merged_into IS NULL;
CREATE INDEX tags_subject_idx ON tags (subject_id);

-- Dữ liệu ban đầu.
INSERT INTO grades (name, slug, sort_order, is_system)
SELECT 'Lớp ' || n, 'lop-' || n, n, true FROM generate_series(1, 12) AS n;

INSERT INTO subjects (name, slug, sort_order) VALUES
    ('Toán', 'toan', 1),
    ('Tiếng Việt', 'tieng-viet', 2),
    ('Ngữ văn', 'ngu-van', 3),
    ('Tiếng Anh', 'tieng-anh', 4),
    ('Khoa học tự nhiên', 'khoa-hoc-tu-nhien', 5),
    ('Vật lý', 'vat-ly', 6),
    ('Hóa học', 'hoa-hoc', 7),
    ('Sinh học', 'sinh-hoc', 8),
    ('Lịch sử', 'lich-su', 9),
    ('Địa lý', 'dia-ly', 10),
    ('Tin học', 'tin-hoc', 11),
    ('Giáo dục kinh tế và pháp luật', 'giao-duc-kinh-te-va-phap-luat', 12);
