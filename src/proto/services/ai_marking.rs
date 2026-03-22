tonic::include_proto!("ai_marking");

use crate::types::{error::Result, token::Token};
pub use ai_marking_server::AiMarkingServer;
use std::future::Future;
use tonic::{Request, Response, Status};

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
        let token: Token = request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("missing authorization"))?
            .to_str()
            .map_err(|_| Status::unauthenticated("invalid authorization"))?
            .strip_prefix("Bearer ")
            .ok_or_else(|| Status::unauthenticated("missing Bearer prefix"))?
            .parse()?;
        let inner = request.into_inner();
        let response = AiMarking::request_upload_urls(self, token, inner).await?;
        Ok(Response::new(response))
    }

    async fn mark_paper(
        &self,
        request: Request<MarkPaperRequest>,
    ) -> std::result::Result<Response<MarkPaperResponse>, Status> {
        eprintln!("[AI-GRPC] mark_paper: request received");
        let token: Token = match request
            .metadata()
            .get("authorization")
            .ok_or_else(|| Status::unauthenticated("missing authorization"))
            .and_then(|v| v.to_str().map_err(|_| Status::unauthenticated("invalid authorization")))
            .and_then(|s| s.strip_prefix("Bearer ").ok_or_else(|| Status::unauthenticated("missing Bearer prefix")))
            .and_then(|s| s.parse::<Token>().map_err(Status::from))
        {
            Ok(t) => {
                eprintln!("[AI-GRPC] mark_paper: token OK (user={})", t.user);
                t
            }
            Err(e) => {
                eprintln!("[AI-GRPC] mark_paper: token ERROR — {}", e);
                return Err(e);
            }
        };
        let inner = request.into_inner();
        eprintln!(
            "[AI-GRPC] mark_paper: calling service (school={} exam={} students={})",
            inner.school, inner.exam, inner.students.len()
        );
        let t0 = std::time::Instant::now();
        let response = AiMarking::mark_paper(self, token, inner).await;
        eprintln!("[AI-GRPC] mark_paper: service returned in {}ms", t0.elapsed().as_millis());
        let response = response?;
        eprintln!("[AI-GRPC] mark_paper: sending response (accepted={} message={})", response.accepted, response.message);
        Ok(Response::new(response))
    }
}
