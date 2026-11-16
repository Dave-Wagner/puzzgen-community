use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{
        DefaultBodyLimit, Path, Query, Request, State,
        rejection::{JsonRejection, PathRejection, QueryRejection},
    },
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::accounts::{AccountStore, Principal};
use crate::puzzles::{self, Difficulty, GenerateRequest, PuzzleKind};

pub struct Config {
    pub listen: SocketAddr,
    pub workers: usize,
    pub requests_per_minute: u64,
    pub api_keys: Vec<String>,
    pub static_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8080".parse().unwrap(),
            workers: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(2)
                .clamp(1, 32),
            requests_per_minute: 120,
            api_keys: Vec::new(),
            static_dir: "frontend/dist".into(),
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Self::default();
        if let Ok(value) = std::env::var("PUZZGEN_LISTEN") {
            config.listen = value.parse()?;
        }
        if let Ok(value) = std::env::var("PUZZGEN_WORKERS") {
            config.workers = value.parse()?;
        }
        if !(1..=256).contains(&config.workers) {
            return Err("PUZZGEN_WORKERS must be 1..=256".into());
        }
        if let Ok(value) = std::env::var("PUZZGEN_REQUESTS_PER_MINUTE") {
            config.requests_per_minute = value.parse()?;
        }
        if !(1..=1_000_000).contains(&config.requests_per_minute) {
            return Err("PUZZGEN_REQUESTS_PER_MINUTE must be 1..=1000000".into());
        }
        if let Ok(value) = std::env::var("PUZZGEN_API_KEYS") {
            config.api_keys = value
                .split(',')
                .map(str::trim)
                .filter(|key| !key.is_empty())
                .map(String::from)
                .collect();
            if config.api_keys.is_empty()
                || config.api_keys.iter().any(|key| {
                    key.len() < 32
                        || !key.is_ascii()
                        || key.bytes().any(|b| b.is_ascii_whitespace())
                })
            {
                return Err("PUZZGEN_API_KEYS requires comma-separated ASCII tokens of at least 32 characters without whitespace".into());
            }
        }
        if let Ok(value) = std::env::var("PUZZGEN_STATIC_DIR") {
            config.static_dir = value.into();
        }
        Ok(config)
    }
}

#[derive(Clone)]
pub struct AppState(pub(crate) Arc<Inner>);

pub(crate) struct Inner {
    pub(crate) accounts: Option<AccountStore>,
    config: Config,
    permits: Arc<Semaphore>,
    limiter: Mutex<(Instant, u64)>,
    generated: AtomicU64,
    rejected: AtomicU64,
    generation_micros: AtomicU64,
    started: Instant,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self::with_accounts(config, None)
    }
    pub fn with_accounts(config: Config, accounts: Option<AccountStore>) -> Self {
        let workers = config.workers;
        Self(Arc::new(Inner {
            accounts,
            config,
            permits: Arc::new(Semaphore::new(workers)),
            limiter: Mutex::new((Instant::now(), 0)),
            generated: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            generation_micros: AtomicU64::new(0),
            started: Instant::now(),
        }))
    }
}

#[derive(Debug)]
pub struct ApiError(
    pub(crate) StatusCode,
    pub(crate) &'static str,
    pub(crate) String,
);
impl ApiError {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, "invalid_request", message.into())
    }
    pub(crate) fn internal() -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "The operation could not be completed".into(),
        )
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (
            self.0,
            Json(json!({"error":{"code":self.1,"message":self.2}})),
        )
            .into_response();
        if self.0 == StatusCode::TOO_MANY_REQUESTS || self.0 == StatusCode::SERVICE_UNAVAILABLE {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        }
        if self.0 == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

pub fn router(state: AppState) -> Router {
    let compute = Router::new()
        .route("/generate", post(generate))
        .route("/batch", post(batch))
        .route("/verify", post(verify))
        .route("/puzzles/{kind}", get(generate_kind))
        .route("/daily/{kind}", get(daily))
        .route_layer(middleware::from_fn_with_state(state.clone(), admission));
    let api = Router::new()
        .route("/catalog", get(|| async { Json(puzzles::catalog()) }))
        .route(
            "/openapi.json",
            get(|| async { Json(crate::openapi::document()) }),
        )
        .merge(compute)
        .fallback(|| async {
            ApiError(
                StatusCode::NOT_FOUND,
                "not_found",
                "API endpoint not found".into(),
            )
        })
        .method_not_allowed_fallback(|| async {
            ApiError(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "HTTP method not supported".into(),
            )
        })
        // Public educational clients can run on a learner's own origin. No cookies.
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
                .expose_headers([header::RETRY_AFTER]),
        );
    Router::new()
        .route(
            "/health/live",
            get(|| async { Json(json!({"status":"ok"})) }),
        )
        .route(
            "/health/ready",
            get(|State(state): State<AppState>| async move {
                if let Some(accounts) = &state.0.accounts {
                    if sqlx::query("SELECT 1")
                        .execute(&accounts.pool)
                        .await
                        .is_err()
                    {
                        return ApiError(
                            StatusCode::SERVICE_UNAVAILABLE,
                            "database_unavailable",
                            "Account database is unavailable".into(),
                        )
                        .into_response();
                    }
                    Json(json!({"status":"ready","storage":"postgresql"})).into_response()
                } else {
                    Json(json!({"status":"ready","storage":"stateless"})).into_response()
                }
            }),
        )
        .route(
            "/metrics",
            get(metrics).route_layer(middleware::from_fn_with_state(state.clone(), authenticate)),
        )
        .route(
            "/docs",
            get(|| async { Html(include_str!("../docs/api.html")) }),
        )
        .route_service(
            "/account",
            ServeFile::new(state.0.config.static_dir.join("index.html")),
        )
        .nest("/api/v1", api)
        .nest("/api/v1/account", crate::accounts::routes())
        .fallback_service(ServeDir::new(&state.0.config.static_dir))
        .layer(DefaultBodyLimit::max(65_536))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(response_headers))
        .with_state(state)
}

async fn response_headers(request: Request, next: Next) -> Response {
    let is_api = request.uri().path().starts_with("/api/");
    let is_account_page = request.uri().path() == "/account";
    let is_index = request.uri().path() == "/";
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    if is_api || is_account_page {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    } else if is_index {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }
    response
}

fn authorized(state: &AppState, request: &Request) -> bool {
    if state.0.config.api_keys.is_empty() {
        return true;
    }
    let Some(token) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("Bearer"))
        .map(|(_, token)| token)
    else {
        return false;
    };
    let candidate = Sha256::digest(token.as_bytes());
    state.0.config.api_keys.iter().fold(false, |found, key| {
        let expected = Sha256::digest(key.as_bytes());
        found | bool::from(candidate.as_slice().ct_eq(expected.as_slice()))
    })
}

async fn authenticate(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if let Some(accounts) = &state.0.accounts {
        return match accounts.authorize_admin(request.headers()).await {
            Ok(()) => next.run(request).await,
            Err(error) => error.into_response(),
        };
    }
    if !authorized(&state, &request) {
        return ApiError(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "A valid Bearer API key is required".into(),
        )
        .into_response();
    }
    next.run(request).await
}

async fn admission(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    if let Some(accounts) = &state.0.accounts {
        match accounts
            .authenticate_compute(
                request.headers(),
                request.method() != axum::http::Method::GET,
            )
            .await
        {
            Ok(principal) => {
                request.extensions_mut().insert(principal);
            }
            Err(error) => return error.into_response(),
        }
        return tokio::time::timeout(Duration::from_secs(15), next.run(request))
            .await
            .unwrap_or_else(|_| {
                ApiError(
                    StatusCode::GATEWAY_TIMEOUT,
                    "request_timeout",
                    "Request exceeded its time budget".into(),
                )
                .into_response()
            });
    }
    if !authorized(&state, &request) {
        return ApiError(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "A valid Bearer API key is required".into(),
        )
        .into_response();
    }
    let admitted = {
        let mut window = state.0.limiter.lock().unwrap_or_else(|e| e.into_inner());
        if window.0.elapsed() >= Duration::from_secs(60) {
            *window = (Instant::now(), 0);
        }
        if window.1 >= state.0.config.requests_per_minute {
            false
        } else {
            window.1 += 1;
            true
        }
    };
    if !admitted {
        state.0.rejected.fetch_add(1, Ordering::Relaxed);
        return ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Instance request budget exhausted; retry in 60 seconds".into(),
        )
        .into_response();
    }
    // Bound the entire compute request, including receiving and parsing its body.
    tokio::time::timeout(Duration::from_secs(15), next.run(request))
        .await
        .unwrap_or_else(|_| {
            ApiError(
                StatusCode::GATEWAY_TIMEOUT,
                "request_timeout",
                "Request exceeded its time budget".into(),
            )
            .into_response()
        })
}

fn parse_json<T>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(value)| value).map_err(|error| {
        let status = if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            StatusCode::PAYLOAD_TOO_LARGE
        } else {
            StatusCode::BAD_REQUEST
        };
        ApiError(status, "invalid_json", error.body_text())
    })
}

async fn cpu<T: Send + 'static>(
    state: AppState,
    principal: Option<Extension<Principal>>,
    cost: i64,
    is_batch: bool,
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, ApiError> {
    let permit = state.0.permits.clone().try_acquire_owned().map_err(|_| {
        state.0.rejected.fetch_add(1, Ordering::Relaxed);
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "busy",
            "All puzzle workers are busy; retry later".into(),
        )
    })?;
    let lease = match (&state.0.accounts, principal) {
        (Some(accounts), Some(Extension(principal))) => {
            Some(accounts.reserve(principal, cost, is_batch).await?)
        }
        (Some(_), None) => {
            return Err(ApiError(
                StatusCode::UNAUTHORIZED,
                "sign_in_required",
                "Sign in or use a valid member API key".into(),
            ));
        }
        (None, _) => None,
    };
    let task = tokio::task::spawn_blocking(move || {
        // Keep the permit inside the task: an HTTP timeout must not free a running CPU slot.
        let _permit = permit;
        let started = Instant::now();
        let result = job();
        state.0.generation_micros.fetch_add(
            started.elapsed().as_micros().min(u64::MAX as u128) as u64,
            Ordering::Relaxed,
        );
        (result, lease)
    });
    let (result, lease) = tokio::time::timeout(Duration::from_secs(15), task)
        .await
        .map_err(|_| {
            ApiError(
                StatusCode::GATEWAY_TIMEOUT,
                "generation_timeout",
                "Puzzle operation exceeded its time budget".into(),
            )
        })?
        .map_err(|error| {
            tracing::error!(%error, "puzzle worker failed");
            ApiError::internal()
        })?;
    if let Some(lease) = lease {
        lease.release().await?;
    }
    result.map_err(ApiError::invalid)
}

async fn run_generate(
    state: AppState,
    request: GenerateRequest,
    principal: Option<Extension<Principal>>,
) -> Result<Json<crate::puzzles::Puzzle>, ApiError> {
    let puzzle = cpu(state.clone(), principal, 1, false, move || {
        puzzles::generate(request)
    })
    .await?;
    state.0.generated.fetch_add(1, Ordering::Relaxed);
    Ok(Json(puzzle))
}

async fn generate(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    body: Result<Json<GenerateRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    run_generate(state, parse_json(body)?, principal).await
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct GenerateQuery {
    seed: Option<u64>,
    #[serde(default)]
    difficulty: Difficulty,
    #[serde(default)]
    include_solution: bool,
    content_version: Option<String>,
}
async fn generate_kind(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<GenerateQuery>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Path(kind) = path.map_err(|e| ApiError::invalid(e.body_text()))?;
    let kind = kind
        .parse()
        .map_err(|_| ApiError::invalid("Unknown puzzle kind; see /api/v1/catalog"))?;
    let Query(query) = query.map_err(|e| ApiError::invalid(e.body_text()))?;
    run_generate(
        state,
        GenerateRequest {
            kind,
            seed: query.seed,
            difficulty: query.difficulty,
            include_solution: query.include_solution,
            content_version: query.content_version,
        },
        principal,
    )
    .await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchRequest {
    puzzles: Vec<GenerateRequest>,
}
async fn batch(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    body: Result<Json<BatchRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_json(body)?;
    if request.puzzles.is_empty() || request.puzzles.len() > 16 {
        return Err(ApiError::invalid("Batch must contain 1..=16 puzzles"));
    }
    let count = request.puzzles.len() as u64;
    let puzzles = cpu(state.clone(), principal, count as i64, true, move || {
        request
            .puzzles
            .into_iter()
            .map(puzzles::generate)
            .collect::<Result<Vec<_>, _>>()
    })
    .await?;
    state.0.generated.fetch_add(count, Ordering::Relaxed);
    Ok(Json(json!({"puzzles":puzzles})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyRequest {
    kind: PuzzleKind,
    seed: u64,
    #[serde(default)]
    difficulty: Difficulty,
    answer: Value,
    content_version: Option<String>,
}
async fn verify(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    body: Result<Json<VerifyRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let request = parse_json(body)?;
    let correct = cpu(state, principal, 1, false, move || {
        puzzles::verify(
            GenerateRequest {
                kind: request.kind,
                seed: Some(request.seed),
                difficulty: request.difficulty,
                include_solution: true,
                content_version: request.content_version,
            },
            request.answer,
        )
    })
    .await?;
    Ok(Json(json!({"correct":correct})))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DailyQuery {
    date: Option<String>,
    #[serde(default)]
    difficulty: Difficulty,
    #[serde(default)]
    include_solution: bool,
    content_version: Option<String>,
}
async fn daily(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<DailyQuery>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Path(kind) = path.map_err(|e| ApiError::invalid(e.body_text()))?;
    let puzzle_kind = kind
        .parse()
        .map_err(|_| ApiError::invalid("Unknown puzzle kind; see /api/v1/catalog"))?;
    let Query(query) = query.map_err(|e| ApiError::invalid(e.body_text()))?;
    let format = time::format_description::parse_borrowed::<2>("[year]-[month]-[day]")
        .map_err(|_| ApiError::internal())?;
    let date = match query.date {
        Some(text) => time::Date::parse(&text, &format).map_err(|_| {
            ApiError::invalid("date must be a valid YYYY-MM-DD calendar date (UTC)")
        })?,
        None => time::OffsetDateTime::now_utc().date(),
    }
    .format(&format)
    .map_err(|_| ApiError::internal())?;
    let difficulty = serde_json::to_string(&query.difficulty).map_err(|_| ApiError::internal())?;
    let hash = Sha256::digest(
        format!(
            "puzzgen-v{}:{kind}:{date}:{difficulty}",
            puzzles::GENERATOR_VERSION
        )
        .as_bytes(),
    );
    let seed = u64::from_le_bytes(hash[..8].try_into().unwrap());
    let Json(puzzle) = run_generate(
        state,
        GenerateRequest {
            kind: puzzle_kind,
            seed: Some(seed),
            difficulty: query.difficulty,
            include_solution: query.include_solution,
            content_version: query.content_version,
        },
        principal,
    )
    .await?;
    Ok(Json(json!({"date":date,"timezone":"UTC","puzzle":puzzle})))
}

async fn metrics(State(state): State<AppState>) -> Response {
    let inner = &state.0;
    let text = format!(
        "# TYPE puzzgen_generated_total counter\npuzzgen_generated_total {}\n# TYPE puzzgen_rejected_total counter\npuzzgen_rejected_total {}\n# TYPE puzzgen_cpu_seconds_total counter\npuzzgen_cpu_seconds_total {:.6}\n# TYPE puzzgen_uptime_seconds gauge\npuzzgen_uptime_seconds {}\n# TYPE puzzgen_workers_available gauge\npuzzgen_workers_available {}\n",
        inner.generated.load(Ordering::Relaxed),
        inner.rejected.load(Ordering::Relaxed),
        inner.generation_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0,
        inner.started.elapsed().as_secs(),
        inner.permits.available_permits()
    );
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        Body::from(text),
    )
        .into_response()
}
