use std::{
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::State,
    http::{HeaderName, HeaderValue, StatusCode},
    response::Json,
    routing::{get, post},
};
use chrono::Local;
use colored::Colorize;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::task::JoinSet;
use tower_http::set_header::SetResponseHeaderLayer;

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct SolverConfig {
    name: String,
    url: String,
    enabled: bool,
}

struct AppConfig {
    host: String,
    port: u16,
    race_timeout_ms: u64,
    solvers: Vec<SolverConfig>,
}

impl AppConfig {
    fn from_env() -> Self {
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8191_u16);
        let race_timeout_ms = env::var("RACE_TIMEOUT_MS").ok().and_then(|t| t.parse().ok()).unwrap_or(60_000_u64);

        // Dynamic solver list via SOLVER_n_NAME / SOLVER_n_URL / SOLVER_n_ENABLED.
        // Gaps in numbering are skipped; scanning stops at n=10.
        let mut solvers: Vec<SolverConfig> = (0..10)
            .filter_map(|i| {
                let name = env::var(format!("SOLVER_{i}_NAME")).ok()?;
                let url = env::var(format!("SOLVER_{i}_URL")).ok()?;
                let enabled = env::var(format!("SOLVER_{i}_ENABLED"))
                    .map(|v| !matches!(v.to_lowercase().as_str(), "false" | "0" | "no"))
                    .unwrap_or(true);
                Some(SolverConfig { name, url, enabled })
            })
            .collect();

        // Fallback: individual well-known env vars if no numbered solvers set.
        if solvers.is_empty() {
            if let Ok(flaresolverr_url) = env::var("FLARESOLVERR_URL") {
                solvers.push(SolverConfig {
                    name: "flaresolverr".to_string(),
                    url: flaresolverr_url,
                    enabled: true,
                });
            }

            if let Ok(byparr_url) = env::var("BYPARR_URL") {
                solvers.push(SolverConfig {
                    name: "byparr".to_string(),
                    url: byparr_url,
                    enabled: true,
                });
            }

            if let Ok(cfc_url) = env::var("CF_CLEARANCE_URL") {
                solvers.push(SolverConfig {
                    name: "cf-clearance".to_string(),
                    url: cfc_url,
                    enabled: true,
                });
            }
        }

        Self {
            host,
            port,
            race_timeout_ms,
            solvers,
        }
    }
}

// ─── Shared state ─────────────────────────────────────────────────────────────

struct AppState {
    client: Client,
    solvers: Vec<SolverConfig>,
    race_timeout_ms: u64,
}

// ─── Logging ──────────────────────────────────────────────────────────────────

fn ts() -> String {
    Local::now().format("%H:%M:%S%.3f").to_string()
}

macro_rules! log_info {
    ($tag:expr, $($arg:tt)*) => {
        println!("{} {} {} {}",
            format!("[{}]", ts()).dimmed(),
            "INFO".cyan().bold(),
            format!("[{}]", $tag).yellow(),
            format!($($arg)*),
        )
    };
}

macro_rules! log_ok {
    ($tag:expr, $($arg:tt)*) => {
        println!("{} {} {} {}",
            format!("[{}]", ts()).dimmed(),
            " OK ".green().bold(),
            format!("[{}]", $tag).yellow(),
            format!($($arg)*),
        )
    };
}

macro_rules! log_warn {
    ($tag:expr, $($arg:tt)*) => {
        println!("{} {} {} {}",
            format!("[{}]", ts()).dimmed(),
            "WARN".yellow().bold(),
            format!("[{}]", $tag).yellow(),
            format!($($arg)*),
        )
    };
}

macro_rules! log_err {
    ($tag:expr, $($arg:tt)*) => {
        eprintln!("{} {} {} {}",
            format!("[{}]", ts()).dimmed(),
            " ERR".red().bold(),
            format!("[{}]", $tag).yellow(),
            format!($($arg)*),
        )
    };
}

// ─── Solver logic ─────────────────────────────────────────────────────────────

/// Attempt one solver. Returns `Some(response)` on `"status": "ok"`, else `None`.
async fn try_solver(client: Client, solver: SolverConfig, mut payload: Value, timeout: Duration) -> Option<Value> {
    // Cap maxTimeout so the solver self-terminates before our race timeout.
    let solver_ms = timeout.as_millis().saturating_sub(2_000) as u64;
    if let Some(obj) = payload.as_object_mut() {
        let current = obj.get("maxTimeout").and_then(Value::as_u64).unwrap_or(u64::MAX);
        if current > solver_ms {
            obj.insert("maxTimeout".into(), json!(solver_ms));
        }
    }

    let target = payload.get("url").and_then(Value::as_str).unwrap_or("?").to_string();

    log_info!(solver.name, "→ {target}");

    let send = client.post(&solver.url).json(&payload).timeout(timeout).send().await;

    let resp = match send {
        Ok(r) => r,
        Err(e) => {
            log_warn!(solver.name, "Request failed: {e}");
            return None;
        },
    };

    let data: Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            log_warn!(solver.name, "Bad response body: {e}");
            return None;
        },
    };

    let status = data.get("status").and_then(Value::as_str).unwrap_or("");
    if status == "ok" {
        log_ok!(solver.name, "✓ succeeded");
        Some(data)
    } else {
        let msg = data.get("message").and_then(Value::as_str).unwrap_or("unknown");
        log_warn!(solver.name, "status={status:?}: {msg}");
        None
    }
}

/// Fan out to all enabled solvers and return the first successful result.
async fn race_solvers(state: &AppState, payload: Value) -> Option<Value> {
    let enabled: Vec<SolverConfig> = state.solvers.iter().filter(|s| s.enabled).cloned().collect();

    if enabled.is_empty() {
        log_err!("proxy", "No enabled solvers");
        return None;
    }

    let target = payload.get("url").and_then(Value::as_str).unwrap_or("?");

    log_info!("proxy", "Racing {} solver(s) → {target}", enabled.len());

    let timeout = Duration::from_millis(state.race_timeout_ms);
    let mut set: JoinSet<Option<Value>> = JoinSet::new();

    for solver in enabled {
        let client = state.client.clone();
        let payload = payload.clone();
        set.spawn(try_solver(client, solver, payload, timeout));
    }

    while let Some(res) = set.join_next().await {
        match res {
            Ok(Some(value)) => {
                // Winner found — cancel remaining tasks and return.
                set.abort_all();
                return Some(value);
            },
            Ok(None) => {},                   // This solver failed; wait for others.
            Err(e) if e.is_cancelled() => {}, // Expected after abort_all().
            Err(e) => log_warn!("proxy", "Task error: {e}"),
        }
    }

    None // All solvers exhausted without success.
}

/// Forward a single request to the first enabled solver (used for session commands).
async fn forward_to_first(state: &AppState, payload: Value) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let solver = state
        .solvers
        .iter()
        .find(|s| s.enabled)
        .ok_or_else(|| err_response(StatusCode::SERVICE_UNAVAILABLE, "No enabled solvers"))?;

    let cmd = payload.get("cmd").and_then(Value::as_str).unwrap_or("?");

    log_info!(solver.name, "→ (session) {cmd}");

    let resp = state
        .client
        .post(&solver.url)
        .json(&payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| err_response(StatusCode::BAD_GATEWAY, &e.to_string()))?;

    let data = resp
        .json::<Value>()
        .await
        .map_err(|e| err_response(StatusCode::BAD_GATEWAY, &e.to_string()))?;

    Ok(Json(data))
}

// ─── Axum handlers ────────────────────────────────────────────────────────────

async fn handle_v1(State(state): State<Arc<AppState>>, Json(payload): Json<Value>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let cmd = payload.get("cmd").and_then(Value::as_str).unwrap_or("");

    // Session commands are solver-specific; forward to first available only.
    if cmd.starts_with("sessions.") {
        return forward_to_first(&state, payload).await;
    }

    let start = Instant::now();

    match race_solvers(&state, payload).await {
        Some(result) => {
            log_ok!("proxy", "Solved in {:.2}s", start.elapsed().as_secs_f64());
            Ok(Json(result))
        },
        None => {
            let msg = format!("All solvers failed or timed out after {:.2}s", start.elapsed().as_secs_f64(),);
            log_err!("proxy", "{msg}");
            Err(err_response(StatusCode::BAD_GATEWAY, &msg))
        },
    }
}

async fn handle_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "solvers": state.solvers.iter().map(|s| json!({
            "name":    s.name,
            "url":     s.url,
            "enabled": s.enabled,
        })).collect::<Vec<_>>(),
    }))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn err_response(code: StatusCode, msg: &str) -> (StatusCode, Json<Value>) {
    (code, Json(json!({ "status": "error", "message": msg })))
}

// ─── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();

    println!("{}", format!("\n  FlareSolverr Aggregate  v{}\n", env!("CARGO_PKG_VERSION")).cyan().bold(),);

    if config.solvers.is_empty() {
        log_err!("config", "No solvers configured — set SOLVER_0_NAME / SOLVER_0_URL or FLARESOLVERR_URL");
        std::process::exit(1);
    }

    for s in &config.solvers {
        let status = if s.enabled {
            "enabled".green().to_string()
        } else {
            "disabled".dimmed().to_string()
        };
        log_info!("config", "{} → {} [{}]", s.name, s.url, status);
    }
    log_info!("config", "Race timeout: {}ms", config.race_timeout_ms);

    let client = Client::builder()
        .pool_idle_timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client");

    let state = Arc::new(AppState {
        client,
        solvers: config.solvers,
        race_timeout_ms: config.race_timeout_ms,
    });

    let app = Router::new()
        .route("/v1", post(handle_v1))
        .route("/health", get(handle_health))
        .with_state(state)
        .layer(SetResponseHeaderLayer::overriding(HeaderName::from_static("x-solver-proxy"), HeaderValue::from_static("flaresolverr-aggregate")));

    let addr = format!("{}:{}", config.host, config.port);
    log_ok!("server", "Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind {addr}: {e}"));

    axum::serve(listener, app).await.expect("Server crashed");
}
