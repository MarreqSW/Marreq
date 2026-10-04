-- Shared login rate-limit records (issue #286): the PostgreSQL
-- RateLimitStore, so failed-login counters and lockouts are shared by every
-- backend replica and survive restarts. One row per rate-limited subject
-- while it has failures or a lock; rows are removed on a successful login
-- and expired by the server's periodic sweep.
CREATE TABLE login_rate_limits (
    scope_kind   varchar(10) NOT NULL CHECK (scope_kind IN ('username', 'ip')),
    scope_key    text        NOT NULL,
    failures     integer     NOT NULL DEFAULT 0 CHECK (failures >= 0),
    locked_until timestamptz,
    updated_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (scope_kind, scope_key)
);

CREATE INDEX idx_login_rate_limits_updated_at ON login_rate_limits (updated_at);
