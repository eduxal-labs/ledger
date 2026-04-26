tonic::include_proto!("event_service");

use crate::types::{error::Result, token::Token};
pub use event_service_server::EventServiceServer;
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

pub trait EventService: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> EventServiceServer<Self>;

    fn create_event(
        &self,
        token: Token,
        request: CreateEventRequest,
    ) -> impl Future<Output = Result<CreateEventResponse>> + Send;

    fn get_event(
        &self,
        token: Token,
        request: GetEventRequest,
    ) -> impl Future<Output = Result<GetEventResponse>> + Send;

    fn list_events(
        &self,
        token: Token,
        request: ListEventsRequest,
    ) -> impl Future<Output = Result<ListEventsResponse>> + Send;

    fn update_event(
        &self,
        token: Token,
        request: UpdateEventRequest,
    ) -> impl Future<Output = Result<UpdateEventResponse>> + Send;

    fn delete_event(
        &self,
        token: Token,
        request: DeleteEventRequest,
    ) -> impl Future<Output = Result<DeleteEventResponse>> + Send;
}

#[tonic::async_trait]
impl<T: EventService> event_service_server::EventService for T {
    async fn create_event(
        &self,
        request: Request<CreateEventRequest>,
    ) -> std::result::Result<Response<CreateEventResponse>, Status> {
        let token = extract_token(&request)?;
        let response = EventService::create_event(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_event(
        &self,
        request: Request<GetEventRequest>,
    ) -> std::result::Result<Response<GetEventResponse>, Status> {
        let token = extract_token(&request)?;
        let response = EventService::get_event(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn list_events(
        &self,
        request: Request<ListEventsRequest>,
    ) -> std::result::Result<Response<ListEventsResponse>, Status> {
        let token = extract_token(&request)?;
        let response = EventService::list_events(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn update_event(
        &self,
        request: Request<UpdateEventRequest>,
    ) -> std::result::Result<Response<UpdateEventResponse>, Status> {
        let token = extract_token(&request)?;
        let response = EventService::update_event(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn delete_event(
        &self,
        request: Request<DeleteEventRequest>,
    ) -> std::result::Result<Response<DeleteEventResponse>, Status> {
        let token = extract_token(&request)?;
        let response = EventService::delete_event(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }
}
