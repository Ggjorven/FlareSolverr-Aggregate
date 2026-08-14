use std::{
    sync::{Arc},
};

use axum::{
    extract::{State, Json},
    http::{StatusCode},
};
use serde_json::{Value};
use tokio::time::Instant;

use crate::{env, solvers};

/////////////////////////////////////////////////////
// State
/////////////////////////////////////////////////////
pub struct AppState {
    pub environment: env::EnvOptions,   // readonly
    pub solvers: solvers::SolversState, // readonly
}

impl AppState {
    pub fn new(state: solvers::SolversState, environment: env::EnvOptions) -> Self {
        Self {
            solvers: state,
            environment: environment,
        }
    }
}

/////////////////////////////////////////////////////
// API
/////////////////////////////////////////////////////
pub async fn v1(State(state): State<Arc<AppState>>, Json(payload): Json<Value>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    trace!("Received /v1 with payload {:?}", payload);

    let cmd = payload.get("cmd").and_then(Value::as_str).unwrap_or("");
    let max_timeout = payload.get("maxTimeout").and_then(Value::as_u64).unwrap_or(60000);

    // Session commands are solver-specific, forward to first available only.
    if cmd.starts_with("sessions.") {
        match state.solvers.session_cmd(payload, max_timeout).await {
            Ok(result) => Ok(Json(result)),
            Err(_error) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("{{ status: \"error\", message: \"Failed to handle session command, see logs.\" }}".into()),
            )),
        }
    } else if cmd.starts_with("request.") {
        let start = Instant::now();

        match state.solvers.request_cmd(payload, max_timeout).await {
            Ok(result) => {
                info!("[proxy] Solved in {:.2}s", start.elapsed().as_secs_f64());
                Ok(Json(result))
            },
            Err(_error) => {
                let msg =
                    format!("{{ status: \"error\", message: \"All solvers failed or timed out after {:.2}s\" }}", start.elapsed().as_secs_f64());
                error!("[proxy] {}", msg);
                Err((StatusCode::BAD_GATEWAY, Json(msg.into())))
            },
        }
    } else {
        Err((StatusCode::NOT_FOUND, Json(Value::from("Invalid cmd, please report to the developer."))))
    }
}
