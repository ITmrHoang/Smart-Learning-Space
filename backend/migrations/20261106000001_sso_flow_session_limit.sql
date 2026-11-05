-- M6.2: luồng OIDC thật + giới hạn số thiết bị.

-- Lượt đăng nhập SSO đang dở (từ lúc chuyển sang Google / Microsoft tới lúc quay về callback).
-- Dùng một lần: callback xóa bản ghi ngay khi đọc. `state` chỉ lưu hash vì nằm trên URL.
CREATE TABLE sso_login_states (
    state_hash    text PRIMARY KEY,
    provider      text        NOT NULL,
    nonce         text        NOT NULL,
    code_verifier text        NOT NULL,
    -- Có giá trị khi người dùng đang đăng nhập bấm "Liên kết" trong Hồ sơ.
    link_user_id  uuid REFERENCES users (id) ON DELETE CASCADE,
    next_path     text,
    expires_at    timestamptz NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX sso_login_states_expires_idx ON sso_login_states (expires_at);

-- Lý do thu hồi refresh token, để báo đúng cho thiết bị bị đăng xuất (`session_limit`: vượt số thiết bị).
ALTER TABLE refresh_tokens ADD COLUMN revoked_reason text;
