-- M3: nhật ký thao tác quản trị.
CREATE TABLE audit_logs (
    id          uuid PRIMARY KEY DEFAULT uuidv7(),
    actor_id    uuid REFERENCES users (id) ON DELETE SET NULL,
    action      text        NOT NULL,          -- ví dụ: user.lock, grade.merge
    target_type text        NOT NULL,          -- ví dụ: user, grade, tag
    target_id   uuid,
    data        jsonb       NOT NULL DEFAULT '{}'::jsonb,
    ip          text,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX audit_logs_created_idx ON audit_logs (created_at DESC);
CREATE INDEX audit_logs_target_idx ON audit_logs (target_type, target_id);
