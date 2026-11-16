//! Run explicitly against a disposable PostgreSQL DB: TEST_DATABASE_URL=... cargo test --test accounts -- --ignored
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use puzzgen::{
    accounts::AccountStore,
    api::{AppState, Config, router},
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

const ORIGIN: &str = "http://127.0.0.1:8080";
const PASSWORD: &str = "a unique integration test passphrase";

// Real TCP requests, synchronized before sending, rather than sequential router calls.
async fn burst(
    addresses: &[std::net::SocketAddr],
    keys: &[String],
    count: usize,
    body: Option<Value>,
) -> Vec<(u16, Value)> {
    use std::io::{Read, Write};
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(count));
    let mut tasks = tokio::task::JoinSet::new();
    for i in 0..count {
        let address = addresses[i % addresses.len()];
        let key = keys[i % keys.len()].clone();
        let barrier = barrier.clone();
        let body = body.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            tokio::task::spawn_blocking(move || {
                let mut stream = std::net::TcpStream::connect(address).unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(15))).unwrap();
                let (method, path, payload) = match body {
                    Some(body) => ("POST", "/api/v1/batch", body.to_string()),
                    None => ("GET", "/api/v1/puzzles/arithmetic?seed=42", String::new()),
                };
                write!(stream, "{method} {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {key}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}", payload.len()).unwrap();
                let mut raw = String::new();
                stream.read_to_string(&mut raw).unwrap();
                let (headers, body) = raw.split_once("\r\n\r\n").unwrap();
                let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
                if status == 429 {
                    assert!(headers.to_ascii_lowercase().contains("retry-after: 60"));
                }
                (status, serde_json::from_str(body).unwrap())
            }).await.unwrap()
        });
    }
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result.unwrap());
    }
    results
}

fn check_burst(
    report: &mut Vec<Value>,
    name: &str,
    results: Vec<(u16, Value)>,
    accepted: usize,
    reason: &str,
) {
    let ok = results.iter().filter(|(status, _)| *status == 200).count();
    let limited = results.iter().filter(|(status, _)| *status == 429).count();
    assert_eq!(ok, accepted, "{name}: {results:?}");
    assert_eq!(limited, results.len() - accepted, "{name}: {results:?}");
    for (status, body) in &results {
        if *status == 429 {
            assert!(
                body["error"]["message"].as_str().unwrap().contains(reason),
                "{name}: {body}"
            );
        }
    }
    println!("{name}: {ok} accepted, {limited} limited");
    report.push(json!({"scenario":name,"requests":results.len(),"accepted":ok,"limited":limited}));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires a disposable PostgreSQL database with TEST_DATABASE_URL"]
async fn concurrent_http_quotas_and_admin_overrides() {
    let url = std::env::var("TEST_DATABASE_URL").unwrap();
    let mut parsed = url::Url::parse(&url).unwrap();
    assert!(parsed.path().ends_with("_test"));
    let control = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let schema = format!("quota_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&control)
        .await
        .unwrap();
    parsed
        .query_pairs_mut()
        .append_pair("options", &format!("-c search_path={schema}"));
    let store = AccountStore::connect(parsed.as_str(), ORIGIN, true)
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store
        .bootstrap_owner("owner@example.test", "Owner", PASSWORD.into())
        .await
        .unwrap();
    let make_app = || {
        router(AppState::with_accounts(
            Config {
                workers: 64,
                ..Config::default()
            },
            Some(store.clone()),
        ))
    };
    let app = make_app();
    let (_, value, cookie) = request(
        &app,
        "POST",
        "/api/v1/account/login",
        json!({"email":"owner@example.test","password":PASSWORD}),
        None,
        None,
    )
    .await;
    let owner = actor(&value, cookie);
    let (_, value, cookie) = request(
        &app,
        "POST",
        "/api/v1/account/register",
        json!({"email":"quota@example.test","display_name":"Quota probe","password":PASSWORD}),
        None,
        None,
    )
    .await;
    assert_eq!(value["user"]["role"], "member");
    assert_eq!(
        value["user"]["limits"],
        json!({"requests_per_minute":10,"operations_per_day":100,"max_batch":1,"max_keys":3,"max_concurrent":1})
    );
    let member = actor(&value, cookie);
    let (_, value, cookie) = request(
        &app,
        "POST",
        "/api/v1/account/register",
        json!({"email":"admin@example.test","display_name":"Admin probe","password":PASSWORD}),
        None,
        None,
    )
    .await;
    let admin = actor(&value, cookie);
    let admin_path = format!("/api/v1/account/admin/members/{}", admin.id);
    assert_eq!(
        request(
            &app,
            "POST",
            &admin_path,
            allocation("admin", None, None, None, None, None),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let path = format!("/api/v1/account/admin/members/{}", member.id);
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, None, None, None, None),
            Some(&member),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, key1) = create_key(&app, &member, "first").await;
    let (_, key2) = create_key(&app, &member, "second").await;
    let keys = vec![
        key1["secret"].as_str().unwrap().to_owned(),
        key2["secret"].as_str().unwrap().to_owned(),
    ];
    let (_, third) = create_key(&app, &member, "third").await;
    assert!(third["secret"].is_string());
    assert_eq!(
        create_key(&app, &member, "fourth blocked").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", Some(10), Some(100), Some(1), Some(4), Some(1)),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        create_key(&app, &member, "fourth after admin override")
            .await
            .0,
        StatusCode::OK
    );

    // Two independent server states share PostgreSQL quotas, not local counters.
    let mut addresses = Vec::new();
    let mut servers = Vec::new();
    for _ in 0..2 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        addresses.push(listener.local_addr().unwrap());
        let server = make_app();
        servers.push(tokio::spawn(async move {
            axum::serve(
                listener,
                server.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap();
        }));
    }
    let mut report = Vec::new();
    // Keep default daily/minute limits, lift only concurrency to isolate counter races.
    // Avoid crossing the wall-clock minute boundary during this short sequence.
    let seconds_left = 60 - time::OffsetDateTime::now_utc().second() as u64;
    if seconds_left < 5 {
        tokio::time::sleep(std::time::Duration::from_secs(seconds_left)).await;
    }
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", Some(10), Some(100), Some(1), Some(3), None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Minute limit 10 / 24 simultaneous",
        burst(&addresses, &keys, 24, None).await,
        10,
        "requests-per-minute",
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", Some(14), Some(100), Some(1), Some(3), None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Admin raises minute limit to 14",
        burst(&addresses, &keys, 10, None).await,
        4,
        "requests-per-minute",
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, Some(16), Some(3), Some(3), None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let batch = json!({"puzzles":[{"kind":"arithmetic","seed":1},{"kind":"arithmetic","seed":2},{"kind":"arithmetic","seed":3}]});
    check_burst(
        &mut report,
        "Batch cannot overdraw remaining 2 daily operations",
        burst(&addresses, &keys, 4, Some(batch.clone())).await,
        0,
        "daily puzzle-operation",
    );
    check_burst(
        &mut report,
        "Daily limit 16 / 14 already used",
        burst(&addresses, &keys, 12, None).await,
        2,
        "daily puzzle-operation",
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, Some(20), Some(3), Some(3), None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Admin raises daily limit to 20",
        burst(&addresses, &keys, 12, None).await,
        4,
        "daily puzzle-operation",
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, Some(25), Some(3), Some(3), None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Concurrent batches share 5 remaining operations",
        burst(&addresses, &keys, 8, Some(batch.clone())).await,
        1,
        "daily puzzle-operation",
    );
    let (_, session, _) = request(
        &app,
        "GET",
        "/api/v1/account/session",
        Value::Null,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(session["usage"]["operations_today"], 23);
    assert_eq!(session["usage"]["requests_this_minute"], 21);
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, None, Some(1), Some(3), None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Default batch size 1 enforced",
        burst(&addresses, &keys, 4, Some(batch.clone())).await,
        0,
        "batch limit",
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, None, Some(3), Some(3), None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Admin batch override permits size 3",
        burst(&addresses, &keys, 4, Some(batch)).await,
        4,
        "",
    );

    // Hold actual work leases so the concurrency boundary is deterministic even
    // when arithmetic finishes too quickly for simultaneous arrivals to overlap.
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, None, Some(1), Some(3), Some(1)),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {}", keys[0]).parse().unwrap(),
    );
    let principal = store.authenticate_compute(&headers, false).await.unwrap();
    let held = store.reserve(principal.clone(), 1, false).await.unwrap();
    check_burst(
        &mut report,
        "Default concurrency 1 with occupied slot",
        burst(&addresses, &keys, 12, None).await,
        0,
        "concurrent operation",
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, None, Some(1), Some(3), Some(2)),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Admin raises concurrency to 2 with one occupied slot",
        burst(&addresses, &keys, 1, None).await,
        1,
        "",
    );
    held.release().await.unwrap();
    // Simultaneous reservations stay held until all competitors have returned.
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(16));
    let mut contenders = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let store = store.clone();
        let principal = principal.clone();
        let barrier = barrier.clone();
        contenders.spawn(async move {
            barrier.wait().await;
            store.reserve(principal, 1, false).await
        });
    }
    let mut leases = Vec::new();
    let mut rejected = 0;
    while let Some(result) = contenders.join_next().await {
        match result.unwrap() {
            Ok(lease) => leases.push(lease),
            Err(error) => {
                use axum::response::IntoResponse;
                assert_eq!(
                    error.into_response().status(),
                    StatusCode::TOO_MANY_REQUESTS
                );
                rejected += 1;
            }
        }
    }
    assert_eq!(leases.len(), 2);
    assert_eq!(rejected, 14);
    report.push(json!({"scenario":"16 competing held reservations / concurrency 2","requests":16,"accepted":2,"limited":14}));
    for lease in leases {
        lease.release().await.unwrap();
    }
    check_burst(
        &mut report,
        "Lease release restores capacity",
        burst(&addresses, &keys, 1, None).await,
        1,
        "",
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", Some(0), None, Some(1), Some(3), Some(1)),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Admin zero limit disables compute",
        burst(&addresses, &keys, 1, None).await,
        0,
        "requests-per-minute",
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, None, None, None, None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    check_burst(
        &mut report,
        "Admin unlimited limits restore access on same keys",
        burst(&addresses, &keys, 16, None).await,
        16,
        "",
    );
    // Move the isolated test clock buckets back instead of waiting until midnight.
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", Some(10), Some(100), Some(1), Some(3), Some(1)),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE account_usage SET minute_start=now()-interval '2 minutes',minute_count=999,day_start=(now() AT TIME ZONE 'UTC')::date-1,day_count=999 WHERE account_id=$1").bind(member.id).execute(&store.pool).await.unwrap();
    check_burst(
        &mut report,
        "Expired minute/day counters reset",
        burst(&addresses, &keys, 1, None).await,
        1,
        "",
    );
    let (_, session, _) = request(
        &app,
        "GET",
        "/api/v1/account/session",
        Value::Null,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(session["usage"]["operations_today"], 1);
    assert_eq!(session["usage"]["requests_this_minute"], 1);
    // Exhaust the actual default daily allocation. Lift the other limits only
    // so neither minute nor concurrency rejection masks the daily boundary.
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("member", None, Some(100), Some(1), Some(3), None),
            Some(&admin),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut daily_results = Vec::new();
    for _ in 0..5 {
        daily_results.extend(burst(&addresses, &keys, 24, None).await);
    }
    check_burst(
        &mut report,
        "Default daily 100 / 1 already used / 120 contenders",
        daily_results,
        99,
        "daily puzzle-operation",
    );
    let (_, session, _) = request(
        &app,
        "GET",
        "/api/v1/account/session",
        Value::Null,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(session["usage"]["operations_today"], 100);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM account_work")
            .fetch_one(&store.pool)
            .await
            .unwrap(),
        0
    );
    for server in servers {
        server.abort();
        let _ = server.await;
    }
    store.pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&control)
        .await
        .unwrap();
    control.close().await;
    if let Ok(path) = std::env::var("QUOTA_REPORT_PATH") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
}
#[derive(Clone)]
struct Actor {
    cookie: String,
    csrf: String,
    id: Uuid,
}
async fn request(
    app: &Router,
    method: &str,
    path: &str,
    body: Value,
    actor: Option<&Actor>,
    key: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("origin", ORIGIN);
    if let Some(actor) = actor {
        builder = builder
            .header("cookie", &actor.cookie)
            .header("x-csrf-token", &actor.csrf);
    }
    if let Some(key) = key {
        builder = builder.header("authorization", format!("Bearer {key}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(';').next().unwrap().to_owned());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"raw":String::from_utf8_lossy(&bytes)}));
    (status, value, cookie)
}
fn actor(value: &Value, cookie: Option<String>) -> Actor {
    Actor {
        cookie: cookie.unwrap(),
        csrf: value["csrf_token"].as_str().unwrap().into(),
        id: Uuid::parse_str(value["user"]["id"].as_str().unwrap()).unwrap(),
    }
}
fn allocation(
    role: &str,
    rpm: Option<i64>,
    daily: Option<i64>,
    batch: Option<i64>,
    keys: Option<i64>,
    concurrency: Option<i64>,
) -> Value {
    json!({"role":role,"disabled":false,"requests_per_minute":rpm,"operations_per_day":daily,"max_batch":batch,"max_keys":keys,"max_concurrent":concurrency})
}
async fn create_key(app: &Router, user: &Actor, name: &str) -> (StatusCode, Value) {
    let (status, value, _) = request(
        app,
        "POST",
        "/api/v1/account/keys",
        json!({"name":name}),
        Some(user),
        None,
    )
    .await;
    (status, value)
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database with TEST_DATABASE_URL"]
async fn postgres_enforces_identity_rbac_keys_and_shared_quotas() {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("Set TEST_DATABASE_URL to a disposable PostgreSQL database");
    let mut parsed = url::Url::parse(&url).unwrap();
    assert!(
        parsed.path().ends_with("_test"),
        "Test database name must end in _test"
    );
    let control = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    let schema = format!("account_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&control)
        .await
        .unwrap();
    parsed
        .query_pairs_mut()
        .append_pair("options", &format!("-c search_path={schema}"));
    let store = AccountStore::connect(parsed.as_str(), ORIGIN, true)
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let owner_id = store
        .bootstrap_owner("owner@example.test", "Owner", PASSWORD.into())
        .await
        .unwrap();
    assert!(
        store
            .bootstrap_owner("other-owner@example.test", "Other", PASSWORD.into())
            .await
            .is_err()
    );
    let app = router(AppState::with_accounts(
        Config {
            api_keys: vec!["legacy-key-with-32-characters-or-more".into()],
            ..Config::default()
        },
        Some(store.clone()),
    ));
    assert_eq!(
        request(&app, "GET", "/api/v1/puzzles/maze", Value::Null, None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/puzzles/maze",
            Value::Null,
            None,
            Some("legacy-key-with-32-characters-or-more")
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/catalog", Value::Null, None, None)
            .await
            .0,
        StatusCode::OK
    );

    // Signup never accepts a caller-supplied role or resource allocation.
    assert_eq!(request(&app,"POST","/api/v1/account/register",json!({"email":"member@example.test","display_name":"Member","password":PASSWORD,"role":"owner"}),None,None).await.0,StatusCode::BAD_REQUEST);
    let (status, value, cookie) = request(
        &app,
        "POST",
        "/api/v1/account/register",
        json!({"email":"  Member@Example.Test ","display_name":"Member","password":PASSWORD}),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["user"]["role"], "member");
    assert_eq!(value["user"]["limits"]["max_keys"], 3);
    assert!(!value.to_string().contains("password_hash"));
    let member = actor(&value, cookie);
    let (status, value, cookie) = request(
        &app,
        "POST",
        "/api/v1/account/login",
        json!({"email":"owner@example.test","password":PASSWORD}),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    let owner = actor(&value, cookie);
    assert_eq!(owner.id, owner_id);
    let (status, value, cookie) = request(
        &app,
        "POST",
        "/api/v1/account/register",
        json!({"email":"other@example.test","display_name":"Other","password":PASSWORD}),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    let other = actor(&value, cookie);
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/account/admin/members",
            Value::Null,
            Some(&member),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/v1/account/admin/members/{}", member.id),
            allocation("admin", None, None, None, None, None),
            Some(&member),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut no_csrf = member.clone();
    no_csrf.csrf.clear();
    assert_eq!(
        create_key(&app, &no_csrf, "bad").await.0,
        StatusCode::FORBIDDEN
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/account/login")
                .header("origin", "https://attacker.example")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"email":"owner@example.test","password":PASSWORD}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let (status, key1) = create_key(&app, &member, "one").await;
    assert_eq!(status, StatusCode::OK, "{key1}");
    let (status, key2) = create_key(&app, &member, "two").await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = create_key(&app, &member, "three").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        create_key(&app, &member, "four").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    let secret1 = key1["secret"].as_str().unwrap();
    let secret2 = key2["secret"].as_str().unwrap();
    let (_, list, _) = request(
        &app,
        "GET",
        "/api/v1/account/keys",
        Value::Null,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(list["keys"].as_array().unwrap().len(), 3);
    assert!(!list.to_string().contains(secret1));
    let stored: String = sqlx::query_scalar("SELECT token_hash FROM member_api_keys WHERE id=$1")
        .bind(Uuid::parse_str(key1["id"].as_str().unwrap()).unwrap())
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(stored.len(), 64);
    assert_ne!(stored, secret1);
    assert_eq!(
        request(
            &app,
            "POST",
            &format!(
                "/api/v1/account/keys/{}/revoke",
                key1["id"].as_str().unwrap()
            ),
            json!({}),
            Some(&other),
            None
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!(
                "/api/v1/account/keys/{}/revoke",
                key1["id"].as_str().unwrap()
            ),
            json!({}),
            Some(&member),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/puzzles/maze",
            Value::Null,
            None,
            Some(secret1)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        create_key(&app, &member, "replacement").await.0,
        StatusCode::OK
    );

    // Competing creations cannot both claim the third slot.
    assert_eq!(create_key(&app, &other, "a").await.0, StatusCode::OK);
    assert_eq!(create_key(&app, &other, "b").await.0, StatusCode::OK);
    let (a, b) = tokio::join!(create_key(&app, &other, "c"), create_key(&app, &other, "d"));
    let mut statuses = [a.0.as_u16(), b.0.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 429]);
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM member_api_keys WHERE account_id=$1 AND revoked_at IS NULL",
    )
    .bind(other.id)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    assert_eq!(count, 3);

    // Changing an allocation takes effect without issuing a new key or session.
    let path = format!("/api/v1/account/admin/members/{}", member.id);
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("trusted", Some(2), Some(4), Some(2), Some(3), Some(1)),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    for seed in [1, 2] {
        assert_eq!(
            request(
                &app,
                "GET",
                &format!("/api/v1/puzzles/arithmetic?seed={seed}"),
                Value::Null,
                None,
                Some(secret2)
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/puzzles/arithmetic",
            Value::Null,
            Some(&member),
            None
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("trusted", None, Some(4), Some(2), Some(3), Some(1)),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let batch = json!({"puzzles":[{"kind":"arithmetic","seed":1},{"kind":"arithmetic","seed":2}]});
    let (status, value, _) =
        request(&app, "POST", "/api/v1/batch", batch, None, Some(secret2)).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/puzzles/maze",
            Value::Null,
            None,
            Some(secret2)
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    let (_, session, _) = request(
        &app,
        "GET",
        "/api/v1/account/session",
        Value::Null,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(session["usage"]["operations_today"], 4);
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("trusted", None, None, Some(2), Some(3), Some(1)),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {secret2}").parse().unwrap(),
    );
    let principal = store.authenticate_compute(&headers, false).await.unwrap();
    let lease = store.reserve(principal.clone(), 1, false).await.unwrap();
    assert!(store.reserve(principal, 1, false).await.is_err());
    drop(lease);
    for _ in 0..100 {
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM account_work WHERE account_id=$1")
                .bind(member.id)
                .fetch_one(&store.pool)
                .await
                .unwrap();
        if count == 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM account_work WHERE account_id=$1")
        .bind(member.id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);

    // Expired UTC day/minute counters reset even when old counts exceed new limits.
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("trusted", Some(2), Some(4), Some(2), Some(3), Some(1)),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE account_usage SET minute_start=now()-interval '2 minutes',minute_count=999,day_start=(now() AT TIME ZONE 'UTC')::date-1,day_count=999 WHERE account_id=$1").bind(member.id).execute(&store.pool).await.unwrap();
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/puzzles/arithmetic?seed=42",
            Value::Null,
            None,
            Some(secret2)
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, refreshed, _) = request(
        &app,
        "GET",
        "/api/v1/account/session",
        Value::Null,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(refreshed["usage"]["operations_today"], 1);
    assert_eq!(refreshed["usage"]["requests_this_minute"], 1);

    // Owner protection and administrator delegation are server-side rules.
    let owner_path = format!("/api/v1/account/admin/members/{}", owner.id);
    assert_eq!(
        request(
            &app,
            "POST",
            &owner_path,
            allocation("member", None, None, None, None, None),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let mut disable_owner = allocation("owner", None, None, None, None, None);
    disable_owner["disabled"] = json!(true);
    assert_eq!(
        request(&app, "POST", &owner_path, disable_owner, Some(&owner), None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/v1/account/admin/members/{}", other.id),
            allocation("admin", None, None, None, None, None),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &owner_path,
            allocation("owner", Some(1), None, None, None, None),
            Some(&other),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("trusted", Some(20), Some(200), Some(2), Some(3), Some(1)),
            Some(&other),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/v1/account/admin/members/{}", other.id),
            allocation("admin", None, None, None, None, None),
            Some(&other),
            None
        )
        .await
        .0,
        StatusCode::OK
    );

    let (status, reduction, _) = request(
        &app,
        "POST",
        &format!("/api/v1/account/admin/members/{}", other.id),
        allocation("admin", None, None, None, Some(1), None),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reduction["keys_revoked"], 2);
    let (_, reduced, _) = request(
        &app,
        "GET",
        "/api/v1/account/keys",
        Value::Null,
        Some(&other),
        None,
    )
    .await;
    assert_eq!(reduced["keys"].as_array().unwrap().len(), 1);
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/account/admin/members",
            Value::Null,
            None,
            Some(secret2)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );

    let new_password = "another unique integration passphrase";
    let (status, value, cookie) = request(
        &app,
        "POST",
        "/api/v1/account/password",
        json!({"current_password":PASSWORD,"new_password":new_password}),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    let member_new = actor(&value, cookie);
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/account/session",
            Value::Null,
            Some(&member),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/account/login",
            json!({"email":"member@example.test","password":PASSWORD}),
            None,
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/account/login",
            json!({"email":"member@example.test","password":new_password}),
            None,
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut disabled = allocation("trusted", Some(20), Some(200), Some(2), Some(3), Some(1));
    disabled["disabled"] = json!(true);
    assert_eq!(
        request(&app, "POST", &path, disabled, Some(&owner), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/account/session",
            Value::Null,
            Some(&member_new),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/puzzles/maze",
            Value::Null,
            None,
            Some(secret2)
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let (_, audit, _) = request(
        &app,
        "GET",
        "/api/v1/account/admin/audit",
        Value::Null,
        Some(&owner),
        None,
    )
    .await;
    assert!(!audit.to_string().contains(secret2));
    assert!(!audit.to_string().contains(PASSWORD));
    assert!(
        audit["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["action"] == "member_updated")
    );
    store
        .reset_password(
            "other@example.test",
            "a different operator recovery phrase".into(),
        )
        .await
        .unwrap();
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/account/session",
            Value::Null,
            Some(&other),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM member_api_keys WHERE account_id=$1 AND revoked_at IS NULL",
    )
    .bind(other.id)
    .fetch_one(&store.pool)
    .await
    .unwrap();
    assert_eq!(active, 0);

    assert_eq!(
        request(
            &app,
            "POST",
            &path,
            allocation("trusted", Some(20), Some(200), Some(2), Some(3), Some(1)),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, value, cookie) = request(
        &app,
        "POST",
        "/api/v1/account/login",
        json!({"email":"member@example.test","password":new_password}),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let deletion_actor = actor(&value, cookie);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/account/delete",
            json!({"password":new_password,"confirmation":"DELETE"}),
            Some(&deletion_actor),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=$1)")
        .bind(member.id)
        .fetch_one(&store.pool)
        .await
        .unwrap();
    assert!(!exists);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/v1/account/delete",
            json!({"password":PASSWORD,"confirmation":"DELETE"}),
            Some(&owner),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    store.cleanup().await.unwrap();
    store.pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&control)
        .await
        .unwrap();
    control.close().await;
}
