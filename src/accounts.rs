//! PostgreSQL-backed identities, opaque sessions, API keys, and shared quotas.
use crate::api::{ApiError, AppState};
use argon2::{
    Algorithm, Argon2, Params, PasswordHasher, PasswordVerifier, Version,
    password_hash::{PasswordHash, SaltString},
};
use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{
    FromRow, PgPool, Row,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use uuid::Uuid;

type Result<T> = std::result::Result<T, ApiError>;

#[derive(Clone)]
pub struct AccountStore {
    pub pool: PgPool,
    origin: String,
    secure_cookie: bool,
    registration_open: bool,
    passwords: Arc<Semaphore>,
    dummy_hash: String,
    firebase: Option<crate::firebase::FirebaseVerifier>,
}

#[derive(Clone, FromRow)]
struct Account {
    id: Uuid,
    email: String,
    display_name: String,
    password_hash: String,
    role: String,
    disabled: bool,
    requests_per_minute: Option<i64>,
    operations_per_day: Option<i64>,
    max_batch: Option<i64>,
    max_concurrent: Option<i64>,
    max_keys: Option<i64>,
    created_at: time::OffsetDateTime,
}

impl Account {
    fn view(&self) -> Value {
        json!({"id":self.id,"email":self.email,"display_name":self.display_name,"role":self.role,"disabled":self.disabled,"created_at":self.created_at.to_string(),"limits":{
            "requests_per_minute":self.requests_per_minute,"operations_per_day":self.operations_per_day,
            "max_batch":self.max_batch,"max_concurrent":self.max_concurrent,"max_keys":self.max_keys}})
    }
    fn is_admin(&self) -> bool {
        matches!(self.role.as_str(), "admin" | "owner")
    }
}

#[derive(Clone)]
pub struct Principal {
    pub account_id: Uuid,
    credential: Credential,
}
#[derive(Clone)]
enum Credential {
    Session(String),
    Key(String),
    Guest { project: String, subject: String },
}

#[cfg(test)]
mod guest_tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires disposable PostgreSQL TEST_DATABASE_URL ending in _test"]
    async fn guests_share_identity_and_enforce_individual_and_global_budgets() {
        let url = std::env::var("TEST_DATABASE_URL").unwrap();
        let mut parsed = url::Url::parse(&url).unwrap();
        assert!(parsed.path().ends_with("_test"));
        let control = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("guest_test_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("CREATE SCHEMA {schema}"))
            .execute(&control)
            .await
            .unwrap();
        parsed
            .query_pairs_mut()
            .append_pair("options", &format!("-c search_path={schema}"));
        let store = AccountStore::connect(parsed.as_str(), "http://127.0.0.1:8080", false)
            .await
            .unwrap();
        store.migrate().await.unwrap();
        let a = store
            .guest_principal("beacon-dev", "fixture-a".into())
            .await
            .unwrap();
        let again = store
            .guest_principal("beacon-dev", "fixture-a".into())
            .await
            .unwrap();
        assert_eq!(a.account_id, again.account_id);
        let b = store
            .guest_principal("beacon-dev", "fixture-b".into())
            .await
            .unwrap();
        assert_ne!(a.account_id, b.account_id);
        let account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE id=$1")
            .bind(a.account_id)
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert!(!account.is_admin());
        assert_eq!(account.max_keys, Some(0));
        assert_eq!(account.operations_per_day, Some(100));
        let lease = store.reserve(a.clone(), 1, false).await.unwrap();
        assert!(store.reserve(a.clone(), 1, false).await.is_err());
        lease.release().await.unwrap();
        sqlx::query("UPDATE accounts SET operations_per_day=1 WHERE id=$1")
            .bind(a.account_id)
            .execute(&store.pool)
            .await
            .unwrap();
        assert!(store.reserve(a.clone(), 1, false).await.is_err());
        store
            .reserve(b.clone(), 1, false)
            .await
            .unwrap()
            .release()
            .await
            .unwrap();
        sqlx::query(
            "UPDATE guest_budget SET minute_count=60,minute_start=date_trunc('minute',now())",
        )
        .execute(&store.pool)
        .await
        .unwrap();
        assert!(store.reserve(b.clone(), 1, false).await.is_err());
        sqlx::query("UPDATE guest_budget SET minute_count=0,day_count=10000,day_start=(now() AT TIME ZONE 'UTC')::date").execute(&store.pool).await.unwrap();
        assert!(store.reserve(b.clone(), 1, false).await.is_err());
        sqlx::query("UPDATE guest_budget SET day_count=0")
            .execute(&store.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE accounts SET disabled=true WHERE id=$1")
            .bind(b.account_id)
            .execute(&store.pool)
            .await
            .unwrap();
        assert!(store.reserve(b, 1, false).await.is_err());
        // An old API key does not enter Firebase verification or mutate guest state.
        let headers = HeaderMap::new();
        assert!(store.authenticate_compute(&headers, false).await.is_err());
        store.pool.close().await;
        sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
            .execute(&control)
            .await
            .unwrap();
        control.close().await;
    }
}

/// A lease is held inside the blocking worker, even after its HTTP request times out.
pub struct WorkLease {
    pool: PgPool,
    id: Option<Uuid>,
}
impl WorkLease {
    pub async fn release(mut self) -> Result<()> {
        if let Some(id) = self.id {
            sqlx::query("DELETE FROM account_work WHERE id=$1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(db)?;
            self.id = None;
        }
        Ok(())
    }
}
impl Drop for WorkLease {
    fn drop(&mut self) {
        let pool = self.pool.clone();
        let Some(id) = self.id.take() else {
            return;
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                if let Err(error) = sqlx::query("DELETE FROM account_work WHERE id=$1").bind(id).execute(&pool).await {
                    tracing::error!(%error, "account work lease cleanup failed; expiry will recover it");
                }
            });
        }
    }
}

fn db(error: sqlx::Error) -> ApiError {
    // Database messages may contain email values; never send them to the client.
    tracing::error!(code = ?error.as_database_error().and_then(|e| e.code()), "account database operation failed");
    ApiError(
        StatusCode::SERVICE_UNAVAILABLE,
        "account_store_unavailable",
        "Account storage is temporarily unavailable".into(),
    )
}
fn denied() -> ApiError {
    ApiError(
        StatusCode::FORBIDDEN,
        "forbidden",
        "You do not have permission for this action".into(),
    )
}
fn unauthorized() -> ApiError {
    ApiError(
        StatusCode::UNAUTHORIZED,
        "sign_in_required",
        "Sign in or use a valid member API key".into(),
    )
}
fn limited(message: &str) -> ApiError {
    ApiError(
        StatusCode::TOO_MANY_REQUESTS,
        "quota_exceeded",
        message.into(),
    )
}
fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
fn random_secret() -> String {
    hex::encode(rand::random::<[u8; 32]>())
}
fn csrf(raw_session: &str) -> String {
    digest(&format!("puzzgen-csrf-v1:{raw_session}"))
}
fn json_body<T>(body: std::result::Result<Json<T>, JsonRejection>) -> Result<T> {
    body.map(|Json(value)| value)
        .map_err(|_| ApiError::invalid("Invalid account request body"))
}
fn email(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    let Some((local, domain)) = value.split_once('@') else {
        return Err(ApiError::invalid("Enter a valid email address"));
    };
    if value.len() > 254
        || local.is_empty()
        || local.len() > 64
        || domain.is_empty()
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.contains('@')
        || !value.is_ascii()
        || value
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    {
        return Err(ApiError::invalid("Enter a valid email address"));
    }
    Ok(value)
}
fn display_name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
        return Err(ApiError::invalid("Display name must be 1–80 characters"));
    }
    Ok(value.into())
}
fn password_policy(value: &str) -> Result<()> {
    if value.chars().count() < 15 || value.len() > 1024 {
        return Err(ApiError::invalid(
            "Use a password or passphrase of at least 15 characters and at most 1024 bytes",
        ));
    }
    Ok(())
}
fn argon() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19_456, 2, 1, None).expect("fixed Argon2 parameters"),
    )
}
fn hash_password(password: &str) -> std::result::Result<String, String> {
    let salt = SaltString::encode_b64(&rand::random::<[u8; 16]>())
        .map_err(|_| "Salt generation failed")?;
    argon()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| "Password hashing failed".into())
}

impl AccountStore {
    pub async fn connect(
        database_url: &str,
        public_origin: &str,
        registration_open: bool,
    ) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let origin = url::Url::parse(public_origin)?;
        if origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || !origin.username().is_empty()
            || origin.password().is_some()
        {
            return Err(
                "PUZZGEN_PUBLIC_ORIGIN must contain only scheme, host and optional port".into(),
            );
        }
        let loopback = matches!(
            origin.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
        );
        if origin.scheme() != "https" && !(origin.scheme() == "http" && loopback) {
            return Err("Account origins require HTTPS except on localhost".into());
        }
        let options = PgConnectOptions::from_str(database_url)?;
        // Our VPS topology uses local PostgreSQL. Avoid silently sending credentials
        // over a remote plaintext connection; add a deliberate TLS deployment if needed.
        if !matches!(options.get_host(), "localhost" | "127.0.0.1" | "::1")
            && !options.get_host().starts_with('/')
        {
            return Err("This build requires PostgreSQL on loopback or a Unix socket; use a local tunnel for remote development".into());
        }
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options.ssl_mode(PgSslMode::Disable))
            .await?;
        let dummy_hash = tokio::task::spawn_blocking(|| hash_password(&random_secret()))
            .await?
            .map_err(std::io::Error::other)?;
        Ok(Self {
            pool,
            origin: origin.origin().ascii_serialization(),
            secure_cookie: origin.scheme() == "https",
            registration_open,
            passwords: Arc::new(Semaphore::new(2)),
            dummy_hash,
            firebase: None,
        })
    }
    pub fn with_firebase(mut self, verifier: crate::firebase::FirebaseVerifier) -> Self {
        self.firebase = Some(verifier);
        self
    }
    pub async fn migrate(&self) -> std::result::Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./migrations").run(&self.pool).await
    }
    pub async fn check_schema(&self) -> std::result::Result<(), sqlx::Error> {
        if self.firebase.is_some() {
            sqlx::query("SELECT project,subject FROM firebase_guests LIMIT 0")
                .execute(&self.pool)
                .await?;
            sqlx::query("SELECT id FROM guest_budget LIMIT 0")
                .execute(&self.pool)
                .await?;
        }
        sqlx::query("SELECT id FROM accounts LIMIT 0")
            .execute(&self.pool)
            .await
            .map(|_| ())
    }
    pub async fn owner_exists(&self) -> std::result::Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE role='owner')")
            .fetch_one(&self.pool)
            .await
    }
    pub async fn reset_password(&self, address: &str, password: String) -> Result<()> {
        let address = email(address)?;
        password_policy(&password)?;
        let hash = self.hash(password).await?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        let id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM accounts WHERE email=$1 FOR UPDATE")
            .bind(address)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(|| ApiError::invalid("Account not found"))?;
        sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1")
            .bind(id)
            .bind(hash)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("DELETE FROM account_sessions WHERE account_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE member_api_keys SET revoked_at=now() WHERE account_id=$1 AND revoked_at IS NULL").bind(id).execute(&mut *tx).await.map_err(db)?;
        audit(&mut tx, None, id, "operator_password_reset", json!({})).await?;
        tx.commit().await.map_err(db)?;
        Ok(())
    }
    pub async fn cleanup(&self) -> std::result::Result<(), sqlx::Error> {
        for statement in [
            "DELETE FROM account_sessions WHERE expires_at<=now()",
            "DELETE FROM account_work WHERE expires_at<=now()",
            "DELETE FROM auth_throttle WHERE window_start<now()-interval '2 days'",
            "DELETE FROM member_api_keys WHERE revoked_at<now()-interval '30 days'",
            "DELETE FROM account_audit WHERE created_at<now()-interval '90 days'",
        ] {
            sqlx::query(statement).execute(&self.pool).await?;
        }
        Ok(())
    }
    async fn password_job<T: Send + 'static>(
        &self,
        job: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T> {
        let permit = self
            .passwords
            .clone()
            .try_acquire_owned()
            .map_err(|_| limited("Password workers are busy; retry later"))?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            job()
        })
        .await
        .map_err(|_| ApiError::internal())
    }
    async fn hash(&self, password: String) -> Result<String> {
        self.password_job(move || hash_password(&password))
            .await?
            .map_err(|_| ApiError::internal())
    }
    async fn verify_password(&self, password: String, hash: String) -> Result<bool> {
        if password.len() > 1024 {
            return Ok(false);
        }
        self.password_job(move || {
            PasswordHash::new(&hash)
                .is_ok_and(|hash| argon().verify_password(password.as_bytes(), &hash).is_ok())
        })
        .await
    }
    pub async fn bootstrap_owner(
        &self,
        address: &str,
        name: &str,
        password: String,
    ) -> Result<Uuid> {
        let address = email(address)?;
        let name = display_name(name)?;
        password_policy(&password)?;
        let hash = self.hash(password).await?;
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("SELECT pg_advisory_xact_lock(63772970)")
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE role='owner')")
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if exists {
            return Err(ApiError::invalid(
                "The protected owner account already exists",
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO accounts(id,email,display_name,password_hash,role,requests_per_minute,operations_per_day,max_batch,max_concurrent,max_keys) VALUES($1,$2,$3,$4,'owner',NULL,NULL,NULL,NULL,NULL)")
            .bind(id).bind(address).bind(name).bind(hash).execute(&mut *tx).await.map_err(db)?;
        audit(&mut tx, Some(id), id, "owner_bootstrapped", json!({})).await?;
        tx.commit().await.map_err(db)?;
        Ok(id)
    }
    fn cookie_name(&self) -> &'static str {
        if self.secure_cookie {
            "__Host-puzzgen_session"
        } else {
            "puzzgen_session"
        }
    }
    fn cookie(&self, token: &str, clear: bool) -> String {
        format!(
            "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
            self.cookie_name(),
            token,
            if clear { 0 } else { 604800 },
            if self.secure_cookie { "; Secure" } else { "" }
        )
    }
    fn browser_origin(&self, headers: &HeaderMap, require: bool) -> Result<()> {
        let supplied = headers.get(header::ORIGIN).and_then(|h| h.to_str().ok());
        if (require && supplied.is_none())
            || supplied.is_some_and(|origin| origin != self.origin)
            || headers
                .get("sec-fetch-site")
                .is_some_and(|h| h == "cross-site")
        {
            return Err(denied());
        }
        Ok(())
    }
    async fn session(&self, headers: &HeaderMap, mutation: bool) -> Result<(Account, String)> {
        self.browser_origin(headers, mutation)?;
        let token = headers
            .get(header::COOKIE)
            .and_then(|h| h.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';').find_map(|c| {
                    c.trim()
                        .split_once('=')
                        .filter(|(key, _)| *key == self.cookie_name())
                        .map(|(_, token)| token.to_owned())
                })
            })
            .ok_or_else(unauthorized)?;
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(unauthorized());
        }
        if mutation {
            let provided = headers
                .get("x-csrf-token")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");
            if !bool::from(csrf(&token).as_bytes().ct_eq(provided.as_bytes())) {
                return Err(denied());
            }
        }
        let account=sqlx::query_as::<_,Account>("SELECT a.* FROM accounts a JOIN account_sessions s ON s.account_id=a.id WHERE s.token_hash=$1 AND s.expires_at>now() AND NOT a.disabled")
            .bind(digest(&token)).fetch_optional(&self.pool).await.map_err(db)?.ok_or_else(unauthorized)?;
        Ok((account, token))
    }
    async fn new_session(&self, id: Uuid) -> Result<String> {
        let raw = random_secret();
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx::query("SELECT id FROM accounts WHERE id=$1 AND NOT disabled FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(unauthorized)?;
        sqlx::query("DELETE FROM account_sessions WHERE account_id=$1 AND expires_at<=now()")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        // Keep no more than five live sessions per account.
        sqlx::query("DELETE FROM account_sessions WHERE token_hash IN (SELECT token_hash FROM account_sessions WHERE account_id=$1 ORDER BY created_at DESC OFFSET 4)").bind(id).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO account_sessions(token_hash,account_id,expires_at) VALUES($1,$2,now()+interval '7 days')").bind(digest(&raw)).bind(id).execute(&mut *tx).await.map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(raw)
    }
    pub async fn authenticate_compute(
        &self,
        headers: &HeaderMap,
        mutation: bool,
    ) -> Result<Principal> {
        if let Some(value) = headers.get(header::AUTHORIZATION) {
            let token = value
                .to_str()
                .ok()
                .and_then(|h| h.split_once(' '))
                .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("Bearer"))
                .map(|(_, token)| token)
                .ok_or_else(unauthorized)?;
            if !token.starts_with("pgk_") || token.len() != 68 {
                let verifier = self.firebase.as_ref().ok_or_else(unauthorized)?;
                let subject = verifier.verify(token).await?;
                return self.guest_principal(verifier.project(), subject).await;
            }
            let hash = digest(token);
            let id=sqlx::query_scalar::<_,Uuid>("SELECT a.id FROM accounts a JOIN member_api_keys k ON k.account_id=a.id WHERE k.token_hash=$1 AND k.revoked_at IS NULL AND NOT a.disabled").bind(&hash).fetch_optional(&self.pool).await.map_err(db)?.ok_or_else(unauthorized)?;
            Ok(Principal {
                account_id: id,
                credential: Credential::Key(hash),
            })
        } else {
            let (account, raw) = self.session(headers, mutation).await?;
            Ok(Principal {
                account_id: account.id,
                credential: Credential::Session(digest(&raw)),
            })
        }
    }
    pub async fn authorize_admin(&self, headers: &HeaderMap) -> Result<()> {
        if !self.session(headers, false).await?.0.is_admin() {
            return Err(denied());
        }
        Ok(())
    }
    async fn guest_principal(&self, project: &str, subject: String) -> Result<Principal> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        // Serialize registration; one identity cannot create several accounts in a race.
        let count: i64 =
            sqlx::query_scalar("SELECT identities FROM guest_budget WHERE id=true FOR UPDATE")
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT account_id FROM firebase_guests WHERE project=$1 AND subject=$2",
        )
        .bind(project)
        .bind(&subject)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        let id = if let Some(id) = existing {
            id
        } else {
            if count >= 10000 {
                return Err(limited(
                    "Guest capacity is full. Your saved puzzle still works.",
                ));
            }
            let id = Uuid::new_v4();
            // No user email, password, API key or client-provided role is trusted.
            sqlx::query("INSERT INTO accounts(id,email,display_name,password_hash,role,requests_per_minute,operations_per_day,max_batch,max_concurrent,max_keys) VALUES($1,$2,'Garden guest',$3,'member',10,100,1,1,0)")
                .bind(id).bind(format!("guest-{}@firebase.invalid",id.simple())).bind(&self.dummy_hash).execute(&mut *tx).await.map_err(db)?;
            sqlx::query("INSERT INTO firebase_guests(project,subject,account_id) VALUES($1,$2,$3)")
                .bind(project)
                .bind(&subject)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query("UPDATE guest_budget SET identities=identities+1 WHERE id=true")
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            id
        };
        tx.commit().await.map_err(db)?;
        Ok(Principal {
            account_id: id,
            credential: Credential::Guest {
                project: project.into(),
                subject,
            },
        })
    }
    pub async fn reserve(
        &self,
        principal: Principal,
        cost: i64,
        is_batch: bool,
    ) -> Result<WorkLease> {
        let mut tx = self.pool.begin().await.map_err(db)?;
        if matches!(principal.credential, Credential::Guest { .. }) {
            // Shared admission lock always precedes the per-account lock.
            let row=sqlx::query("SELECT CASE WHEN minute_start=date_trunc('minute',now()) THEN minute_count ELSE 0 END AS minute_count, CASE WHEN day_start=(now() AT TIME ZONE 'UTC')::date THEN day_count ELSE 0 END AS day_count FROM guest_budget WHERE id=true FOR UPDATE").fetch_one(&mut *tx).await.map_err(db)?;
            let minute: i64 = row.get("minute_count");
            let day: i64 = row.get("day_count");
            if minute >= 60 || cost > 10000_i64.saturating_sub(day) {
                return Err(limited(
                    "The shared guest service needs a little rest. Your saved puzzle is still here.",
                ));
            }
            sqlx::query("UPDATE guest_budget SET minute_start=date_trunc('minute',now()),minute_count=$1,day_start=(now() AT TIME ZONE 'UTC')::date,day_count=$2 WHERE id=true").bind(minute+1).bind(day+cost).execute(&mut *tx).await.map_err(db)?;
        }
        // Serialize all admissions and key creations for the same account. Quotas
        // are shared by every key and browser session across server processes.
        let account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE id=$1 FOR UPDATE")
            .bind(principal.account_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?
            .ok_or_else(unauthorized)?;
        if account.disabled {
            return Err(unauthorized());
        }
        let valid=match &principal.credential {
            Credential::Key(hash)=>sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM member_api_keys WHERE account_id=$1 AND token_hash=$2 AND revoked_at IS NULL)").bind(account.id).bind(hash).fetch_one(&mut *tx).await.map_err(db)?,
            Credential::Session(hash)=>sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM account_sessions WHERE account_id=$1 AND token_hash=$2 AND expires_at>now())").bind(account.id).bind(hash).fetch_one(&mut *tx).await.map_err(db)?,
            Credential::Guest{project,subject}=>sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM firebase_guests WHERE account_id=$1 AND project=$2 AND subject=$3)").bind(account.id).bind(project).bind(subject).fetch_one(&mut *tx).await.map_err(db)?,
        };
        if !valid {
            return Err(unauthorized());
        }
        if let Credential::Key(hash) = &principal.credential {
            sqlx::query("UPDATE member_api_keys SET last_used_at=now() WHERE token_hash=$1")
                .bind(hash)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
        }
        if is_batch && account.max_batch.is_some_and(|limit| cost > limit) {
            return Err(limited("This batch exceeds your account's batch limit"));
        }
        sqlx::query("DELETE FROM account_work WHERE account_id=$1 AND expires_at<=now()")
            .bind(account.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let working: i64 =
            sqlx::query_scalar("SELECT count(*) FROM account_work WHERE account_id=$1")
                .bind(account.id)
                .fetch_one(&mut *tx)
                .await
                .map_err(db)?;
        if account.max_concurrent.is_some_and(|limit| working >= limit) {
            return Err(limited(
                "Your account's concurrent operation limit has been reached",
            ));
        }
        sqlx::query("INSERT INTO account_usage(account_id) VALUES($1) ON CONFLICT DO NOTHING")
            .bind(account.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let usage=sqlx::query("SELECT CASE WHEN minute_start=date_trunc('minute',now()) THEN minute_count ELSE 0 END AS minute_count, CASE WHEN day_start=(now() AT TIME ZONE 'UTC')::date THEN day_count ELSE 0 END AS day_count FROM account_usage WHERE account_id=$1").bind(account.id).fetch_one(&mut *tx).await.map_err(db)?;
        let minute: i64 = usage.get("minute_count");
        let day: i64 = usage.get("day_count");
        if account
            .requests_per_minute
            .is_some_and(|limit| minute >= limit)
        {
            return Err(limited("Your requests-per-minute limit has been reached"));
        }
        if account
            .operations_per_day
            .is_some_and(|limit| cost > limit.saturating_sub(day))
        {
            return Err(limited(
                "Your daily puzzle-operation limit has been reached (resets at midnight UTC)",
            ));
        }
        sqlx::query("UPDATE account_usage SET minute_start=date_trunc('minute',now()),minute_count=$2,day_start=(now() AT TIME ZONE 'UTC')::date,day_count=$3,total_operations=total_operations+$4 WHERE account_id=$1").bind(account.id).bind(minute+1).bind(day+cost).bind(cost).execute(&mut *tx).await.map_err(db)?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO account_work(id,account_id) VALUES($1,$2)")
            .bind(id)
            .bind(account.id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        tx.commit().await.map_err(db)?;
        Ok(WorkLease {
            pool: self.pool.clone(),
            id: Some(id),
        })
    }
    async fn throttle(&self, bucket: &str, limit: i64) -> Result<()> {
        let admitted=sqlx::query_scalar::<_,i64>("INSERT INTO auth_throttle(bucket) VALUES($1) ON CONFLICT(bucket) DO UPDATE SET window_start=date_trunc('minute',now()), attempts=CASE WHEN auth_throttle.window_start=date_trunc('minute',now()) THEN auth_throttle.attempts+1 ELSE 1 END WHERE auth_throttle.window_start<>date_trunc('minute',now()) OR auth_throttle.attempts<$2 RETURNING attempts")
            .bind(digest(bucket)).bind(limit).fetch_optional(&self.pool).await.map_err(db)?;
        if admitted.is_none() {
            return Err(limited(
                "Too many account requests; wait a minute before trying again",
            ));
        }
        Ok(())
    }
}

async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: Option<Uuid>,
    subject: Uuid,
    action: &str,
    details: Value,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO account_audit(actor_id,subject_id,action,details) VALUES($1,$2,$3,$4)",
    )
    .bind(actor)
    .bind(subject)
    .bind(action)
    .bind(details)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

fn store(state: &AppState) -> Result<AccountStore> {
    state.0.accounts.clone().ok_or_else(|| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "accounts_not_configured",
            "Accounts are not configured on this instance".into(),
        )
    })
}
async fn auth_budget(
    store: &AccountStore,
    headers: &HeaderMap,
    peer: Option<ConnectInfo<SocketAddr>>,
) -> Result<()> {
    store.browser_origin(headers, true)?;
    store.throttle("auth:global", 30).await?;
    let peer = peer.map_or_else(|| "unknown".into(), |peer| peer.0.ip().to_string());
    // Forwarded headers are deliberately not trusted. Configure proxy limits too.
    store.throttle(&format!("auth:peer:{peer}"), 10).await
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/config", get(config))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/session", get(session))
        .route("/logout", post(logout))
        .route("/profile", post(profile))
        .route("/password", post(change_password))
        .route("/delete", post(delete_account))
        .route("/sessions/revoke", post(revoke_sessions))
        .route("/keys", get(keys).post(create_key))
        .route("/keys/{id}/revoke", post(revoke_key))
        .route("/admin/members", get(members))
        .route("/admin/members/{id}", post(update_member))
        .route("/admin/members/{id}/revoke", post(admin_revoke))
        .route("/admin/audit", get(audit_log))
        .layer(axum::middleware::from_fn(account_deadline))
}
async fn account_deadline(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    tokio::time::timeout(Duration::from_secs(15), next.run(request))
        .await
        .unwrap_or_else(|_| {
            ApiError(
                StatusCode::GATEWAY_TIMEOUT,
                "request_timeout",
                "Account request exceeded its time budget".into(),
            )
            .into_response()
        })
}
async fn config(State(state): State<AppState>) -> Json<Value> {
    Json(
        json!({"enabled":state.0.accounts.is_some(),"registration_open":state.0.accounts.as_ref().is_some_and(|s|s.registration_open),"defaults":{"requests_per_minute":10,"operations_per_day":100,"max_batch":1,"max_concurrent":1,"max_keys":3},"capacity":{"max_batch":16}}),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Register {
    email: String,
    display_name: String,
    password: String,
}
async fn register(
    State(state): State<AppState>,
    peer: std::result::Result<
        ConnectInfo<SocketAddr>,
        axum::extract::rejection::ExtensionRejection,
    >,
    headers: HeaderMap,
    body: std::result::Result<Json<Register>, JsonRejection>,
) -> Result<Response> {
    let store = store(&state)?;
    auth_budget(&store, &headers, peer.ok()).await?;
    if !store.registration_open {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "registration_closed",
            "Registration is currently closed".into(),
        ));
    }
    let input = json_body(body)?;
    let email = email(&input.email)?;
    let name = display_name(&input.display_name)?;
    password_policy(&input.password)?;
    store.throttle(&format!("register:{email}"), 3).await?;
    let hash = store.hash(input.password).await?;
    let id = Uuid::new_v4();
    let result=sqlx::query("INSERT INTO accounts(id,email,display_name,password_hash) VALUES($1,$2,$3,$4) ON CONFLICT(email) DO NOTHING RETURNING id").bind(id).bind(email).bind(name).bind(hash).fetch_optional(&store.pool).await.map_err(db)?;
    if result.is_none() {
        return Err(ApiError::invalid(
            "The account could not be created; try signing in or choose another address",
        ));
    }
    signed_in(&store, id).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Login {
    email: String,
    password: String,
}
async fn login(
    State(state): State<AppState>,
    peer: std::result::Result<
        ConnectInfo<SocketAddr>,
        axum::extract::rejection::ExtensionRejection,
    >,
    headers: HeaderMap,
    body: std::result::Result<Json<Login>, JsonRejection>,
) -> Result<Response> {
    let store = store(&state)?;
    auth_budget(&store, &headers, peer.ok()).await?;
    let input = json_body(body)?;
    let email = email(&input.email)?;
    store.throttle(&format!("login:{email}"), 5).await?;
    let account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE email=$1")
        .bind(email)
        .fetch_optional(&store.pool)
        .await
        .map_err(db)?;
    let hash = account
        .as_ref()
        .map_or(&store.dummy_hash, |a| &a.password_hash)
        .clone();
    let valid = store.verify_password(input.password, hash).await?;
    let account = account.filter(|a| valid && !a.disabled).ok_or_else(|| {
        ApiError(
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
            "Email or password is incorrect".into(),
        )
    })?;
    signed_in(&store, account.id).await
}
async fn signed_in(store: &AccountStore, id: Uuid) -> Result<Response> {
    let token = store.new_session(id).await?;
    let account = sqlx::query_as::<_, Account>("SELECT * FROM accounts WHERE id=$1")
        .bind(id)
        .fetch_one(&store.pool)
        .await
        .map_err(db)?;
    let mut response =
        Json(json!({"user":account.view(),"csrf_token":csrf(&token)})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&store.cookie(&token, false)).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}
async fn session(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (account, token) = store.session(&headers, false).await?;
    let row=sqlx::query("SELECT CASE WHEN minute_start=date_trunc('minute',now()) THEN minute_count ELSE 0 END AS minute_count,CASE WHEN day_start=(now() AT TIME ZONE 'UTC')::date THEN day_count ELSE 0 END AS day_count,total_operations FROM account_usage WHERE account_id=$1").bind(account.id).fetch_optional(&store.pool).await.map_err(db)?;
    let usage=row.map(|r|json!({"requests_this_minute":r.get::<i64,_>("minute_count"),"operations_today":r.get::<i64,_>("day_count"),"total_operations":r.get::<i64,_>("total_operations")})).unwrap_or_else(||json!({"requests_this_minute":0,"operations_today":0,"total_operations":0}));
    Ok(Json(
        json!({"user":account.view(),"csrf_token":csrf(&token),"usage":usage}),
    ))
}
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response> {
    let store = store(&state)?;
    let (_, token) = store.session(&headers, true).await?;
    sqlx::query("DELETE FROM account_sessions WHERE token_hash=$1")
        .bind(digest(&token))
        .execute(&store.pool)
        .await
        .map_err(db)?;
    let mut response = Json(json!({"ok":true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&store.cookie("", true)).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    display_name: String,
}
async fn profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: std::result::Result<Json<Profile>, JsonRejection>,
) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, true).await?;
    store.throttle(&format!("manage:{}", user.id), 20).await?;
    let name = display_name(&json_body(body)?.display_name)?;
    sqlx::query("UPDATE accounts SET display_name=$2 WHERE id=$1")
        .bind(user.id)
        .bind(name)
        .execute(&store.pool)
        .await
        .map_err(db)?;
    Ok(Json(json!({"ok":true})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordChange {
    current_password: String,
    new_password: String,
}
async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: std::result::Result<Json<PasswordChange>, JsonRejection>,
) -> Result<Response> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, true).await?;
    store.throttle(&format!("password:{}", user.id), 5).await?;
    let input = json_body(body)?;
    password_policy(&input.new_password)?;
    if !store
        .verify_password(input.current_password, user.password_hash.clone())
        .await?
    {
        return Err(unauthorized());
    }
    let hash = store.hash(input.new_password).await?;
    let mut tx = store.pool.begin().await.map_err(db)?;
    // Compare the old hash to prevent concurrent password changes using stale state.
    let changed =
        sqlx::query("UPDATE accounts SET password_hash=$2 WHERE id=$1 AND password_hash=$3")
            .bind(user.id)
            .bind(hash)
            .bind(user.password_hash)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
    if changed.rows_affected() != 1 {
        return Err(unauthorized());
    }
    sqlx::query("DELETE FROM account_sessions WHERE account_id=$1")
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    audit(
        &mut tx,
        Some(user.id),
        user.id,
        "password_changed",
        json!({}),
    )
    .await?;
    tx.commit().await.map_err(db)?;
    signed_in(&store, user.id).await
}
async fn revoke_sessions(State(state): State<AppState>, headers: HeaderMap) -> Result<Response> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, true).await?;
    sqlx::query("DELETE FROM account_sessions WHERE account_id=$1")
        .bind(user.id)
        .execute(&store.pool)
        .await
        .map_err(db)?;
    let mut response = Json(json!({"ok":true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&store.cookie("", true)).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeleteAccount {
    password: String,
    confirmation: String,
}
async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: std::result::Result<Json<DeleteAccount>, JsonRejection>,
) -> Result<Response> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, true).await?;
    if user.role == "owner" {
        return Err(ApiError::invalid(
            "The protected owner account cannot be deleted",
        ));
    }
    store.throttle(&format!("password:{}", user.id), 5).await?;
    let input = json_body(body)?;
    if input.confirmation != "DELETE"
        || !store
            .verify_password(input.password, user.password_hash.clone())
            .await?
    {
        return Err(denied());
    }
    let mut tx = store.pool.begin().await.map_err(db)?;
    let result =
        sqlx::query("DELETE FROM accounts WHERE id=$1 AND role<>'owner' AND password_hash=$2")
            .bind(user.id)
            .bind(user.password_hash)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
    if result.rows_affected() != 1 {
        return Err(denied());
    }
    tx.commit().await.map_err(db)?;
    let mut response = Json(json!({"ok":true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&store.cookie("", true)).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}
async fn keys(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, false).await?;
    let rows=sqlx::query("SELECT id,name,prefix,created_at,last_used_at FROM member_api_keys WHERE account_id=$1 AND revoked_at IS NULL ORDER BY created_at DESC LIMIT 1000").bind(user.id).fetch_all(&store.pool).await.map_err(db)?;
    Ok(Json(
        json!({"keys":rows.iter().map(|r|json!({"id":r.get::<Uuid,_>("id"),"name":r.get::<String,_>("name"),"prefix":r.get::<String,_>("prefix"),"created_at":r.get::<time::OffsetDateTime,_>("created_at").to_string(),"last_used_at":r.get::<Option<time::OffsetDateTime>,_>("last_used_at").map(|t|t.to_string())})).collect::<Vec<_>>()}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyName {
    name: String,
}
async fn create_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: std::result::Result<Json<KeyName>, JsonRejection>,
) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, true).await?;
    store.throttle(&format!("keys:{}", user.id), 5).await?;
    let name = display_name(&json_body(body)?.name)?;
    let raw = format!("pgk_{}", random_secret());
    let prefix = raw[..12].to_owned();
    let id = Uuid::new_v4();
    let mut tx = store.pool.begin().await.map_err(db)?;
    let owner = sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE id=$1 AND NOT disabled FOR UPDATE",
    )
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?
    .ok_or_else(unauthorized)?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM member_api_keys WHERE account_id=$1 AND revoked_at IS NULL",
    )
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db)?;
    if owner.max_keys.is_some_and(|limit| count >= limit) {
        return Err(limited(
            "Your active API-key limit has been reached; revoke a key to free a slot",
        ));
    }
    sqlx::query(
        "INSERT INTO member_api_keys(id,account_id,name,prefix,token_hash) VALUES($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(user.id)
    .bind(&name)
    .bind(&prefix)
    .bind(digest(&raw))
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    audit(
        &mut tx,
        Some(user.id),
        user.id,
        "key_created",
        json!({"key_id":id,"prefix":prefix}),
    )
    .await?;
    tx.commit().await.map_err(db)?;
    Ok(Json(
        json!({"id":id,"name":name,"prefix":prefix,"secret":raw,"message":"Save this key now. Its secret will not be shown again."}),
    ))
}
async fn revoke_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, true).await?;
    let mut tx = store.pool.begin().await.map_err(db)?;
    // Share the account lock with admission, making revocation ordering explicit.
    sqlx::query("SELECT id FROM accounts WHERE id=$1 FOR UPDATE")
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    let changed=sqlx::query("UPDATE member_api_keys SET revoked_at=now() WHERE id=$1 AND account_id=$2 AND revoked_at IS NULL").bind(id).bind(user.id).execute(&mut *tx).await.map_err(db)?;
    if changed.rows_affected() != 1 {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "key_not_found",
            "Active key not found".into(),
        ));
    }
    audit(
        &mut tx,
        Some(user.id),
        user.id,
        "key_revoked",
        json!({"key_id":id}),
    )
    .await?;
    tx.commit().await.map_err(db)?;
    Ok(Json(json!({"ok":true})))
}
async fn members(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, false).await?;
    if !user.is_admin() {
        return Err(denied());
    }
    let rows =
        sqlx::query_as::<_, Account>("SELECT * FROM accounts ORDER BY created_at LIMIT 1000")
            .fetch_all(&store.pool)
            .await
            .map_err(db)?;
    Ok(Json(
        json!({"members":rows.iter().map(Account::view).collect::<Vec<_>>(),"limit":1000}),
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberUpdate {
    role: String,
    disabled: bool,
    requests_per_minute: Option<i64>,
    operations_per_day: Option<i64>,
    max_batch: Option<i64>,
    max_concurrent: Option<i64>,
    max_keys: Option<i64>,
}
fn validate_limits(input: &MemberUpdate) -> Result<()> {
    if [
        input.requests_per_minute,
        input.operations_per_day,
        input.max_batch,
        input.max_concurrent,
        input.max_keys,
    ]
    .into_iter()
    .flatten()
    .any(|limit| !(0..=i32::MAX as i64).contains(&limit))
    {
        return Err(ApiError::invalid(
            "Limits must be nonnegative integers up to 2147483647, or null for unlimited",
        ));
    }
    if !matches!(
        input.role.as_str(),
        "member" | "trusted" | "admin" | "owner"
    ) {
        return Err(ApiError::invalid("Unknown role"));
    }
    Ok(())
}
async fn update_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    body: std::result::Result<Json<MemberUpdate>, JsonRejection>,
) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (actor, _) = store.session(&headers, true).await?;
    if !actor.is_admin() {
        return Err(denied());
    }
    let input = json_body(body)?;
    validate_limits(&input)?;
    let mut tx = store.pool.begin().await.map_err(db)?;
    // Fixed lock order avoids deadlocks when two administrators edit each other.
    let locked = sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE id=$1 OR id=$2 ORDER BY id FOR UPDATE",
    )
    .bind(actor.id)
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let actor = locked
        .iter()
        .find(|a| a.id == actor.id && !a.disabled)
        .ok_or_else(unauthorized)?;
    let target = locked.iter().find(|a| a.id == id).ok_or_else(|| {
        ApiError(
            StatusCode::NOT_FOUND,
            "member_not_found",
            "Member not found".into(),
        )
    })?;
    if !actor.is_admin()
        || (actor.role != "owner"
            && (target.role == "owner"
                || (target.role == "admin" && target.id != actor.id)
                || input.role != target.role))
        || (target.role == "owner" && (input.role != "owner" || input.disabled))
        || (input.role == "owner" && target.role != "owner")
    {
        return Err(denied());
    }
    sqlx::query("UPDATE accounts SET role=$2,disabled=$3,requests_per_minute=$4,operations_per_day=$5,max_batch=$6,max_concurrent=$7,max_keys=$8 WHERE id=$1")
        .bind(id).bind(&input.role).bind(input.disabled).bind(input.requests_per_minute).bind(input.operations_per_day).bind(input.max_batch).bind(input.max_concurrent).bind(input.max_keys).execute(&mut *tx).await.map_err(db)?;
    if input.disabled {
        sqlx::query("DELETE FROM account_sessions WHERE account_id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        sqlx::query("UPDATE member_api_keys SET revoked_at=now() WHERE account_id=$1 AND revoked_at IS NULL").bind(id).execute(&mut *tx).await.map_err(db)?;
    }
    let keys_revoked = if let Some(limit) = input.max_keys {
        sqlx::query("UPDATE member_api_keys SET revoked_at=now() WHERE id IN (SELECT id FROM member_api_keys WHERE account_id=$1 AND revoked_at IS NULL ORDER BY created_at,id OFFSET $2)").bind(id).bind(limit).execute(&mut *tx).await.map_err(db)?.rows_affected()
    } else {
        0
    };
    audit(&mut tx,Some(actor.id),id,"member_updated",json!({"role":input.role,"disabled":input.disabled,"keys_revoked":keys_revoked,"limits":{"requests_per_minute":input.requests_per_minute,"operations_per_day":input.operations_per_day,"max_batch":input.max_batch,"max_concurrent":input.max_concurrent,"max_keys":input.max_keys}})).await?;
    tx.commit().await.map_err(db)?;
    Ok(Json(json!({"ok":true,"keys_revoked":keys_revoked})))
}
async fn admin_revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (actor, _) = store.session(&headers, true).await?;
    if !actor.is_admin() {
        return Err(denied());
    }
    let mut tx = store.pool.begin().await.map_err(db)?;
    let locked = sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE id=$1 OR id=$2 ORDER BY id FOR UPDATE",
    )
    .bind(actor.id)
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(db)?;
    let actor = locked
        .iter()
        .find(|a| a.id == actor.id && !a.disabled)
        .ok_or_else(unauthorized)?;
    let target = locked.iter().find(|a| a.id == id).ok_or_else(denied)?;
    if !actor.is_admin() || (actor.role != "owner" && target.id != actor.id && target.is_admin()) {
        return Err(denied());
    }
    sqlx::query("DELETE FROM account_sessions WHERE account_id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    sqlx::query(
        "UPDATE member_api_keys SET revoked_at=now() WHERE account_id=$1 AND revoked_at IS NULL",
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(db)?;
    audit(
        &mut tx,
        Some(actor.id),
        id,
        "all_credentials_revoked",
        json!({}),
    )
    .await?;
    tx.commit().await.map_err(db)?;
    Ok(Json(json!({"ok":true})))
}
async fn audit_log(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>> {
    let store = store(&state)?;
    let (user, _) = store.session(&headers, false).await?;
    if !user.is_admin() {
        return Err(denied());
    }
    let rows=sqlx::query("SELECT actor_id,subject_id,action,details,created_at FROM account_audit ORDER BY id DESC LIMIT 100").fetch_all(&store.pool).await.map_err(db)?;
    Ok(Json(
        json!({"events":rows.iter().map(|r|json!({"actor_id":r.get::<Option<Uuid>,_>("actor_id"),"subject_id":r.get::<Option<Uuid>,_>("subject_id"),"action":r.get::<String,_>("action"),"details":r.get::<Value,_>("details"),"created_at":r.get::<time::OffsetDateTime,_>("created_at").to_string()})).collect::<Vec<_>>()}),
    ))
}

pub fn extend_openapi(document: &mut Value) {
    document["components"]["securitySchemes"]["memberSession"] = json!({"type":"apiKey","in":"cookie","name":"__Host-puzzgen_session","description":"Opaque HttpOnly cookie; localhost HTTP uses puzzgen_session. Account mutations also require the canonical Origin and X-CSRF-Token returned by /account/session."});
    document["components"]["securitySchemes"]["bearerAuth"]["description"] = json!(
        "Member API key in account mode; legacy environment key only when member accounts are unconfigured. Keys cannot access account/admin endpoints."
    );
    document["security"] = json!([{"bearerAuth":[]},{"memberSession":[]}]);
    document["paths"]["/metrics"]["get"]["security"] = json!([{"memberSession":[]}]);
    document["paths"]["/metrics"]["get"]["description"] = json!(
        "Member mode requires an admin/owner cookie session. Legacy mode accepts its configured static key."
    );
    let text = json!({"type":"string"});
    let password = json!({"type":"string","maxLength":1024,"writeOnly":true});
    let role = json!({"type":"string","enum":["member","trusted","admin","owner"]});
    let mut schemas = serde_json::Map::new();
    schemas.insert("RegisterAccount".into(),json!({"type":"object","additionalProperties":false,"required":["email","display_name","password"],"properties":{"email":{"type":"string","format":"email","maxLength":254},"display_name":{"type":"string","minLength":1,"maxLength":80},"password":{"type":"string","minLength":15,"maxLength":1024,"writeOnly":true}}}));
    schemas.insert("LoginAccount".into(),json!({"type":"object","additionalProperties":false,"required":["email","password"],"properties":{"email":{"type":"string","format":"email"},"password":password}}));
    schemas.insert("AccountProfile".into(),json!({"type":"object","additionalProperties":false,"required":["display_name"],"properties":{"display_name":{"type":"string","minLength":1,"maxLength":80}}}));
    schemas.insert("ChangePassword".into(),json!({"type":"object","additionalProperties":false,"required":["current_password","new_password"],"properties":{"current_password":password,"new_password":{"type":"string","minLength":15,"maxLength":1024,"writeOnly":true}}}));
    schemas.insert("DeleteAccount".into(),json!({"type":"object","additionalProperties":false,"required":["password","confirmation"],"properties":{"password":password,"confirmation":{"const":"DELETE"}}}));
    schemas.insert("CreateMemberKey".into(),json!({"type":"object","additionalProperties":false,"required":["name"],"properties":{"name":{"type":"string","minLength":1,"maxLength":80}}}));
    let nullable_limit = json!({"type":["integer","null"],"minimum":0,"maximum":2147483647,"description":"Null means unlimited account quota; zero disables the resource. Physical server limits still apply."});
    schemas.insert("MemberAllocation".into(),json!({"type":"object","additionalProperties":false,"required":["role","disabled","requests_per_minute","operations_per_day","max_keys","max_batch","max_concurrent"],"properties":{"role":role,"disabled":{"type":"boolean"},"requests_per_minute":nullable_limit,"operations_per_day":nullable_limit,"max_keys":nullable_limit,"max_batch":nullable_limit,"max_concurrent":nullable_limit}}));
    schemas.insert("MemberKeyCreated".into(),json!({"type":"object","properties":{"id":{"type":"string","format":"uuid"},"name":text,"prefix":text,"secret":{"type":"string","pattern":"^pgk_[0-9a-f]{64}$","description":"Shown exactly once; never returned in key listings."},"message":text}}));
    for (name, schema) in schemas {
        document["components"]["schemas"][name] = schema;
    }
    for (method, path, summary, schema, public) in [
        (
            "get",
            "config",
            "Account availability and default allocation",
            None,
            true,
        ),
        (
            "post",
            "register",
            "Create a regular account and cookie session",
            Some("RegisterAccount"),
            true,
        ),
        (
            "post",
            "login",
            "Sign in using email and password",
            Some("LoginAccount"),
            true,
        ),
        (
            "get",
            "session",
            "Profile, current limits, usage, and CSRF token",
            None,
            false,
        ),
        ("post", "logout", "Sign out this session", None, false),
        (
            "post",
            "sessions/revoke",
            "Sign out every session",
            None,
            false,
        ),
        (
            "post",
            "profile",
            "Update display name",
            Some("AccountProfile"),
            false,
        ),
        (
            "post",
            "password",
            "Change password and revoke prior sessions",
            Some("ChangePassword"),
            false,
        ),
        (
            "post",
            "delete",
            "Permanently delete own non-owner account",
            Some("DeleteAccount"),
            false,
        ),
        (
            "get",
            "keys",
            "List active key metadata, never secrets",
            None,
            false,
        ),
        (
            "post",
            "keys",
            "Create a key within the account's active-key cap",
            Some("CreateMemberKey"),
            false,
        ),
        (
            "post",
            "keys/{id}/revoke",
            "Revoke an owned key",
            None,
            false,
        ),
        (
            "get",
            "admin/members",
            "Admin: list up to 1000 members",
            None,
            false,
        ),
        (
            "post",
            "admin/members/{id}",
            "Admin: update allocation and disabled state; owner assigns roles",
            Some("MemberAllocation"),
            false,
        ),
        (
            "post",
            "admin/members/{id}/revoke",
            "Admin: revoke all keys and sessions",
            None,
            false,
        ),
        (
            "get",
            "admin/audit",
            "Admin: latest 100 audit events",
            None,
            false,
        ),
    ] {
        let mut operation = json!({"summary":summary,"tags":["Accounts"],"security":if public{json!([])}else{json!([{"memberSession":[]}])},"responses":{"200":{"description":"Success","content":{"application/json":{"schema":{"type":"object"}}}},"400":{"$ref":"#/components/responses/Error"},"401":{"$ref":"#/components/responses/Error"},"403":{"$ref":"#/components/responses/Error"},"404":{"$ref":"#/components/responses/Error"},"429":{"$ref":"#/components/responses/Error"},"503":{"$ref":"#/components/responses/Error"}}});
        let mut parameters = Vec::new();
        if method == "post" {
            parameters.push(json!({"name":"Origin","in":"header","required":true,"schema":{"type":"string"},"description":"Exact PUZZGEN_PUBLIC_ORIGIN; browsers send this automatically."}));
            if !public {
                parameters.push(json!({"name":"X-CSRF-Token","in":"header","required":true,"schema":{"type":"string"}}));
            }
        }
        if path.contains("{id}") {
            parameters.push(json!({"name":"id","in":"path","required":true,"schema":{"type":"string","format":"uuid"}}));
        }
        operation["parameters"] = json!(parameters);
        if let Some(schema) = schema {
            operation["requestBody"] = json!({"required":true,"content":{"application/json":{"schema":{"$ref":format!("#/components/schemas/{schema}")}}}});
        }
        if path == "keys" && method == "post" {
            operation["responses"]["200"]["content"]["application/json"]["schema"] =
                json!({"$ref":"#/components/schemas/MemberKeyCreated"});
        }
        document["paths"][format!("/api/v1/account/{path}")][method] = operation;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passwords_are_salted_and_verified() {
        let password = "a long unique test passphrase";
        let first = hash_password(password).unwrap();
        let second = hash_password(password).unwrap();
        assert_ne!(first, second);
        let hash = PasswordHash::new(&first).unwrap();
        assert!(argon().verify_password(password.as_bytes(), &hash).is_ok());
        assert!(argon().verify_password(b"wrong", &hash).is_err());
    }
    #[test]
    fn identities_and_limits_reject_bad_input() {
        assert_eq!(email("  Test@Example.com ").unwrap(), "test@example.com");
        assert!(email("a@b@c.test").is_err());
        assert!(password_policy("short").is_err());
        assert!(display_name("\n").is_err());
        let input = MemberUpdate {
            role: "member".into(),
            disabled: false,
            requests_per_minute: Some(-1),
            operations_per_day: None,
            max_batch: None,
            max_concurrent: None,
            max_keys: None,
        };
        assert!(validate_limits(&input).is_err());
    }
}
