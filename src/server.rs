use crate::config::Configuration;
use crate::proto::services::ai_marking::AiMarking;
use crate::proto::services::authentication::Authentication;
use crate::proto::services::sync::Sync;
use crate::services::ai_marking::AiMarkingService;
use crate::services::authentication::Authenticator;
use crate::services::sync::SyncService;
use std::sync::Arc;
use tonic::transport::Server;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static>>;

pub async fn start() -> Result<()> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 50051));
    let config = Arc::new(Configuration::default());
    let authenticator = Authenticator::new(config.clone());
    let sync = SyncService::new(config.clone());
    let ai_marking = AiMarkingService::new(config.clone());
    Server::builder()
        .add_service(authenticator)
        .add_service(sync)
        .add_service(ai_marking)
        .serve(addr)
        .await?;
    Ok(())
}
