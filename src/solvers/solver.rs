use serde_json::{Value};
use reqwest::Client;

/////////////////////////////////////////////////////
// Solver
/////////////////////////////////////////////////////
#[async_trait::async_trait]
pub trait Solver: Send + Sync {
    async fn request(&self, client: Client, payload: Value, timeout: u64) -> Result<Value, ()>;
    async fn session(&self, client: Client, payload: Value) -> Result<Value, ()>;
}
