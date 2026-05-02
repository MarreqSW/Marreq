-- Server-side session table replacing the cookie-as-user-id scheme.
--
-- The cookie carries a 256-bit base64url random token; we store the SHA-256
-- of that token here (so a leaked database row cannot be replayed as a
-- cookie).  Sessions are revocable per-row and have a hard server-enforced
-- `expires_at` that the cookie cannot extend.
CREATE TABLE sessions (
    token_hash   CHAR(64)     PRIMARY KEY,
    user_id      INT          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   TIMESTAMP    NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMP    NOT NULL,
    last_seen_at TIMESTAMP    NOT NULL DEFAULT NOW(),
    user_agent   TEXT,
    ip_addr      TEXT
);

CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);
