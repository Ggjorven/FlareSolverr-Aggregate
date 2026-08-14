use std::sync::Arc;

use serde_json::json;
use thiserror::Error;
use axum::{routing, response};
use axum::http::StatusCode;

use super::api;
use crate::http::api::AppState;
use crate::env;
use crate::solvers::SolversState;

/////////////////////////////////////////////////////
// RouteError
/////////////////////////////////////////////////////
#[derive(Debug, Error)]
pub enum RouteError {
    #[error("Failed to bind to port {port} with error: {error}.")]
    FailedToBind { port: u16, error: std::io::Error },
}

/////////////////////////////////////////////////////
// Router
/////////////////////////////////////////////////////
pub struct Router {
    router: axum::Router,
    listener: tokio::net::TcpListener,
}

impl Router {
    pub async fn new(environment: env::EnvOptions, state: SolversState) -> Result<Self, RouteError> {
        let address = "0.0.0.0:".to_string() + environment.port.to_string().as_str();
        let listener = tokio::net::TcpListener::bind(address.as_str())
            .await
            .map_err(|error| RouteError::FailedToBind {
                port: environment.port,
                error: error,
            })?;

        info!("HTTP server listening on {}.", address.as_str());

        let router = axum::Router::new()
            // health
            .route("/health", routing::get(Self::health))

            // v1
            .route("/v1", routing::post(api::v1))

            .with_state(Arc::new(AppState::new(state, environment)));

        trace!("Created HTTP router.");

        Ok(Self {
            router: router,
            listener: listener,
        })
    }

    pub async fn serve(self) {
        // NOTE: Never returns an error
        axum::serve(self.listener, self.router)
            .with_graceful_shutdown(Self::shutdown_signal())
            .await
            .unwrap();
    }

    async fn shutdown_signal() {
        let ctrl_c = async {
            tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install terminate signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
    }

    async fn health() -> (StatusCode, response::Json<serde_json::Value>) {
        trace!("Got /health request.");

        let response = {
            // TODO: Future internet checks or something
            response::Json(json!({ "health": "healthy" }))
        };

        trace!("Responding with: {}", response.to_string());
        (StatusCode::OK, response)
    }
}
