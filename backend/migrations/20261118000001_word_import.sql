-- Đợt 7 (M31): nhập câu hỏi từ Word.
-- File .docx upload lên Garage với mục đích `import` (API đọc một lần rồi xóa); ảnh trong đề lưu thành ảnh nội dung,
-- trùng nội dung (SHA-256) thì dùng lại file cũ — xem trước nhiều lần không sinh thêm ảnh.

ALTER TABLE files DROP CONSTRAINT files_purpose_check;
ALTER TABLE files ADD CONSTRAINT files_purpose_check CHECK (purpose IN ('content', 'homework', 'import'));

ALTER TABLE files ADD COLUMN sha256 text;
CREATE INDEX files_owner_sha_idx ON files (owner_id, purpose, sha256) WHERE sha256 IS NOT NULL;
