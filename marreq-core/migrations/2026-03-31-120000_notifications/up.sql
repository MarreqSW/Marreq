CREATE TABLE notifications (
    id          SERIAL PRIMARY KEY,
    user_id     INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id  INT REFERENCES projects(id) ON DELETE CASCADE,
    notification_type VARCHAR(50) NOT NULL,
    title       VARCHAR(255) NOT NULL,
    body        TEXT,
    entity_type VARCHAR(50),
    entity_id   INT,
    actor_id    INT REFERENCES users(id) ON DELETE SET NULL,
    read        BOOLEAN NOT NULL DEFAULT FALSE,
    emailed     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user_read_created
    ON notifications (user_id, read, created_at DESC);

CREATE INDEX idx_notifications_dedup
    ON notifications (user_id, notification_type, entity_id);

CREATE TABLE notification_preferences (
    id          SERIAL PRIMARY KEY,
    user_id     INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    project_id  INT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    notify_in_app BOOLEAN NOT NULL DEFAULT TRUE,
    notify_email  BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(user_id, project_id)
);
