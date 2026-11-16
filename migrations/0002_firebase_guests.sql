-- Environment comes from the configured verifier, never a client-selected project.
CREATE TABLE firebase_guests (
    project TEXT NOT NULL,
    subject TEXT NOT NULL,
    account_id UUID NOT NULL UNIQUE REFERENCES accounts(id) ON DELETE CASCADE,
    PRIMARY KEY(project, subject)
);
-- Shared guest budgets prevent identity churn from bypassing the service ceiling.
CREATE TABLE guest_budget (
    id BOOLEAN PRIMARY KEY DEFAULT true CHECK(id),
    minute_start TIMESTAMPTZ NOT NULL DEFAULT date_trunc('minute',now()),
    minute_count BIGINT NOT NULL DEFAULT 0,
    day_start DATE NOT NULL DEFAULT (now() AT TIME ZONE 'UTC')::date,
    day_count BIGINT NOT NULL DEFAULT 0,
    identities BIGINT NOT NULL DEFAULT 0
);
INSERT INTO guest_budget(id) VALUES(true);
