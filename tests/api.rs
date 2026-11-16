use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use puzzgen::api::{AppState, Config, router};
use serde_json::{Value, json};
use tower::ServiceExt;

fn app() -> Router {
    router(AppState::new(Config::default()))
}

async fn request(
    app: Router,
    method: &str,
    uri: &str,
    body: Value,
    key: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(key) = key {
        builder = builder.header("authorization", format!("Bearer {key}"));
    }
    let response = app
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({"raw":String::from_utf8_lossy(&bytes)})),
    )
}

#[tokio::test]
async fn public_catalog_and_document_cover_all_real_generators() {
    let (status, catalog) = request(app(), "GET", "/api/v1/catalog", Value::Null, None).await;
    assert_eq!(status, StatusCode::OK);
    let kinds = catalog["kinds"].as_array().unwrap();
    assert_eq!(kinds.len(), 21);
    for entry in kinds {
        let uri = format!(
            "/api/v1/puzzles/{}?seed=42&difficulty=easy",
            entry["kind"].as_str().unwrap()
        );
        let (status, puzzle) = request(app(), "GET", &uri, Value::Null, None).await;
        assert_eq!(status, StatusCode::OK, "{puzzle}");
        assert!(puzzle.get("solution").is_none());
        assert_eq!(puzzle["seed"], "42");
    }
    let (status, doc) = request(app(), "GET", "/api/v1/openapi.json", Value::Null, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(doc["openapi"], "3.1.0");
    assert_eq!(
        doc["components"]["schemas"]["GenerateRequest"]["properties"]["kind"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        kinds.len()
    );
}

#[tokio::test]
async fn seeded_generation_replays_and_every_solution_verifies() {
    let (_, catalog) = request(app(), "GET", "/api/v1/catalog", Value::Null, None).await;
    for item in catalog["kinds"].as_array().unwrap() {
        let kind = item["kind"].clone();
        let query =
            json!({"kind":kind,"seed":u64::MAX,"difficulty":"medium","include_solution":true});
        let (status, first) = request(app(), "POST", "/api/v1/generate", query.clone(), None).await;
        assert_eq!(status, StatusCode::OK, "{first}");
        let (_, second) = request(app(), "POST", "/api/v1/generate", query, None).await;
        assert_eq!(first, second);
        let (status, verification) = request(
            app(),
            "POST",
            "/api/v1/verify",
            json!({"kind":kind,"seed":u64::MAX,"difficulty":"medium","answer":first["solution"]}),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{verification}");
        assert_eq!(verification["correct"], true, "{kind}: {first}");
    }
}

#[tokio::test]
async fn daily_uses_valid_utc_dates_and_replayable_seeds() {
    let api = app();
    let uri = "/api/v1/daily/maze?date=2028-02-29&difficulty=hard";
    let (status, first) = request(api.clone(), "GET", uri, Value::Null, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["date"], "2028-02-29");
    assert_eq!(first["timezone"], "UTC");
    let (_, again) = request(api.clone(), "GET", uri, Value::Null, None).await;
    assert_eq!(first, again);
    let replay = format!(
        "/api/v1/puzzles/maze?seed={}&difficulty=hard",
        first["puzzle"]["seed"].as_str().unwrap()
    );
    let (_, replayed) = request(api.clone(), "GET", &replay, Value::Null, None).await;
    assert_eq!(replayed, first["puzzle"]);
    let (status, error) = request(
        api,
        "GET",
        "/api/v1/daily/maze?date=2027-02-29",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn keys_protect_compute_and_metrics_but_leave_discovery_public() {
    let token = "test-key-with-at-least-32-characters";
    let api = router(AppState::new(Config {
        api_keys: vec![token.into()],
        ..Config::default()
    }));
    for uri in ["/api/v1/puzzles/maze", "/metrics"] {
        let (status, _) = request(api.clone(), "GET", uri, Value::Null, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = request(api.clone(), "GET", uri, Value::Null, Some("wrong")).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = request(api.clone(), "GET", uri, Value::Null, Some(token)).await;
        assert_eq!(status, StatusCode::OK);
    }
    for uri in [
        "/api/v1/catalog",
        "/health/live",
        "/health/ready",
        "/api/v1/openapi.json",
    ] {
        let (status, _) = request(api.clone(), "GET", uri, Value::Null, None).await;
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn teaching_fixture_matches_its_seed_and_version() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../frontend/examples/fixtures/arithmetic.json"
    ))
    .unwrap();
    let (status, live) = request(
        app(),
        "GET",
        "/api/v1/puzzles/arithmetic?seed=42&difficulty=medium",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        live, fixture,
        "Update lesson fixtures deliberately if generator behavior changes"
    );
}

#[tokio::test]
async fn learners_can_make_cross_origin_json_requests() {
    let api = router(AppState::new(Config {
        api_keys: vec!["test-key-with-at-least-32-characters".into()],
        ..Config::default()
    }));
    let response = api
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/verify")
                .header("origin", "https://learner.example")
                .header("access-control-request-method", "POST")
                .header(
                    "access-control-request-headers",
                    "content-type,authorization",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["access-control-allow-origin"], "*");
    assert!(
        response
            .headers()
            .get("access-control-allow-credentials")
            .is_none()
    );
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/puzzles/arithmetic?seed=42")
                .header("origin", "https://learner.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["access-control-allow-origin"], "*");
    assert_eq!(
        response.headers()["access-control-expose-headers"],
        "retry-after"
    );
}

#[tokio::test]
async fn bearer_scheme_is_case_insensitive_and_token_is_not() {
    let token = "test-key-with-at-least-32-characters";
    let api = router(AppState::new(Config {
        api_keys: vec![token.into()],
        ..Config::default()
    }));
    for scheme in ["bearer", "BEARER", "BeArEr"] {
        let response = api
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/puzzles/arithmetic?seed=42")
                    .header("authorization", format!("{scheme} {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let (status, _) = request(
        api,
        "GET",
        "/api/v1/puzzles/arithmetic",
        Value::Null,
        Some(&token.to_uppercase()),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_requests_stay_json_and_batches_are_bounded() {
    let api = app();
    for body in [
        json!({"kind":"not-real"}),
        json!({"kind":"sudoku","seed":-1}),
        json!({"kind":"sudoku","difficulty":"extreme"}),
        json!({"kind":"sudoku","unexpected":true}),
    ] {
        let (status, value) = request(api.clone(), "POST", "/api/v1/generate", body, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(value["error"]["code"], "invalid_json");
    }
    for uri in [
        "/api/v1/puzzles/nope",
        "/api/v1/puzzles/%FF",
        "/api/v1/daily/%FF",
        "/api/v1/puzzles/maze?seed=-1",
        "/api/v1/puzzles/maze?difficulty=unknown",
        "/api/v1/puzzles/maze?size=1000000",
    ] {
        let (status, value) = request(api.clone(), "GET", uri, Value::Null, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {value}");
        assert!(value["error"]["message"].is_string());
    }
    for count in [0, 17] {
        let (status, _) = request(
            api.clone(),
            "POST",
            "/api/v1/batch",
            json!({"puzzles":vec![json!({"kind":"maze"});count]}),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, data) = request(
        api.clone(),
        "POST",
        "/api/v1/batch",
        json!({"puzzles":[{"kind":"maze","seed":1},{"kind":"arithmetic","seed":2}]}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(data["puzzles"].as_array().unwrap().len(), 2);
    let (status, error) = request(api.clone(), "GET", "/api/v1/nope", Value::Null, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error["error"]["code"], "not_found");
    let (status, _) = request(api, "DELETE", "/api/v1/generate", Value::Null, None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn rate_and_body_limits_are_enforced() {
    let api = router(AppState::new(Config {
        requests_per_minute: 1,
        ..Config::default()
    }));
    let (status, _) = request(
        api.clone(),
        "GET",
        "/api/v1/puzzles/maze",
        Value::Null,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let response = api
        .oneshot(
            Request::builder()
                .uri("/api/v1/puzzles/maze")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()["retry-after"], "60");
    let (status, error) = request(
        app(),
        "POST",
        "/api/v1/generate",
        json!({"kind":"maze","oversized":"x".repeat(70_000)}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{error}");
}

#[tokio::test]
async fn language_batch_daily_and_auth_share_existing_routes() {
    let token = "language-test-key-with-at-least-32-characters";
    let api = router(AppState::new(Config {
        api_keys: vec![token.into()],
        ..Config::default()
    }));
    let kinds = ["spoonerism", "anagram", "word-ladder", "compound-bridge"];
    for kind in kinds {
        let uri = format!("/api/v1/daily/{kind}?date=2026-11-16&difficulty=hard");
        let (status, _) = request(api.clone(), "GET", &uri, Value::Null, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, first) = request(api.clone(), "GET", &uri, Value::Null, Some(token)).await;
        assert_eq!(status, StatusCode::OK, "{first}");
        let (_, again) = request(api.clone(), "GET", &uri, Value::Null, Some(token)).await;
        assert_eq!(first, again);
        assert!(first["puzzle"].get("solution").is_none());
        assert_eq!(first["puzzle"]["data"]["content_license"], "CC0-1.0");
    }
    let (status, batch) = request(
        api,
        "POST",
        "/api/v1/batch",
        json!({"puzzles":kinds.map(|kind|json!({"kind":kind,"seed":42}))}),
        Some(token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{batch}");
    assert_eq!(batch["puzzles"].as_array().unwrap().len(), 4);
    assert!(
        batch["puzzles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p.get("solution").is_none())
    );
}

#[tokio::test]
async fn language_versions_replay_and_verify_their_original_content() {
    let fixtures: Value = serde_json::from_str(
        include_str!("../frontend/examples/fixtures/language.json").trim_start_matches('\u{feff}'),
    )
    .unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let version = fixture["data"]["content_version"].as_str().unwrap();
        let kind = fixture["kind"].as_str().unwrap();
        let (status,replay)=request(app(),"POST","/api/v1/generate",json!({"kind":kind,"seed":42,"difficulty":"medium","include_solution":true,"content_version":version}),None).await;
        assert_eq!(status, StatusCode::OK, "{replay}");
        assert_eq!(replay, *fixture);
        let (status,checked)=request(app(),"POST","/api/v1/verify",json!({"kind":kind,"seed":42,"difficulty":"medium","answer":fixture["solution"],"content_version":version}),None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(checked["correct"], true);
        let uri = format!(
            "/api/v1/puzzles/{kind}?seed=42&difficulty=medium&include_solution=true&content_version={version}"
        );
        let (_, get) = request(app(), "GET", &uri, Value::Null, None).await;
        assert_eq!(get, replay);
        let (_, daily) = request(
            app(),
            "GET",
            &format!("/api/v1/daily/{kind}?date=2026-11-16&content_version={version}"),
            Value::Null,
            None,
        )
        .await;
        assert_eq!(daily["puzzle"]["data"]["content_version"], version);
    }
    for query in [
        json!({"kind":"spoonerism","seed":42,"content_version":"999"}),
        json!({"kind":"sudoku","content_version":"1"}),
    ] {
        let (status, _) = request(app(), "POST", "/api/v1/generate", query, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (_, default) = request(
        app(),
        "POST",
        "/api/v1/generate",
        json!({"kind":"spoonerism","seed":42}),
        None,
    )
    .await;
    assert_eq!(default["data"]["content_version"], "2");
    assert!(default["id"].as_str().unwrap().ends_with("-content2"));
    assert!(default["data"]["learning"]["objective"].is_string());
    assert!(default.get("solution").is_none());
}

#[tokio::test]
async fn discovery_games_share_auth_batch_daily_and_verified_answers() {
    let token = "discovery-test-key-with-at-least-32-characters";
    let api = router(AppState::new(Config {
        api_keys: vec![token.into()],
        ..Config::default()
    }));
    let kinds = [
        "word-sorting",
        "sequence-detective",
        "memory-pairs",
        "path-connections",
        "futoshiki",
        "sound-families",
        "sentence-assembly",
    ];
    let queries = kinds
        .map(|kind| json!({"kind":kind,"seed":42,"difficulty":"medium","include_solution":true}));
    let (status, _) = request(
        api.clone(),
        "POST",
        "/api/v1/batch",
        json!({"puzzles":queries}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, batch) = request(
        api.clone(),
        "POST",
        "/api/v1/batch",
        json!({"puzzles":queries}),
        Some(token),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{batch}");
    assert_eq!(batch["puzzles"].as_array().unwrap().len(), 7);
    for puzzle in batch["puzzles"].as_array().unwrap() {
        let (status,result)=request(api.clone(),"POST","/api/v1/verify",json!({"kind":puzzle["kind"],"seed":42,"difficulty":"medium","answer":puzzle["solution"]}),Some(token)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(result["correct"], true, "{puzzle}");
        let kind = puzzle["kind"].as_str().unwrap();
        let uri = format!("/api/v1/daily/{kind}?date=2026-11-16");
        let (status, daily) = request(api.clone(), "GET", &uri, Value::Null, Some(token)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(daily["puzzle"].get("solution").is_none());
        assert!(daily["puzzle"]["data"]["learning"]["objective"].is_string());
    }
}
