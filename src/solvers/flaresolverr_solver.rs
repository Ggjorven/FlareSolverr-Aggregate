use std::time::Duration;

use reqwest::Client;
use serde_json::{Value};

use crate::solvers::Solver;

/////////////////////////////////////////////////////
// Solver
/////////////////////////////////////////////////////
pub struct FlareSolverrSolver {
    url: String,
}

impl FlareSolverrSolver {
    pub fn new(url: String) -> Self {
        Self { url: url }
    }
}

#[async_trait::async_trait]
impl Solver for FlareSolverrSolver {
    async fn request(&self, client: Client, payload: Value, timeout: u64) -> Result<Value, ()> {
        let target = payload.get("url").and_then(Value::as_str).unwrap_or("?").to_string();
        info!("[flaresolverr] Trying: {}...", target);

        let send = client.post(&self.url).json(&payload).timeout(Duration::from_millis(timeout)).send().await;

        let response = match send {
            Ok(response) => response,
            Err(error) => {
                warning!("[flaresolverr] Request failed: {}", error);
                return Err(());
            },
        };

        let data: Value = match response.json().await {
            Ok(data) => data,
            Err(error) => {
                warning!("[flaresolverr] Bad response body: {}", error);
                return Err(());
            },
        };

        let status = data.get("status").and_then(Value::as_str).unwrap_or("");
        if status == "ok" {
            info!("[flaresolverr] succeeded");
            Ok(data)
        } else {
            let msg = data.get("message").and_then(Value::as_str).unwrap_or("unknown");
            warning!("[flaresolverr] status={status:?}: {msg}");
            Err(())
        }
    }

    async fn session(&self, client: Client, payload: Value) -> Result<Value, ()> {
        let cmd = payload.get("cmd").and_then(Value::as_str).unwrap_or("?");
        info!("[flaresolverr] -> (session) {}", cmd);

        let resp = client
            .post(&self.url)
            .json(&payload)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|_error| ())?;

        let data = resp.json::<Value>().await.map_err(|_error| ())?;

        Ok(data)
    }
}
