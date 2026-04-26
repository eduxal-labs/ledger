tonic::include_proto!("paper_management");

use crate::types::{error::Result, token::Token};
pub use paper_management_server::PaperManagementServer;
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

pub trait PaperManagement: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> PaperManagementServer<Self>;

    // ── Schedules ──

    fn schedule_paper(
        &self,
        token: Token,
        request: SchedulePaperRequest,
    ) -> impl Future<Output = Result<SchedulePaperResponse>> + Send;

    fn assign_invigilator(
        &self,
        token: Token,
        request: AssignInvigilatorRequest,
    ) -> impl Future<Output = Result<AssignInvigilatorResponse>> + Send;

    fn list_paper_schedules(
        &self,
        token: Token,
        request: ListSchedulesRequest,
    ) -> impl Future<Output = Result<ListSchedulesResponse>> + Send;

    fn update_schedule(
        &self,
        token: Token,
        request: UpdateScheduleRequest,
    ) -> impl Future<Output = Result<UpdateScheduleResponse>> + Send;

    // ── Taught Topics ──

    fn set_taught_topics(
        &self,
        token: Token,
        request: SetTaughtTopicsRequest,
    ) -> impl Future<Output = Result<SetTaughtTopicsResponse>> + Send;

    fn get_taught_topics(
        &self,
        token: Token,
        request: GetTaughtTopicsRequest,
    ) -> impl Future<Output = Result<GetTaughtTopicsResponse>> + Send;

    // ── Coverage ──

    fn confirm_exam_coverage(
        &self,
        token: Token,
        request: ConfirmExamCoverageRequest,
    ) -> impl Future<Output = Result<ConfirmExamCoverageResponse>> + Send;

    fn get_exam_coverage(
        &self,
        token: Token,
        request: GetExamCoverageRequest,
    ) -> impl Future<Output = Result<GetExamCoverageResponse>> + Send;

    // ── Generation ──

    fn generate_assessment(
        &self,
        token: Token,
        request: GenerateAssessmentRequest,
    ) -> impl Future<Output = Result<GenerateAssessmentResponse>> + Send;

    fn generate_assignment(
        &self,
        token: Token,
        request: GenerateAssignmentRequest,
    ) -> impl Future<Output = Result<GenerateAssignmentResponse>> + Send;

    // ── Per-student PDFs ──

    fn finalize_student_papers(
        &self,
        token: Token,
        request: FinalizeStudentPapersRequest,
    ) -> impl Future<Output = Result<FinalizeStudentPapersResponse>> + Send;

    fn get_student_papers_status(
        &self,
        token: Token,
        request: GetStudentPapersStatusRequest,
    ) -> impl Future<Output = Result<GetStudentPapersStatusResponse>> + Send;

    fn get_student_paper_pdf(
        &self,
        token: Token,
        request: GetStudentPaperPdfRequest,
    ) -> impl Future<Output = Result<GetStudentPaperPdfResponse>> + Send;
}

#[tonic::async_trait]
impl<T: PaperManagement> paper_management_server::PaperManagement for T {
    async fn schedule_paper(
        &self,
        request: Request<SchedulePaperRequest>,
    ) -> std::result::Result<Response<SchedulePaperResponse>, Status> {
        let token = extract_token(&request)?;
        let response = PaperManagement::schedule_paper(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn assign_invigilator(
        &self,
        request: Request<AssignInvigilatorRequest>,
    ) -> std::result::Result<Response<AssignInvigilatorResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::assign_invigilator(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn list_paper_schedules(
        &self,
        request: Request<ListSchedulesRequest>,
    ) -> std::result::Result<Response<ListSchedulesResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::list_paper_schedules(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn update_schedule(
        &self,
        request: Request<UpdateScheduleRequest>,
    ) -> std::result::Result<Response<UpdateScheduleResponse>, Status> {
        let token = extract_token(&request)?;
        let response = PaperManagement::update_schedule(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn set_taught_topics(
        &self,
        request: Request<SetTaughtTopicsRequest>,
    ) -> std::result::Result<Response<SetTaughtTopicsResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::set_taught_topics(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_taught_topics(
        &self,
        request: Request<GetTaughtTopicsRequest>,
    ) -> std::result::Result<Response<GetTaughtTopicsResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::get_taught_topics(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn confirm_exam_coverage(
        &self,
        request: Request<ConfirmExamCoverageRequest>,
    ) -> std::result::Result<Response<ConfirmExamCoverageResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::confirm_exam_coverage(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_exam_coverage(
        &self,
        request: Request<GetExamCoverageRequest>,
    ) -> std::result::Result<Response<GetExamCoverageResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::get_exam_coverage(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn generate_assessment(
        &self,
        request: Request<GenerateAssessmentRequest>,
    ) -> std::result::Result<Response<GenerateAssessmentResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::generate_assessment(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn generate_assignment(
        &self,
        request: Request<GenerateAssignmentRequest>,
    ) -> std::result::Result<Response<GenerateAssignmentResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::generate_assignment(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn finalize_student_papers(
        &self,
        request: Request<FinalizeStudentPapersRequest>,
    ) -> std::result::Result<Response<FinalizeStudentPapersResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::finalize_student_papers(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_student_papers_status(
        &self,
        request: Request<GetStudentPapersStatusRequest>,
    ) -> std::result::Result<Response<GetStudentPapersStatusResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::get_student_papers_status(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_student_paper_pdf(
        &self,
        request: Request<GetStudentPaperPdfRequest>,
    ) -> std::result::Result<Response<GetStudentPaperPdfResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            PaperManagement::get_student_paper_pdf(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }
}
