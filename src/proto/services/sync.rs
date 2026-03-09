tonic::include_proto!("sync");
use crate::types::{error::Result, token::Token};
use std::future::Future;
pub use sync_server::SyncServer;
use tokio::sync::mpsc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};

pub trait Sync: Send + ::std::marker::Sync + 'static + Sized {
    type Config: Send + ::std::marker::Sync + 'static;
    type WatchStream: Stream<Item = Result<SyncDelta>> + Send + 'static;

    fn new(config: Self::Config) -> SyncServer<Self>;

    fn push_changes(
        &self,
        token: Token,
        stream: Streaming<MutationBatch>,
    ) -> impl Future<Output = Result<mpsc::Receiver<PushAck>>> + Send;

    fn watch_changes(
        &self,
        token: Token,
        request: WatchRequest,
    ) -> impl Future<Output = Result<Self::WatchStream>> + Send;
}

#[tonic::async_trait]
impl<T: Sync> sync_server::Sync for T {
    type PushChangesStream =
        tokio_stream::wrappers::ReceiverStream<std::result::Result<PushAck, Status>>;
    type WatchChangesStream =
        std::pin::Pin<Box<dyn Stream<Item = std::result::Result<SyncDelta, Status>> + Send>>;

    async fn push_changes(
        &self,
        request: Request<Streaming<MutationBatch>>,
    ) -> std::result::Result<Response<Self::PushChangesStream>, Status> {
        let token: Token = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("missing authorization"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid authorization"))?
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("missing Bearer prefix"))?
            .parse()?;

        let stream = request.into_inner();
        let rx = Sync::push_changes(self, token, stream).await?;

        let (tx_out, rx_out) = mpsc::channel(64);
        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(ack) = rx.recv().await {
                if tx_out.send(Ok(ack)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx_out,
        )))
    }

    async fn watch_changes(
        &self,
        request: Request<WatchRequest>,
    ) -> std::result::Result<Response<Self::WatchChangesStream>, Status> {
        let token: Token = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("missing authorization"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid authorization"))?
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("missing Bearer prefix"))?
            .parse()?;

        let watch_request = request.into_inner();
        let stream = Sync::watch_changes(self, token, watch_request).await?;

        let mapped = Box::pin(tokio_stream::StreamExt::map(stream, |result| {
            result.map_err(|e| Status::from(e))
        }));

        Ok(Response::new(mapped))
    }
}
