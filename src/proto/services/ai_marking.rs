tonic::include_proto!("ai_marking");

use crate::types::{error::Result, token::Token};
pub use ai_marking_server::AiMarkingServer;
use std::future::Future;
use tonic::{Request, Response, Status};

fn extract_token<T>(request: &Request<T>) -> std::result::Result<Token, Status> {
    request
        .metadata()
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("missing authorization"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("invalid authorization"))?
        .strip_prefix("Bearer ")
        .ok_or_else(|| Status::unauthenticated("missing Bearer prefix"))?
        .parse()
        .map_err(Status::from)
}

pub trait AiMarking: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> AiMarkingServer<Self>;

    fn request_upload_urls(
        &self,
        token: Token,
        request: UploadUrlsRequest,
    ) -> impl Future<Output = Result<UploadUrlsResponse>> + Send;

    fn mark_paper(
        &self,
        token: Token,
        request: MarkPaperRequest,
    ) -> impl Future<Output = Result<MarkPaperResponse>> + Send;
}

#[tonic::async_trait]
impl<T: AiMarking> ai_marking_server::AiMarking for T {
    async fn request_upload_urls(
        &self,
        request: Request<UploadUrlsRequest>,
    ) -> std::result::Result<Response<UploadUrlsResponse>, Status> {
        let token = extract_token(&request)?;
        let response = AiMarking::request_upload_urls(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn mark_paper(
        &self,
        request: Request<MarkPaperRequest>,
    ) -> std::result::Result<Response<MarkPaperResponse>, Status> {
        let token = extract_token(&request)?;
        let response = AiMarking::mark_paper(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }
}
