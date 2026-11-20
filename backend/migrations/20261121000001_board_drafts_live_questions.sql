-- Đợt 10: bảng dạy Toán WB2.

-- M42: nháp bảng tự lưu trong buổi (giáo viên tải lại trang / đổi máy vẫn mở lại đúng bảng). Mỗi trang một dòng
-- để mỗi lần gửi nhỏ hơn giới hạn 1 MB của API; chỉ trang vừa đổi mới gửi lại.
CREATE TABLE live_board_drafts (
    session_id uuid        NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    page       int         NOT NULL,
    data       jsonb       NOT NULL DEFAULT '[]',
    background text        NOT NULL DEFAULT 'none',   -- nền trang: none | grid | oly | lines | dots | axes
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (session_id, page)
);

-- Bảng đã lưu (ảnh + nét vẽ) giữ cả nền trang.
ALTER TABLE live_board_pages ADD COLUMN background text NOT NULL DEFAULT 'none';

-- M45: câu hỏi lấy từ ngân hàng (một / nhiều đáp án, đúng sai, trả lời ngắn) hỏi nhanh trong phòng.
ALTER TABLE live_polls DROP CONSTRAINT live_polls_kind_check;
ALTER TABLE live_polls ADD CONSTRAINT live_polls_kind_check CHECK (kind IN ('poll', 'quiz', 'question'));
ALTER TABLE live_polls
    ADD COLUMN question_id   uuid REFERENCES questions (id) ON DELETE SET NULL,
    ADD COLUMN question_kind text,                         -- single | multiple | true_false | short
    ADD COLUMN right_count   int NOT NULL DEFAULT 0;       -- số học sinh trả lời đúng
