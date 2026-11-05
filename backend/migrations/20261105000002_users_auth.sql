-- M1: tài khoản và phiên đăng nhập.
CREATE TYPE user_role AS ENUM ('admin', 'teacher', 'student');
CREATE TYPE user_status AS ENUM ('pending_verification', 'active', 'locked');

CREATE TABLE users (
    id                uuid PRIMARY KEY DEFAULT uuidv7(),
    username          text        NOT NULL,
    email             text        NOT NULL,
    full_name         text        NOT NULL,
    password_hash     text,                       -- NULL: tài khoản chỉ đăng nhập bằng SSO
    role              user_role   NOT NULL DEFAULT 'student',
    status            user_status NOT NULL DEFAULT 'pending_verification',
    email_verified_at timestamptz,
    last_login_at     timestamptz,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now()
);
-- Không phân biệt hoa thường khi kiểm tra trùng.
CREATE UNIQUE INDEX users_username_key ON users (lower(username));
CREATE UNIQUE INDEX users_email_key ON users (lower(email));
CREATE INDEX users_role_status_idx ON users (role, status);
-- Tìm kiếm gần đúng theo username / email / họ tên (màn hình admin).
CREATE INDEX users_search_idx ON users USING gin ((lower(username || ' ' || email || ' ' || full_name)) gin_trgm_ops);

-- Mã OTP xác minh email (mỗi user tối đa 1 mã đang hiệu lực).
CREATE TABLE email_verifications (
    user_id    uuid PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    code_hash  text        NOT NULL,
    attempts   int         NOT NULL DEFAULT 0,
    expires_at timestamptz NOT NULL,
    sent_at    timestamptz NOT NULL DEFAULT now()
);

-- Refresh token: chỉ lưu hash. Mỗi lần đăng nhập mở một "chuỗi phiên" (family_id);
-- mỗi lần refresh thu hồi token cũ và cấp token mới cùng family.
CREATE TABLE refresh_tokens (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    user_id     uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    family_id   uuid        NOT NULL,
    token_hash  text        NOT NULL UNIQUE,
    expires_at  timestamptz NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    revoked_at  timestamptz,
    replaced_by uuid,
    user_agent  text,
    ip          text
);
CREATE INDEX refresh_tokens_user_active_idx ON refresh_tokens (user_id) WHERE revoked_at IS NULL;
CREATE INDEX refresh_tokens_family_idx ON refresh_tokens (family_id);

-- Link đặt lại mật khẩu (dùng một lần).
CREATE TABLE password_resets (
    token_hash text PRIMARY KEY,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at timestamptz NOT NULL,
    used_at    timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX password_resets_user_idx ON password_resets (user_id);
