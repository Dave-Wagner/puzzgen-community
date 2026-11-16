//! Closed-loop HTTP benchmark against a separate release binary and isolated DB schema.
//! TEST_DATABASE_URL must name a disposable local *_test database. No keys are logged.
use puzzgen::accounts::AccountStore;
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::{Arc, Barrier},
    time::{Duration, Instant},
};
use uuid::Uuid;

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct Http(BufReader<TcpStream>);
struct Response {
    status: u16,
    cookie: Option<String>,
    body: Vec<u8>,
}
impl Http {
    fn connect(address: SocketAddr) -> std::io::Result<Self> {
        let socket = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
        socket.set_nodelay(true)?;
        socket.set_read_timeout(Some(Duration::from_secs(20)))?;
        socket.set_write_timeout(Some(Duration::from_secs(20)))?;
        Ok(Self(BufReader::new(socket)))
    }
    fn send(
        &mut self,
        address: SocketAddr,
        path: &str,
        headers: &str,
        body: Option<&str>,
    ) -> std::io::Result<Response> {
        let method = if body.is_some() { "POST" } else { "GET" };
        let body = body.unwrap_or("");
        write!(
            self.0.get_mut(),
            "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{headers}\r\n{body}",
            body.len()
        )?;
        let mut line = String::new();
        self.0.read_line(&mut line)?;
        let status = line
            .split_whitespace()
            .nth(1)
            .and_then(|v| v.parse().ok())
            .ok_or_else(|| std::io::Error::other("Invalid HTTP status"))?;
        let mut length = None;
        let mut cookie = None;
        loop {
            line.clear();
            if self.0.read_line(&mut line)? == 0 {
                return Err(std::io::Error::other("Incomplete headers"));
            }
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse::<usize>().ok();
                }
                if name.eq_ignore_ascii_case("set-cookie") {
                    cookie = Some(value.trim().split(';').next().unwrap().to_owned());
                }
            }
        }
        let length = length
            .ok_or_else(|| std::io::Error::other("Benchmark expects Content-Length responses"))?;
        let mut body = vec![0; length];
        self.0.read_exact(&mut body)?;
        Ok(Response {
            status,
            cookie,
            body,
        })
    }
}
fn account(
    http: &mut Http,
    address: SocketAddr,
    path: &str,
    body: Value,
    session: Option<&(String, String)>,
) -> (Value, Option<String>) {
    let mut headers = format!("Origin: http://{address}\r\n");
    if let Some((cookie, csrf)) = session {
        headers.push_str(&format!("Cookie: {cookie}\r\nX-CSRF-Token: {csrf}\r\n"));
    }
    let response = http
        .send(address, path, &headers, Some(&body.to_string()))
        .unwrap();
    assert_eq!(response.status, 200, "Account setup failed at {path}");
    (
        serde_json::from_slice(&response.body).unwrap(),
        response.cookie,
    )
}

#[derive(Default)]
struct Samples {
    success: Vec<f64>,
    rejected: Vec<f64>,
    statuses: BTreeMap<u16, usize>,
    transport_errors: usize,
    bytes: usize,
}
fn percentile(samples: &[f64], fraction: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples[((samples.len() as f64 * fraction).ceil() as usize)
        .saturating_sub(1)
        .min(samples.len() - 1)]
}
fn run(address: SocketAddr, key: &str, workload: &str, concurrency: usize, seconds: u64) -> Value {
    let barrier = Arc::new(Barrier::new(concurrency + 1));
    let duration = Duration::from_secs(seconds);
    let kinds = [
        "sudoku",
        "mini-sudoku",
        "maze",
        "nonogram",
        "word-search",
        "sliding-puzzle",
        "lights-out",
        "minesweeper",
        "arithmetic",
        "cryptogram",
    ];
    let tasks: Vec<_> = (0..concurrency)
        .map(|worker| {
            let barrier = barrier.clone();
            let headers = format!("Authorization: Bearer {key}\r\n");
            let workload = workload.to_owned();
            std::thread::spawn(move || {
                let mut http = Http::connect(address).ok();
                let mut samples = Samples::default();
                barrier.wait();
                let start = Instant::now();
                let mut sequence = 0;
                while start.elapsed() < duration {
                    let kind = if workload == "mixed" {
                        kinds[(sequence + worker) % kinds.len()]
                    } else {
                        &workload
                    };
                    let seed = sequence * concurrency + worker;
                    let path = format!("/api/v1/puzzles/{kind}?seed={seed}&difficulty=medium");
                    let began = Instant::now();
                    if http.is_none() {
                        http = Http::connect(address).ok();
                    }
                    let response = http
                        .as_mut()
                        .ok_or_else(|| std::io::Error::other("Connection failed"))
                        .and_then(|http| http.send(address, &path, &headers, None));
                    let ms = began.elapsed().as_secs_f64() * 1000.0;
                    match response {
                        Ok(response) => {
                            *samples.statuses.entry(response.status).or_default() += 1;
                            samples.bytes += response.body.len();
                            if response.status == 200 {
                                samples.success.push(ms);
                            } else {
                                samples.rejected.push(ms);
                            }
                        }
                        Err(_) => {
                            samples.transport_errors += 1;
                            http = None;
                        }
                    }
                    sequence += 1;
                }
                samples
            })
        })
        .collect();
    let began = Instant::now();
    barrier.wait();
    let mut total = Samples::default();
    for task in tasks {
        let mut samples = task.join().unwrap();
        total.success.append(&mut samples.success);
        total.rejected.append(&mut samples.rejected);
        total.transport_errors += samples.transport_errors;
        total.bytes += samples.bytes;
        for (status, count) in samples.statuses {
            *total.statuses.entry(status).or_default() += count;
        }
    }
    let elapsed = began.elapsed().as_secs_f64();
    total.success.sort_by(f64::total_cmp);
    total.rejected.sort_by(f64::total_cmp);
    let successes = total.success.len();
    let requests = successes + total.rejected.len() + total.transport_errors;
    json!({"workload":workload,"concurrency":concurrency,"target_seconds":seconds,"elapsed_seconds":elapsed,
        "requests":requests,"successes":successes,"success_rps":successes as f64/elapsed,"all_rps":requests as f64/elapsed,
        "error_percent":100.0*(requests-successes) as f64/requests.max(1) as f64,"statuses":total.statuses,"transport_errors":total.transport_errors,
        "mean_ms":total.success.iter().sum::<f64>()/successes.max(1) as f64,"p50_ms":percentile(&total.success,0.50),"p95_ms":percentile(&total.success,0.95),"p99_ms":percentile(&total.success,0.99),"max_ms":total.success.last().copied().unwrap_or(0.0),
        "rejected_p95_ms":percentile(&total.rejected,0.95),"response_bytes":total.bytes})
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database = std::env::var("TEST_DATABASE_URL")?;
    let mut parsed = url::Url::parse(&database)?;
    if !parsed.path().ends_with("_test")
        || !matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
    {
        return Err("Requires a disposable loopback *_test database".into());
    }
    let seconds = std::env::var("LOAD_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6_u64)
        .clamp(1, 30);
    let workers = std::env::var("LOAD_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4_usize)
        .clamp(1, 256);
    let root = std::env::current_dir()?;
    let output = root.join(".artifacts/release-load");
    std::fs::create_dir_all(&output)?;
    let control = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database)
        .await?;
    let schema = format!("load_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&control)
        .await?;
    parsed
        .query_pairs_mut()
        .append_pair("options", &format!("-c search_path={schema}"));
    let address = std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?;
    let origin = format!("http://{address}");
    let store = AccountStore::connect(parsed.as_str(), &origin, true).await?;
    store.migrate().await?;
    let password = format!("benchmark-{}", Uuid::new_v4());
    store
        .bootstrap_owner("owner@load.test", "Load owner", password.clone())
        .await
        .map_err(|_| "Could not create isolated benchmark owner")?;
    let binary = root.join(if cfg!(windows) {
        "target/release/puzzgen.exe"
    } else {
        "target/release/puzzgen"
    });
    let mut command = Command::new(binary);
    command
        .env("DATABASE_URL", parsed.as_str())
        .env("PUZZGEN_LISTEN", address.to_string())
        .env("PUZZGEN_PUBLIC_ORIGIN", &origin)
        .env("PUZZGEN_WORKERS", workers.to_string())
        .env("PUZZGEN_REGISTRATION_OPEN", "true")
        .env("RUST_LOG", "warn")
        .env_remove("PUZZGEN_API_KEYS")
        .stdout(Stdio::from(std::fs::File::create(
            output.join("server.log"),
        )?))
        .stderr(Stdio::from(std::fs::File::create(
            output.join("server-errors.log"),
        )?));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let server = Server(command.spawn()?);
    let mut ready = false;
    for _ in 0..100 {
        if let Ok(mut http) = Http::connect(address)
            && http
                .send(address, "/health/ready", "", None)
                .is_ok_and(|r| r.status == 200)
        {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(ready, "Release server did not become ready");
    let mut http = Http::connect(address)?;
    let (owner, cookie) = account(
        &mut http,
        address,
        "/api/v1/account/login",
        json!({"email":"owner@load.test","password":password}),
        None,
    );
    let owner_session = (
        cookie.unwrap(),
        owner["csrf_token"].as_str().unwrap().to_owned(),
    );
    let (admin, cookie) = account(
        &mut http,
        address,
        "/api/v1/account/register",
        json!({"email":"admin@load.test","display_name":"Unlimited benchmark admin","password":password}),
        None,
    );
    let admin_session = (
        cookie.unwrap(),
        admin["csrf_token"].as_str().unwrap().to_owned(),
    );
    account(
        &mut http,
        address,
        &format!(
            "/api/v1/account/admin/members/{}",
            admin["user"]["id"].as_str().unwrap()
        ),
        json!({"role":"admin","disabled":false,"requests_per_minute":null,"operations_per_day":null,"max_batch":null,"max_keys":null,"max_concurrent":null}),
        Some(&owner_session),
    );
    let verified = http.send(
        address,
        "/api/v1/account/session",
        &format!("Cookie: {}\r\n", admin_session.0),
        None,
    )?;
    let verified: Value = serde_json::from_slice(&verified.body)?;
    assert_eq!(verified["user"]["role"], "admin");
    assert!(
        verified["user"]["limits"]
            .as_object()
            .unwrap()
            .values()
            .all(Value::is_null)
    );
    let (key, _) = account(
        &mut http,
        address,
        "/api/v1/account/keys",
        json!({"name":"Disposable release load key"}),
        Some(&admin_session),
    );
    let key = key["secret"].as_str().unwrap();
    println!(
        "Release server ready: {workers} CPU worker slots, PostgreSQL pool 5, unlimited admin key verified; {} logical CPUs",
        std::thread::available_parallelism()?.get()
    );
    let mut results = Vec::new();
    let metadata = json!({"profile":"release","workers":workers,"postgres_pool":5,"logical_cpus":std::thread::available_parallelism()?.get(),"key_role":"admin","all_account_quotas":null,"duration_per_trial_seconds":seconds,"trials":2,"connection":"HTTP/1.1 keep-alive, no compression","model":"closed-loop, client and server on same Windows host","seed_policy":"varies by worker and request; repeats across trials","difficulty":"medium","solutions_included":false,"mixed_kinds":10});
    for workload in ["arithmetic", "sudoku", "mixed"] {
        for concurrency in [1, 2, 4, 8, 16, 32] {
            let warm_key = key.to_owned();
            let warm_workload = workload.to_owned();
            tokio::task::spawn_blocking(move || {
                run(address, &warm_key, &warm_workload, concurrency, 1)
            })
            .await?;
            for trial in 1..=2 {
                let key = key.to_owned();
                let workload = workload.to_owned();
                let mut row = tokio::task::spawn_blocking(move || {
                    run(address, &key, &workload, concurrency, seconds)
                })
                .await?;
                row["trial"] = json!(trial);
                println!(
                    "{} c={concurrency} trial={trial}: {:.1} successful req/s, p50={:.2}ms p95={:.2}ms, errors={:.1}% {:?}",
                    row["workload"].as_str().unwrap(),
                    row["success_rps"].as_f64().unwrap(),
                    row["p50_ms"].as_f64().unwrap(),
                    row["p95_ms"].as_f64().unwrap(),
                    row["error_percent"].as_f64().unwrap(),
                    row["statuses"]
                );
                results.push(row);
                std::fs::write(
                    output.join("results.json"),
                    serde_json::to_string_pretty(&json!({"metadata":metadata,"trials":results}))?,
                )?;
            }
        }
    }
    drop(http);
    drop(server);
    store.pool.close().await;
    sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
        .execute(&control)
        .await?;
    control.close().await;
    println!(
        "Finished; temporary server stopped and isolated schema removed. Results: {}",
        output.display()
    );
    Ok(())
}
