-- M5: hồ sơ đăng ký giáo viên và môn được dạy.
CREATE TYPE application_status AS ENUM ('pending', 'approved', 'rejected');

CREATE TABLE teacher_applications (
    id            uuid PRIMARY KEY DEFAULT uuidv7(),
    user_id       uuid               NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_ids   uuid[]             NOT NULL,
    grade_ids     uuid[]             NOT NULL DEFAULT '{}',
    bio           text               NOT NULL,
    experience    text               NOT NULL DEFAULT '',
    status        application_status NOT NULL DEFAULT 'pending',
    reviewed_by   uuid REFERENCES users (id) ON DELETE SET NULL,
    reviewed_at   timestamptz,
    reject_reason text,
    created_at    timestamptz        NOT NULL DEFAULT now()
);
-- Mỗi người chỉ có tối đa một hồ sơ đang chờ duyệt.
CREATE UNIQUE INDEX teacher_applications_one_pending_key ON teacher_applications (user_id) WHERE status = 'pending';
CREATE INDEX teacher_applications_status_idx ON teacher_applications (status, created_at);

CREATE TABLE teacher_subjects (
    teacher_id uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    subject_id uuid        NOT NULL REFERENCES subjects (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (teacher_id, subject_id)
);
