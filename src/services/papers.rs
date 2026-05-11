use std::sync::Arc;

use crate::config::storage::sign;
use crate::db::database::CONN;
use crate::db::database::tables::papers as papers_db;
use crate::proto::services::paper_service::*;
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::paper::{Paper, PaperStatus, PaperUpdate};
use crate::types::token::Token;

pub struct PaperServiceImpl<C> {
    #[allow(dead_code)]
    config: Arc<C>,
}

fn paper_to_proto(p: &Paper) -> crate::proto::types::paper::Paper {
    crate::proto::types::paper::Paper {
        id: p.id.to_string(),
        school: p.school.clone(),
        event: p.event.clone(),
        subject: p.subject,
        grade: p.grade as i32,
        stream: p.stream.map(|s| s as i32),
        r#type: p.type_ as i32,
        teacher: p.teacher.clone(),
        name: p.name.clone(),
        total_marks: p.total_marks as i32,
        duration_minutes: p.duration_minutes as i32,
        date: p.date,
        status: p.status as i32,
        pdf_key: p.pdf_key.clone(),
        ms_key: p.ms_key.clone(),
        generation_mode: p.generation_mode as i32,
        instructions: p.instructions.clone(),
        created: p.created,
        updated: p.updated,
    }
}

impl<C: Send + Sync + 'static> PaperService for PaperServiceImpl<C> {
    type Config = Arc<C>;

    fn new(config: Self::Config) -> PaperServiceServer<Self> {
        PaperServiceServer::new(Self { config })
    }

    async fn create_paper(
        &self,
        token: Token,
        req: CreatePaperRequest,
    ) -> Result<CreatePaperResponse> {
        let user_id = token.user.to_string();
        let now = chrono::Utc::now().timestamp();
        let paper = CONN.with(|conn| -> Result<Paper> {
            let new_paper = Paper {
                id: Id::default(),
                school: req.school.clone(),
                event: req.event.clone(),
                subject: req.subject,
                grade: req.grade as i16,
                stream: req.stream.map(|s| s as i16),
                type_: (req.r#type as i16).try_into().unwrap_or_default(),
                teacher: user_id.clone(),
                name: req.name.clone(),
                total_marks: req.total_marks as i16,
                duration_minutes: req.duration_minutes as i16,
                date: req.date,
                status: PaperStatus::Draft,
                pdf_key: None,
                ms_key: None,
                generation_mode: (req.generation_mode as i16).try_into().unwrap_or_default(),
                instructions: req.instructions.clone(),
                created: now,
                updated: now,
            };
            let paper = papers_db::insert_paper(conn, &new_paper)?;
            if !req.topic_weights.is_empty() {
                let topics: Vec<(i32, f32)> = req
                    .topic_weights
                    .iter()
                    .map(|tw| (tw.topic_id, tw.weight))
                    .collect();
                papers_db::set_paper_topics(conn, &paper.id.to_string(), &topics)?;
            }
            Ok(paper)
        })?;
        Ok(CreatePaperResponse {
            paper: Some(paper_to_proto(&paper)),
        })
    }

    async fn get_paper(&self, _token: Token, req: GetPaperRequest) -> Result<GetPaperResponse> {
        let paper = CONN
            .with(|conn| {
                papers_db::get_paper(conn, &req.paper_id)
            })?
            .ok_or(Error::PaperNotFound)?;
        Ok(GetPaperResponse {
            paper: Some(paper_to_proto(&paper)),
        })
    }

    async fn list_papers(
        &self,
        _token: Token,
        req: ListPapersRequest,
    ) -> Result<ListPapersResponse> {
        let papers = CONN.with(|conn| {
            papers_db::list_papers(
                conn,
                &req.school,
                req.event.as_deref(),
                req.grade.map(|g| g as i16),
                req.subject,
            )
        })?;
        Ok(ListPapersResponse {
            papers: papers.iter().map(paper_to_proto).collect(),
        })
    }

    async fn update_paper(
        &self,
        _token: Token,
        req: UpdatePaperRequest,
    ) -> Result<UpdatePaperResponse> {
        let paper = CONN.with(|conn| {
            let existing =
                papers_db::get_paper(conn, &req.paper_id)?.ok_or(Error::PaperNotFound)?;
            if existing.status >= PaperStatus::Finalized {
                return Err(Error::PaperAlreadyFinalized);
            }
            let update = PaperUpdate {
                name: req.name.clone(),
                total_marks: req.total_marks.map(|m| m as i16),
                duration_minutes: req.duration_minutes.map(|d| d as i16),
                date: req.date,
                instructions: req.instructions.map(Some),
                generation_mode: req.generation_mode.and_then(|m| (m as i16).try_into().ok()),
                updated: Some(chrono::Utc::now().timestamp()),
                ..Default::default()
            };
            papers_db::update_paper(conn, &req.paper_id, update)
        })?;
        Ok(UpdatePaperResponse {
            paper: Some(paper_to_proto(&paper)),
        })
    }

    async fn get_paper_pdf_url(
        &self,
        _token: Token,
        req: GetPaperPdfUrlRequest,
    ) -> Result<GetPaperPdfUrlResponse> {
        let paper = CONN
            .with(|conn| {
                papers_db::get_paper(conn, &req.paper_id)
            })?
            .ok_or(Error::PaperNotFound)?;
        let key = paper.pdf_key.ok_or(Error::Forbidden)?;
        let url = sign::url(&key, sign::GET_TTL, false);
        let expiry = chrono::Utc::now().timestamp() + sign::GET_TTL as i64;
        Ok(GetPaperPdfUrlResponse { url, expiry })
    }

    async fn get_marking_scheme_url(
        &self,
        _token: Token,
        req: GetMarkingSchemeUrlRequest,
    ) -> Result<GetMarkingSchemeUrlResponse> {
        let paper = CONN
            .with(|conn| {
                papers_db::get_paper(conn, &req.paper_id)
            })?
            .ok_or(Error::PaperNotFound)?;
        let key = paper.ms_key.ok_or(Error::Forbidden)?;
        let url = sign::url(&key, sign::GET_TTL, false);
        let expiry = chrono::Utc::now().timestamp() + sign::GET_TTL as i64;
        Ok(GetMarkingSchemeUrlResponse { url, expiry })
    }

    async fn force_set_paper_status(
        &self,
        _token: Token,
        req: ForceSetPaperStatusRequest,
    ) -> Result<ForceSetPaperStatusResponse> {
        let paper = CONN.with(|conn| {
            let status: PaperStatus = (req.status as i16)
                .try_into()
                .map_err(|_| Error::NotFound)?;
            papers_db::force_set_paper_status(conn, &req.paper_id, status)
        })?;
        Ok(ForceSetPaperStatusResponse {
            paper: Some(paper_to_proto(&paper)),
        })
    }
}
