-- =============================================================================
-- Marreq deployment-mode foundation: workspaces + email verification + email tokens
-- =============================================================================
-- This migration is a no-op in behavior for existing Server-mode deployments:
--   * `users.email_verified` defaults to TRUE so all existing rows remain able to log in.
--   * `workspaces` is a new table; no existing table gains a foreign key to it.
--   * `email_tokens` is a new table only used by Cloud-mode flows.
-- Cloud-mode bootstrap creates a personal workspace for each user on registration.
-- =============================================================================

ALTER TABLE users
    ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT TRUE;

CREATE TABLE workspaces (
    id            SERIAL PRIMARY KEY,
    slug          VARCHAR(255) NOT NULL UNIQUE,
    name          VARCHAR(255) NOT NULL,
    owner_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind          VARCHAR(32)  NOT NULL DEFAULT 'personal',
    created_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT workspaces_kind_check CHECK (kind IN ('personal', 'shared'))
);

CREATE INDEX workspaces_owner_user_id_idx ON workspaces(owner_user_id);

-- Each user may own at most one personal workspace; shared workspaces are unlimited.
CREATE UNIQUE INDEX workspaces_one_personal_per_user_idx
    ON workspaces(owner_user_id)
    WHERE kind = 'personal';

SELECT diesel_manage_updated_at('workspaces');

CREATE TABLE email_tokens (
    id          SERIAL PRIMARY KEY,
    user_id     INTEGER     NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  VARCHAR(128) NOT NULL UNIQUE,
    purpose     VARCHAR(32)  NOT NULL,
    expires_at  TIMESTAMP    NOT NULL,
    used_at     TIMESTAMP    NULL,
    created_at  TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT email_tokens_purpose_check CHECK (purpose IN ('verify_email', 'reset_password'))
);

CREATE INDEX email_tokens_user_id_idx  ON email_tokens(user_id);
CREATE INDEX email_tokens_purpose_idx  ON email_tokens(purpose);
