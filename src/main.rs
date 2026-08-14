#[macro_use]
mod logging;
mod env;
mod http;
mod solvers;

#[tokio::main]
async fn main() -> Result<(), ()> {
    // Setup
    logging::add_sink(Box::new(logging::ConsoleSink::new(None)));

    let env = env::EnvOptions::from_env().map_err(|_error| ())?;

    logging::clear_sinks();
    logging::add_sink(Box::new(logging::ConsoleSink::new(Some(env.log_level.clone()))));

    trace!("Env options: {:?}", env);

    // State
    let state = solvers::SolversState::new(&env);

    // HTTP Router
    let router = http::Router::new(env, state).await.map_err(|_error| ())?;
    router.serve().await;

    Ok(())
}
