use std::{sync::Arc, time::Duration};

use serde_json::{Value};
use reqwest::Client;
use tokio::task::JoinSet;

use super::Solver;
use crate::{env, solvers::FlareSolverrSolver};

/////////////////////////////////////////////////////
// State
/////////////////////////////////////////////////////
pub struct SolversState {
    pub client: Client,
    pub solvers: Vec<Arc<dyn Solver>>,
}

impl SolversState {
    pub fn new(environment: &env::EnvOptions) -> Self {
        let client = Client::builder()
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        let mut solvers: Vec<Arc<dyn Solver>> = Vec::new();
        if let Some(flaresolverr_url) = &environment.flaresolverr_url {
            solvers.push(Arc::new(FlareSolverrSolver::new("flaresolverr".to_string(), flaresolverr_url.to_string())));
        }
        if let Some(byparr_url) = &environment.byparr_url {
            solvers.push(Arc::new(FlareSolverrSolver::new("byparr".to_string(), byparr_url.to_string())));
        }

        Self {
            client: client,
            solvers: solvers,
        }
    }

    pub async fn request_cmd(&self, payload: Value, timeout: u64) -> Result<Value, ()> {
        if self.solvers.is_empty() {
            error!("[proxy] No enabled solvers.");
            return Err(());
        }

        let mut set: JoinSet<Result<Value, ()>> = JoinSet::new();

        for solver in &self.solvers {
            let solver = Arc::clone(solver);
            let client = self.client.clone();
            let payload = payload.clone();
            set.spawn(async move { solver.request(client, payload, timeout).await });
        }

        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(value)) => {
                    // Winner found, cancel remaining tasks and return.
                    set.abort_all();
                    return Ok(value);
                },
                Ok(Err(_)) => {},                                     // This solver failed; wait for others.
                Err(error) if error.is_cancelled() => return Err(()), // Expected after abort_all().
                Err(error) => {
                    warning!("[proxy] Task error: {}", error);
                    return Err(());
                },
            }
        }

        Err(())
    }

    pub async fn session_cmd(&self, payload: Value) -> Result<Value, ()> {
        if self.solvers.is_empty() {
            error!("[proxy] No enabled solvers.");
            return Err(());
        }

        self.solvers.first().unwrap().session(self.client.clone(), payload).await
    }
}
