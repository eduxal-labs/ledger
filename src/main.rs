mod ai;
mod config;
mod db;
mod proto;
mod server;
mod services;
mod types;

#[tokio::main]
async fn main() -> server::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    server::start().await
}
