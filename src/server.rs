use crate::config::Configuration;
use crate::proto::services::authentication::Authentication;
use crate::services::authentication::Authenticator;
use std::sync::Arc;
use tonic::transport::Server;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static>>;

pub async fn start() -> Result<()> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 50051));
    let config = Arc::new(Configuration::default());
    let authenticator = Authenticator::new(config.clone());
    Server::builder()
        .add_service(authenticator)
        .serve(addr)
        .await?;
    Ok(())
}
