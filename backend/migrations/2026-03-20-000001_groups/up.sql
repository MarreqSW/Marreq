-- Groups: top-level container for organizing related projects.
CREATE TABLE groups (
    id          SERIAL PRIMARY KEY,
    name        VARCHAR(255) NOT NULL,
    slug        VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    owner_id    INTEGER REFERENCES users(id),
    created_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Group membership with role-based access (mirrors project_members pattern).
-- Roles: 1 = Owner, 2 = Maintainer, 3 = Contributor, 4 = Viewer
CREATE TABLE group_members (
    group_id    INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role        INTEGER NOT NULL DEFAULT 4,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, user_id)
);

-- Every project belongs to exactly one group.
ALTER TABLE projects ADD COLUMN group_id INTEGER REFERENCES groups(id);

-- Index for efficient project lookups by group.
CREATE INDEX idx_projects_group_id ON projects(group_id);
CREATE INDEX idx_group_members_user_id ON group_members(user_id);
