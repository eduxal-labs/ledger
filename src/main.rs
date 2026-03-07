mod config;
mod db;
mod proto;
mod server;
mod services;
mod types;

#[tokio::main]
async fn main() -> server::Result<()> {
    server::start().await
}
