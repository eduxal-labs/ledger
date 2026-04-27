tonic::include_proto!("paper_service");

use crate::types::{error::Result, token::Token};
pub use paper_service_server::PaperServiceServer;
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

pub trait PaperService: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> PaperServiceServer<Self>;

    fn create_paper(
        &self,
        token: Token,
        request: CreatePaperRequest,
    ) -> impl Future<Output = Result<CreatePaperResponse>> + Send;

    fn get_paper(
        &self,
        token: Token,
        request: GetPaperRequest,
    ) -> impl Future<Output = Result<GetPaperResponse>> + Send;

    fn list_papers(
        &self,
        token: Token,
        request: ListPapersRequest,
    ) -> impl Future<Output = Result<ListPapersResponse>> + Send;

    fn update_paper(
        &self,
        token: Token,
        request: UpdatePaperRequest,
    ) -> impl Future<Output = Result<UpdatePaperResponse>> + Send;

    fn get_paper_pdf_url(
        &self,
        token: Token,
        request: GetPaperPdfUrlRequest,
    ) -> impl Future<Output = Result<GetPaperPdfUrlResponse>> + Send;

    fn get_marking_scheme_url(
        &self,
        token: Token,
        request: GetMarkingSchemeUrlRequest,
    ) -> impl Future<Output = Result<GetMarkingSchemeUrlResponse>> + Send;

    fn force_set_paper_status(
        &self,
        token: Token,
        request: ForceSetPaperStatusRequest,
    ) -> impl Future<Output = Result<ForceSetPaperStatusResponse>> + Send;
}

#[tonic::async_trait]
impl<T: PaperService> paper_service_server::PaperService for T {
    async fn create_paper(
        &self,
        request: Request<CreatePaperRequest>,
    ) -> std::result::Result<Response<CreatePaperResponse>, Status> {
        let token = extract_token(&request)?;
        let response = PaperService::create_paper(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_paper(
        &self,
        request: Request<GetPaperRequest>,
    ) -> std::result::Result<Response<GetPaperResponse>, Status> {
        let token = extract_token(&request)?;
        let response = PaperService::get_paper(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn list_papers(
        &self,
        request: Request<ListPapersRequest>,
    ) -> std::result::Result<Response<ListPapersResponse>, Status> {
        let token = extract_token(&request)?;
        let response = PaperService::list_papers(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn update_paper(
        &self,
        request: Request<UpdatePaperRequest>,
    ) -> std::result::Result<Response<UpdatePaperResponse>, Status> {
        let token = extract_token(&request)?;
        let response = PaperService::update_paper(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_paper_pdf_url(
        &self,
        request: Request<GetPaperPdfUrlRequest>,
    ) -> std::result::Result<Response<GetPaperPdfUrlResponse>, Status> {
        let token = extract_token(&request)?;
        let response = PaperService::get_paper_pdf_url(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_marking_scheme_url(
        &self,
        request: Request<GetMarkingSchemeUrlRequest>,
    ) -> std::result::Result<Response<GetMarkingSchemeUrlResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperService::get_marking_scheme_url(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn force_set_paper_status(
        &self,
        request: Request<ForceSetPaperStatusRequest>,
    ) -> std::result::Result<Response<ForceSetPaperStatusResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperService::force_set_paper_status(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }
}
