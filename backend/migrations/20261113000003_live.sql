-- Đợt 6: dạy trực tiếp trong ứng dụng (G5) — phòng LiveKit (M26), tương tác (M27), bảng thông minh WB1 (M28),
-- buổi mở cho khách có phòng chờ (M29).

-- Phòng học của buổi: external = link họp ngoài (Google Meet / Zoom, như đợt 3) | live = phòng của hệ thống (LiveKit).
ALTER TABLE sessions ADD COLUMN room text NOT NULL DEFAULT 'external';
ALTER TABLE sessions ADD CONSTRAINT sessions_room_check CHECK (room IN ('external', 'live'));

-- Mức truy cập thứ tư: khách vào bằng link (analyst.md mục 4.3).
ALTER TABLE sessions DROP CONSTRAINT sessions_access_check;
ALTER TABLE sessions ADD CONSTRAINT sessions_access_check
    CHECK (access IN ('invited', 'members', 'logged_in', 'public_link'));
-- Link khách: chỉ lưu hash của khóa trong link; đổi link = tạo khóa mới.
ALTER TABLE sessions ADD COLUMN guest_key_hash text;
ALTER TABLE sessions ADD COLUMN guest_limit int NOT NULL DEFAULT 30;
-- Giáo viên mở / kết thúc phòng (học sinh chỉ vào được khi giáo viên đã mở hoặc tới giờ).
ALTER TABLE sessions ADD COLUMN live_started_at timestamptz;
ALTER TABLE sessions ADD COLUMN live_ended_at timestamptz;

-- Khách vào phòng chờ; giáo viên duyệt mới nhận được quyền vào phòng (chỉ xem và nghe).
CREATE TABLE live_guests (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    session_id  uuid        NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    name        text        NOT NULL,
    token_hash  text        NOT NULL,              -- khóa riêng của khách (hỏi trạng thái, lấy token phòng)
    status      text        NOT NULL DEFAULT 'waiting',  -- waiting | admitted | denied | removed
    ip          text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    decided_at  timestamptz,
    decided_by  uuid REFERENCES users (id) ON DELETE SET NULL,
    CONSTRAINT live_guests_status_check CHECK (status IN ('waiting', 'admitted', 'denied', 'removed'))
);
CREATE INDEX live_guests_session_idx ON live_guests (session_id, created_at);

-- Bình chọn / quiz nhanh trong buổi: lưu câu hỏi + kết quả khi giáo viên đóng.
CREATE TABLE live_polls (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    session_id uuid        NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    kind       text        NOT NULL,              -- poll (bình chọn) | quiz (có đáp án đúng)
    question   text        NOT NULL,
    options    jsonb       NOT NULL,              -- ["A", "B", …]
    correct    int[]       NOT NULL DEFAULT '{}', -- vị trí đáp án đúng (quiz)
    counts     int[]       NOT NULL,              -- số phiếu từng phương án
    voters     int         NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT live_polls_kind_check CHECK (kind IN ('poll', 'quiz'))
);
CREATE INDEX live_polls_session_idx ON live_polls (session_id, created_at);

-- Bảng của buổi học: mỗi trang lưu thành ảnh (Garage) + dữ liệu nét vẽ để mở lại; học sinh xem lại sau buổi.
CREATE TABLE live_board_pages (
    session_id uuid        NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    page       int         NOT NULL,
    file_id    uuid        NOT NULL REFERENCES files (id),
    data       jsonb       NOT NULL DEFAULT '[]',
    saved_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (session_id, page)
);

-- Lượt lên bảng (giáo viên mời học sinh lên bảng) — tính điểm phát biểu trong báo cáo.
CREATE TABLE live_board_turns (
    id         uuid PRIMARY KEY DEFAULT uuidv7(),
    session_id uuid        NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    started_at timestamptz NOT NULL DEFAULT now(),
    ended_at   timestamptz
);
CREATE INDEX live_board_turns_session_idx ON live_board_turns (session_id, user_id);
