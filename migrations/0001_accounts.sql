CREATE TABLE accounts (
    id UUID PRIMARY KEY,
    email TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('member','trusted','admin','owner')),
    disabled BOOLEAN NOT NULL DEFAULT false,
    requests_per_minute BIGINT DEFAULT 10 CHECK (requests_per_minute >= 0),
    operations_per_day BIGINT DEFAULT 100 CHECK (operations_per_day >= 0),
    max_batch BIGINT DEFAULT 1 CHECK (max_batch >= 0),
    max_concurrent BIGINT DEFAULT 1 CHECK (max_concurrent >= 0),
    max_keys BIGINT DEFAULT 3 CHECK (max_keys >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX one_owner ON accounts ((role)) WHERE role = 'owner';

CREATE TABLE account_sessions (
    token_hash TEXT PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX sessions_account ON account_sessions(account_id);
CREATE INDEX sessions_expiry ON account_sessions(expires_at);

CREATE TABLE member_api_keys (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    prefix TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);
CREATE INDEX keys_account ON member_api_keys(account_id);

-- Two rolling counters per account, rather than a growing row for every minute.
CREATE TABLE account_usage (
    account_id UUID PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    minute_start TIMESTAMPTZ NOT NULL DEFAULT date_trunc('minute',now()),
    minute_count BIGINT NOT NULL DEFAULT 0,
    day_start DATE NOT NULL DEFAULT (now() AT TIME ZONE 'UTC')::date,
    day_count BIGINT NOT NULL DEFAULT 0,
    total_operations BIGINT NOT NULL DEFAULT 0
);
CREATE TABLE account_work (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + interval '5 minutes'
);
CREATE INDEX work_account ON account_work(account_id, expires_at);

CREATE TABLE auth_throttle (
    bucket TEXT PRIMARY KEY,
    window_start TIMESTAMPTZ NOT NULL DEFAULT date_trunc('minute',now()),
    attempts BIGINT NOT NULL DEFAULT 1
);
CREATE INDEX throttle_expiry ON auth_throttle(window_start);
CREATE TABLE account_audit (
    id BIGSERIAL PRIMARY KEY,
    actor_id UUID REFERENCES accounts(id) ON DELETE SET NULL,
    subject_id UUID REFERENCES accounts(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    details JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
