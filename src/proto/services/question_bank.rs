tonic::include_proto!("question_bank");

use crate::types::{error::Result, token::Token};
pub use question_bank_server::QuestionBankServer;
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

pub trait QuestionBank: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> QuestionBankServer<Self>;

    // === System User Operations (question management) ===

    fn create_question(
        &self,
        token: Token,
        request: CreateQuestionRequest,
    ) -> impl Future<Output = Result<CreateQuestionResponse>> + Send;

    fn update_question(
        &self,
        token: Token,
        request: UpdateQuestionRequest,
    ) -> impl Future<Output = Result<UpdateQuestionResponse>> + Send;

    fn delete_question(
        &self,
        token: Token,
        request: DeleteQuestionRequest,
    ) -> impl Future<Output = Result<DeleteQuestionResponse>> + Send;

    fn bulk_import_questions(
        &self,
        token: Token,
        request: BulkImportRequest,
    ) -> impl Future<Output = Result<BulkImportResponse>> + Send;

    fn request_image_upload_urls(
        &self,
        token: Token,
        request: ImageUploadUrlsRequest,
    ) -> impl Future<Output = Result<ImageUploadUrlsResponse>> + Send;

    // === Teacher Operations (exam paper assembly) ===

    fn generate_paper(
        &self,
        token: Token,
        request: GeneratePaperRequest,
    ) -> impl Future<Output = Result<GeneratePaperResponse>> + Send;

    fn regenerate_question(
        &self,
        token: Token,
        request: RegenerateQuestionRequest,
    ) -> impl Future<Output = Result<RegenerateQuestionResponse>> + Send;

    fn edit_paper_question(
        &self,
        token: Token,
        request: EditPaperQuestionRequest,
    ) -> impl Future<Output = Result<EditPaperQuestionResponse>> + Send;

    fn finalize_paper(
        &self,
        token: Token,
        request: FinalizePaperRequest,
    ) -> impl Future<Output = Result<FinalizePaperResponse>> + Send;

    fn get_paper_pdf(
        &self,
        token: Token,
        request: GetPaperPdfRequest,
    ) -> impl Future<Output = Result<GetPaperPdfResponse>> + Send;

    fn get_paper_questions(
        &self,
        token: Token,
        request: GetPaperQuestionsRequest,
    ) -> impl Future<Output = Result<GetPaperQuestionsResponse>> + Send;

    fn set_paper_question_section(
        &self,
        token: Token,
        request: SetPaperQuestionSectionRequest,
    ) -> impl Future<Output = Result<SetPaperQuestionSectionResponse>> + Send;

    // === Read Operations ===

    fn list_questions(
        &self,
        token: Token,
        request: ListQuestionsRequest,
    ) -> impl Future<Output = Result<ListQuestionsResponse>> + Send;

    fn get_question(
        &self,
        token: Token,
        request: GetQuestionRequest,
    ) -> impl Future<Output = Result<GetQuestionResponse>> + Send;

    fn get_question_grades(
        &self,
        token: Token,
        request: GetQuestionGradesRequest,
    ) -> impl Future<Output = Result<GetQuestionGradesResponse>> + Send;

    fn get_marking_status(
        &self,
        token: Token,
        request: MarkingStatusRequest,
    ) -> impl Future<Output = Result<MarkingStatusResponse>> + Send;
}

#[tonic::async_trait]
impl<T: QuestionBank> question_bank_server::QuestionBank for T {
    async fn create_question(
        &self,
        request: Request<CreateQuestionRequest>,
    ) -> std::result::Result<Response<CreateQuestionResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::create_question(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn update_question(
        &self,
        request: Request<UpdateQuestionRequest>,
    ) -> std::result::Result<Response<UpdateQuestionResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::update_question(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn delete_question(
        &self,
        request: Request<DeleteQuestionRequest>,
    ) -> std::result::Result<Response<DeleteQuestionResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::delete_question(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn bulk_import_questions(
        &self,
        request: Request<BulkImportRequest>,
    ) -> std::result::Result<Response<BulkImportResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            QuestionBank::bulk_import_questions(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn request_image_upload_urls(
        &self,
        request: Request<ImageUploadUrlsRequest>,
    ) -> std::result::Result<Response<ImageUploadUrlsResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            QuestionBank::request_image_upload_urls(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn generate_paper(
        &self,
        request: Request<GeneratePaperRequest>,
    ) -> std::result::Result<Response<GeneratePaperResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::generate_paper(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn regenerate_question(
        &self,
        request: Request<RegenerateQuestionRequest>,
    ) -> std::result::Result<Response<RegenerateQuestionResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::regenerate_question(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn edit_paper_question(
        &self,
        request: Request<EditPaperQuestionRequest>,
    ) -> std::result::Result<Response<EditPaperQuestionResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::edit_paper_question(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn finalize_paper(
        &self,
        request: Request<FinalizePaperRequest>,
    ) -> std::result::Result<Response<FinalizePaperResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::finalize_paper(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_paper_pdf(
        &self,
        request: Request<GetPaperPdfRequest>,
    ) -> std::result::Result<Response<GetPaperPdfResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::get_paper_pdf(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_paper_questions(
        &self,
        request: Request<GetPaperQuestionsRequest>,
    ) -> std::result::Result<Response<GetPaperQuestionsResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::get_paper_questions(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn set_paper_question_section(
        &self,
        request: Request<SetPaperQuestionSectionRequest>,
    ) -> std::result::Result<Response<SetPaperQuestionSectionResponse>, Status> {
        let token = extract_token(&request)?;
        let response =
            QuestionBank::set_paper_question_section(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn list_questions(
        &self,
        request: Request<ListQuestionsRequest>,
    ) -> std::result::Result<Response<ListQuestionsResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::list_questions(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_question(
        &self,
        request: Request<GetQuestionRequest>,
    ) -> std::result::Result<Response<GetQuestionResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::get_question(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_question_grades(
        &self,
        request: Request<GetQuestionGradesRequest>,
    ) -> std::result::Result<Response<GetQuestionGradesResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::get_question_grades(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }

    async fn get_marking_status(
        &self,
        request: Request<MarkingStatusRequest>,
    ) -> std::result::Result<Response<MarkingStatusResponse>, Status> {
        let token = extract_token(&request)?;
        let response = QuestionBank::get_marking_status(self, token, request.into_inner()).await?;
        Ok(Response::new(response))
    }
}
