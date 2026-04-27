use std::sync::Arc;

use crate::config::storage::sign;
use crate::db::database::CONN;
use crate::db::database::tables::paper_management as pm_db;
use crate::db::database::tables::papers as papers_db;
use crate::proto::services::paper_management::*;
use crate::types::error::{Error, Result};
use crate::types::id::Id;
use crate::types::paper::{
    GenerationStatus, PaperSchedule, PaperScheduleUpdate, TaughtStatus, TaughtTopic,
};
use crate::types::token::Token;

pub struct PaperManagementServiceImpl<C> {
    #[allow(dead_code)]
    config: Arc<C>,
}

fn schedule_to_proto(s: &PaperSchedule) -> PaperScheduleProto {
    PaperScheduleProto {
        id: s.id.to_string(),
        event: s.event.clone(),
        subject: s.subject,
        grade: s.grade as i32,
        stream: s.stream.map(|v| v as i32),
        date: s.date,
        start_time: s.start_time,
        end_time: s.end_time,
        duration_minutes: s.duration_minutes as i32,
        invigilator: s.invigilator.clone(),
        paper: s.paper.clone(),
        generation_status: s.generation_status as i32,
        reveal_at: s.reveal_at,
        generate_at: s.generate_at,
        created: s.created,
    }
}

impl<C: Send + Sync + 'static> PaperManagement for PaperManagementServiceImpl<C> {
    type Config = Arc<C>;

    fn new(config: Self::Config) -> PaperManagementServer<Self> {
        PaperManagementServer::new(Self { config })
    }

    async fn schedule_paper(
        &self,
        token: Token,
        req: SchedulePaperRequest,
    ) -> Result<SchedulePaperResponse> {
        let _user_id = token.user.to_string();
        let now = chrono::Utc::now().timestamp();

        let schedule = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            let new_schedule = PaperSchedule {
                id: Id::default(),
                event: req.event_id.clone(),
                subject: req.subject,
                grade: req.grade as i16,
                stream: req.stream.map(|s| s as i16),
                date: req.date,
                start_time: req.start_time,
                end_time: req.end_time,
                duration_minutes: req.duration_minutes as i16,
                invigilator: req.invigilator.clone(),
                paper: None,
                generation_status: GenerationStatus::Pending,
                reveal_at: req.reveal_at,
                generate_at: req.generate_at,
                created: now,
            };
            pm_db::insert_schedule(conn, &new_schedule)
        })?;

        Ok(SchedulePaperResponse {
            schedule_id: schedule.id.to_string(),
        })
    }

    async fn assign_invigilator(
        &self,
        _token: Token,
        req: AssignInvigilatorRequest,
    ) -> Result<AssignInvigilatorResponse> {
        CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            pm_db::assign_invigilator(conn, &req.schedule_id, req.invigilator.as_deref())
        })?;

        Ok(AssignInvigilatorResponse {})
    }

    async fn list_paper_schedules(
        &self,
        _token: Token,
        req: ListSchedulesRequest,
    ) -> Result<ListSchedulesResponse> {
        let schedules = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            pm_db::list_schedules(conn, &req.event_id)
        })?;

        Ok(ListSchedulesResponse {
            schedules: schedules.iter().map(schedule_to_proto).collect(),
        })
    }

    async fn update_schedule(
        &self,
        _token: Token,
        req: UpdateScheduleRequest,
    ) -> Result<UpdateScheduleResponse> {
        let schedule = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();

            let existing =
                pm_db::get_schedule(conn, &req.schedule_id)?.ok_or(Error::PaperScheduleNotFound)?;

            if existing.generation_status != GenerationStatus::Pending {
                return Err(Error::InvalidStatusTransition);
            }

            let update = PaperScheduleUpdate {
                date: req.date,
                start_time: req.start_time,
                end_time: req.end_time,
                duration_minutes: req.duration_minutes.map(|d| d as i16),
                reveal_at: req.reveal_at,
                generate_at: req.generate_at,
                ..Default::default()
            };

            pm_db::update_schedule(conn, &req.schedule_id, update)
        })?;

        Ok(UpdateScheduleResponse {
            schedule: Some(schedule_to_proto(&schedule)),
        })
    }

    async fn set_taught_topics(
        &self,
        token: Token,
        req: SetTaughtTopicsRequest,
    ) -> Result<SetTaughtTopicsResponse> {
        let user_id = token.user.to_string();
        let now = chrono::Utc::now().timestamp();

        CONN.with(|cell| -> Result<()> {
            let conn = &mut *cell.borrow_mut();
            for topic_proto in &req.topics {
                let status = match topic_proto.status {
                    0 => TaughtStatus::NotStarted,
                    1 => TaughtStatus::InProgress,
                    2 => TaughtStatus::Completed,
                    _ => TaughtStatus::NotStarted,
                };
                let topic = TaughtTopic {
                    school: req.school.clone(),
                    subject: req.subject,
                    grade: req.grade as i16,
                    stream: req.stream.map(|s| s as i16),
                    topic: topic_proto.topic_id,
                    taught_by: user_id.clone(),
                    status,
                    taught_date: topic_proto.taught_date,
                    updated: now,
                };
                pm_db::upsert_taught_topic(conn, &topic)?;
            }
            Ok(())
        })?;

        Ok(SetTaughtTopicsResponse {})
    }

    async fn get_taught_topics(
        &self,
        _token: Token,
        req: GetTaughtTopicsRequest,
    ) -> Result<GetTaughtTopicsResponse> {
        let topics = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            pm_db::get_taught_topics(
                conn,
                &req.school,
                req.subject,
                req.grade as i16,
                req.stream.map(|s| s as i16),
            )
        })?;

        let proto_topics: Vec<TaughtTopicProto> = topics
            .iter()
            .map(|t| TaughtTopicProto {
                topic_id: t.topic,
                status: t.status as i32,
                taught_date: t.taught_date,
            })
            .collect();

        Ok(GetTaughtTopicsResponse {
            topics: proto_topics,
        })
    }

    async fn confirm_exam_coverage(
        &self,
        token: Token,
        req: ConfirmExamCoverageRequest,
    ) -> Result<ConfirmExamCoverageResponse> {
        let confirmed_by = token.user.to_string();

        let count = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();

            let topic_ids = if req.topic_ids.is_empty() {
                pm_db::get_completed_topics_for_schedule(conn, &req.schedule_id)?
            } else {
                req.topic_ids.clone()
            };

            pm_db::confirm_exam_coverage(conn, &req.schedule_id, &topic_ids, &confirmed_by)
        })?;

        Ok(ConfirmExamCoverageResponse {
            topics_confirmed: count as i32,
        })
    }

    async fn get_exam_coverage(
        &self,
        _token: Token,
        req: GetExamCoverageRequest,
    ) -> Result<GetExamCoverageResponse> {
        let topic_ids = CONN.with(|cell| {
            let conn = &mut *cell.borrow_mut();
            pm_db::get_exam_coverage(conn, &req.schedule_id)
        })?;

        Ok(GetExamCoverageResponse { topic_ids })
    }

    async fn generate_assessment(
        &self,
        _token: Token,
        req: GenerateAssessmentRequest,
    ) -> Result<GenerateAssessmentResponse> {
        let paper_id_clone = req.paper_id.clone();
        tokio::spawn(async move {
            crate::services::generation::enqueue_assessment(&paper_id_clone).await;
        });

        Ok(GenerateAssessmentResponse {
            accepted: true,
            message: "Assessment generation enqueued".to_string(),
        })
    }

    async fn generate_assignment(
        &self,
        _token: Token,
        req: GenerateAssignmentRequest,
    ) -> Result<GenerateAssignmentResponse> {
        let paper_id_clone = req.paper_id.clone();
        tokio::spawn(async move {
            crate::services::generation::enqueue_assignment(&paper_id_clone).await;
        });

        Ok(GenerateAssignmentResponse {
            accepted: true,
            message: "Assignment generation enqueued".to_string(),
        })
    }

    async fn finalize_student_papers(
        &self,
        _token: Token,
        req: FinalizeStudentPapersRequest,
    ) -> Result<FinalizeStudentPapersResponse> {
        let paper_id_clone = req.paper_id.clone();
        tokio::spawn(async move {
            crate::services::generation::finalize_student_papers_job(&paper_id_clone).await;
        });

        Ok(FinalizeStudentPapersResponse {
            job_id: req.paper_id.clone(),
        })
    }

    async fn get_student_papers_status(
        &self,
        _token: Token,
        req: GetStudentPapersStatusRequest,
    ) -> Result<GetStudentPapersStatusResponse> {
        let (job_id, total, generated_count, statuses) = CONN.with(
            |cell| -> Result<(String, i32, i32, Vec<StudentPdfStatus>)> {
                let conn = &mut *cell.borrow_mut();

                let paper =
                    papers_db::get_paper(conn, &req.paper_id)?.ok_or(Error::PaperNotFound)?;

                let enrolled = papers_db::get_enrolled_students(
                    conn,
                    &paper.school,
                    paper.grade,
                    paper.stream,
                )?;
                let pdf_keys = papers_db::list_student_pdf_keys(conn, &req.paper_id)?;

                let generated_set: std::collections::HashSet<i32> =
                    pdf_keys.iter().map(|(s, _)| *s).collect();

                let statuses: Vec<StudentPdfStatus> = enrolled
                    .iter()
                    .map(|&s| StudentPdfStatus {
                        student: s,
                        generated: generated_set.contains(&s),
                        error: None,
                    })
                    .collect();

                let total = enrolled.len() as i32;
                let generated = generated_set.len() as i32;

                Ok((req.paper_id.clone(), total, generated, statuses))
            },
        )?;

        let complete = generated_count == total && total > 0;

        Ok(GetStudentPapersStatusResponse {
            job_id,
            complete,
            total,
            generated: generated_count,
            statuses,
        })
    }

    async fn get_student_paper_pdf(
        &self,
        _token: Token,
        req: GetStudentPaperPdfRequest,
    ) -> Result<GetStudentPaperPdfResponse> {
        let key = CONN
            .with(|cell| {
                let conn = &mut *cell.borrow_mut();
                papers_db::get_student_pdf_key(conn, &req.paper_id, req.student)
            })?
            .ok_or(Error::PaperNotFound)?;

        let pdf_url = sign::url(&key, sign::GET_TTL, false);
        let expiry = chrono::Utc::now().timestamp() + sign::GET_TTL as i64;

        Ok(GetStudentPaperPdfResponse { pdf_url, expiry })
    }
}
