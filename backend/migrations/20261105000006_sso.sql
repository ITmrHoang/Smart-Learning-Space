-- M6: đăng nhập SSO (Google / Microsoft).

-- Danh tính SSO đã liên kết với tài khoản.
CREATE TABLE user_identities (
    id            uuid PRIMARY KEY DEFAULT uuidv7(),
    user_id       uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider      text        NOT NULL,          -- google | microsoft
    subject       text        NOT NULL,          -- mã người dùng (claim "sub") của nhà cung cấp
    email         text,
    linked_at     timestamptz NOT NULL DEFAULT now(),
    last_login_at timestamptz,
    -- Một danh tính chỉ thuộc một tài khoản.
    CONSTRAINT user_identities_provider_subject_key UNIQUE (provider, subject),
    -- Một tài khoản liên kết tối đa một danh tính cho mỗi nhà cung cấp.
    CONSTRAINT user_identities_user_provider_key UNIQUE (user_id, provider)
);
CREATE INDEX user_identities_user_idx ON user_identities (user_id);

-- Yêu cầu liên kết đang chờ: email SSO trùng một tài khoản đã xác minh → người dùng phải nhập
-- mật khẩu của tài khoản đó, hoặc mã OTP gửi về email, thì mới liên kết.
CREATE TABLE sso_link_requests (
    token_hash   text PRIMARY KEY,
    user_id      uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider     text        NOT NULL,
    subject      text        NOT NULL,
    email        text        NOT NULL,
    attempts     int         NOT NULL DEFAULT 0,
    code_hash    text,
    code_sent_at timestamptz,
    expires_at   timestamptz NOT NULL,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX sso_link_requests_user_idx ON sso_link_requests (user_id);
