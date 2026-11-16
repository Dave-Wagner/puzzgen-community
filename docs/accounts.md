# Member accounts, API keys, and resource allocations

Puzzgen remains a free learning resource. A member-enabled instance uses PostgreSQL to provide persistent accounts, API keys, shared quotas, and administration. The main site and account page remain Rust/Leptos; there is no hosted authentication dependency.

## Enable on your VPS

Use a dedicated database and application role on your existing PostgreSQL server. The current build connects over loopback or a Unix socket; it intentionally refuses a remote plaintext database connection. Use an SSH tunnel for remote development. Public HTTPS terminates at your existing reverse proxy.

Set process environment variables (or the systemd environment file):

```text
DATABASE_URL=postgres://puzzgen:YOUR_DATABASE_PASSWORD@127.0.0.1:5432/puzzgen
PUZZGEN_PUBLIC_ORIGIN=https://YOUR_PUZZLE_HOST
PUZZGEN_REGISTRATION_OPEN=true
```

`PUZZGEN_PUBLIC_ORIGIN` is the exact canonical browser origin, without a path. Account writes require that Origin and a session CSRF token. For development through Trunk use `http://127.0.0.1:8081`; for Axum directly use `http://127.0.0.1:8080`. Do not switch between localhost and 127.0.0.1 midway through a session.

Apply the additive migration explicitly, then create the protected owner:

```powershell
cargo run --locked -p puzzgen -- migrate
cargo run --locked -p puzzgen -- create-admin
cargo run --locked -p puzzgen
```

The owner command prompts for email, display name, and a confirmed passphrase; password input is hidden. It creates exactly one owner and never promotes the first public registration. The member server refuses to start before an owner exists. Use your own passphrase; no production default owner credential is shipped. Startup does not silently apply migrations.

Open `/account` to sign in. The owner account initially has unlimited account quotas and can set limits for itself. Keep the origin on HTTPS for production: sessions then use a Secure, HttpOnly, SameSite=Lax `__Host-` cookie. Plain HTTP cookies are supported only for local development.

Without `DATABASE_URL`, accounts are shown as unconfigured and the previous public/static-key deployment mode remains available. With `DATABASE_URL`, generation/verification require a member key or a signed-in browser session; legacy environment keys cannot bypass account controls. Documentation, catalog, health, and offline learning remain public. Metrics require an administrator's browser session in member mode.

## Roles and limits

| Role | Account capabilities | Administration |
| --- | --- | --- |
| Member | Profile, password, sessions, own keys, account deletion | None |
| Trusted | Same member capabilities with an allocation granted by an admin | None |
| Admin | Same member capabilities | Set quotas and disable/revoke ordinary members; edit own quotas |
| Owner | Protected main administrator | Manage all allocations and assign member/trusted/admin roles |

Only the owner can assign roles. An admin cannot modify the owner or another admin. The owner cannot be demoted, disabled, deleted, or created through the public API. All permissions are checked server-side against current database records, not just hidden UI controls. The owner is created by an operator with access to the application's database environment.

Regular registrations start with:

| Resource | Default |
| --- | --- |
| Active keys | 3 |
| Compute requests per minute | 10 |
| Puzzle operations per UTC day | 100 |
| Puzzles per batch | 1 |
| Concurrent compute operations | 1 |

The admin form accepts nonnegative whole numbers; a blank input maps to JSON `null` for unlimited. Zero disables the resource. Allocations up to 2,147,483,647 or unlimited are supported. Changing a role does not implicitly change its allocation: explicitly set the desired limits in the same form.

Limits apply to the **account**, shared across all keys and browser sessions, and survive restarts. A batch is one request and costs one daily operation per puzzle. Verification also costs one operation. An admitted operation is charged even when a submitted answer is incorrect, generation fails, or the caller disconnects. Invalid request structure, rejected batches, and quota failures do not consume compute allocation. Unlimited accounts still record usage. The minute window follows database time; daily reset is midnight UTC.

PostgreSQL transactions lock the account row to enforce concurrent key creation and quota admission. Lowering the key cap revokes excess newer keys, preserving the oldest keys within the new cap. Lowering concurrency does not abort existing work; it prevents additional work until usage falls below the limit. Work leases remain held by timed-out CPU jobs; abandoned leases expire after five minutes for crash recovery. A full server-wide worker semaphore and the 16-item absolute batch bound still protect the host, even for an unlimited owner.

Disabling an account immediately revokes all of its keys and sessions. Re-enabling it does not restore revoked credentials. Admin actions, password changes, and key lifecycle events are audited without recording raw keys or passwords.

## Passwords, sessions, and keys

Passwords require at least 15 characters, at most 1024 UTF-8 bytes, and use salted Argon2id (19 MiB, two passes, one lane). Password hashing runs on bounded blocking workers. Login returns a generic credential error, including for disabled accounts; missing identities still perform password verification. Global, peer, and identifier-based throttles reduce authentication abuse. Forwarded client IP headers are deliberately not trusted, so a reverse proxy may share the peer budget; apply signup/login network limits at the proxy and tune deliberately as traffic grows.

Sessions are random opaque 256-bit values; only SHA-256 digests are stored. Sessions expire after seven days, with at most five retained live sessions per account. Password changes revoke prior sessions and issue a new one. Members can sign out every session. API keys are independent credentials and remain valid after a password change until explicitly revoked or the account is disabled.

Keys use a `pgk_` prefix and 256 random secret bits. The secret is returned once by creation, remains only in component memory in the browser, and cannot be recovered by listing keys. The database stores a digest, a short display prefix, and lifecycle metadata. A revoked key no longer authenticates; revoke and replace keys rather than editing their secrets.

Cookie-authenticated mutations require the exact configured Origin and the session's `X-CSRF-Token`. Login and registration also require the configured Origin and JSON input. Account endpoints have no cross-origin credential allowance. API-key requests to generation endpoints retain public CORS so learners can call them from their own applications. API keys never grant access to account management or administrative endpoints.

Email is currently a normalized sign-in identifier, not a verified mailbox. This phase has no email verification, automated email recovery, MFA, or OAuth. An operator with the database environment can run `puzzgen reset-password` to set a new passphrase through hidden prompts; recovery revokes all existing sessions and API keys and writes an audit event. Profile editing covers the display name; sign-in email changes are not exposed. Keep registration closed with `PUZZGEN_REGISTRATION_OPEN=false` until your owner setup and proxy policy are ready. Future email/recovery/OIDC work should preserve this account and quota model.

## Endpoint map

All paths below are under `/api/v1/account`. See `/api/v1/openapi.json` for the request contract.

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/config` | Public availability, registration status, default allocation |
| POST | `/register`, `/login` | Issue a cookie session |
| GET | `/session` | Current profile, allocation, usage, CSRF token |
| POST | `/logout`, `/sessions/revoke` | Sign out current/all sessions |
| POST | `/profile`, `/password` | Update display name or change password |
| POST | `/delete` | Delete own non-owner account with password and typed DELETE |
| GET/POST | `/keys` | List active key metadata / create a one-time secret |
| POST | `/keys/{id}/revoke` | Revoke an owned key |
| GET | `/admin/members` | Bounded member directory (up to 1000 entries) |
| POST | `/admin/members/{id}` | Set role, disabled state, and allocation |
| POST | `/admin/members/{id}/revoke` | Revoke all keys and sessions |
| GET | `/admin/audit` | Latest 100 audit events |

Use normal database backup practices for accounts and audit history. Run `puzzgen cleanup` periodically to remove expired sessions/leases, throttle buckets older than two days, revoked keys older than 30 days, and audit events older than 90 days. No recurring job was installed automatically. Keep the database connection pool small (five connections per API process) when sharing the VPS with other sites.

## Local verification

Unit and HTTP tests run with `cargo test --locked -p puzzgen`. PostgreSQL integration tests are explicitly opt-in:

```powershell
$env:TEST_DATABASE_URL = 'postgres://...@127.0.0.1:5432/puzzgen_auth_test'
cargo test --locked -p puzzgen --test accounts
```

The database name must end in `_test`. Each run creates a unique schema, applies migrations there, exercises real transactions/quotas, and drops its own schema after success. No live VPS database is required. A failed test may leave its uniquely named schema for diagnosis.

The concurrent quota test starts two independent HTTP server states on ephemeral loopback ports, sharing the isolated PostgreSQL schema. It alternates two member keys across both servers and checks atomic minute/day accounting, full default daily exhaustion, batch costs, concurrency leases, and immediate administrator overrides (including zero and unlimited). Worker capacity is raised in these test servers to isolate account quotas from global CPU saturation. Held work leases make the concurrency boundary deterministic; competing lease reservations are also tested directly. Expired minute/day buckets are simulated in the isolated schema rather than waiting for midnight. The existing demo server and its account limits are untouched.

To retain aggregate results without credentials, set `$env:QUOTA_REPORT_PATH` to a writable JSON file before running the integration tests. To run just the concurrent test:

```powershell
cargo test --locked -p puzzgen --test accounts concurrent_http_quotas_and_admin_overrides -- --ignored --nocapture
```

For browser QA, `cargo run --example seed_accounts_demo` seeds a disposable local `_test` database with public test credentials documented in that example. Never use those credentials for your owner account. The demo helper refuses a database name that lacks the `_test` suffix; normal production bootstrap uses the hidden-prompt `create-admin` command.

Reference guidance: [OWASP password storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html), [OWASP session management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html), [RustCrypto Argon2](https://docs.rs/argon2/0.5.3/argon2/), [SQLx](https://docs.rs/sqlx/0.8.6/sqlx/).

