use crate::config::Configuration;
use crate::db::changelog::{LOG, Record};
use crate::proto::services::ai_marking::AiMarking;
use crate::proto::services::authentication::Authentication;
use crate::proto::services::event_service::EventService;
use crate::proto::services::paper_management::PaperManagement;
use crate::proto::services::paper_service::PaperService;
use crate::proto::services::question_bank::QuestionBank;
use crate::proto::services::sync::Sync;
use crate::services::ai_marking::AiMarkingService;
use crate::services::authentication::Authenticator;
use crate::services::events::EventServiceImpl;
use crate::services::paper_management::PaperManagementServiceImpl;
use crate::services::papers::PaperServiceImpl;
use crate::services::question_bank::QuestionBankService;
use crate::services::sync::SyncService;
use crate::types::id::Id;
use std::sync::Arc;
use tonic::transport::Server;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static>>;

pub async fn start() -> Result<()> {
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 50051));
    let config = Arc::new(Configuration::default());

    // Emit resync changelog records for the global catalog tables (subjects=31, topics=32).
    //
    // Rationale: Topics and subjects that were bulk-inserted directly into SQLite
    // (bypassing the gRPC action handler) have no changelog entries, so clients
    // in incremental-sync mode (last_seq > 0) never receive them.  By appending
    // a record with created=0, the watch loop calls snapshot_table_since with
    // min_ts=0, which returns ALL rows regardless of their updated timestamp.
    //
    // This also ensures clients receive the corrected topic grade codes after the
    // 2026-04-15-000000-0004_fix_topic_grades migration updates them from 1–4 to 41–44.
    for table in [31u8, 32u8] {
        // 31 = SubjectCatalog, 32 = Topics
        let record = Record {
            user: Id::system().bytes(),
            table,
            op: 0, // OP_INSERT (upsert semantics in the watch loop)
            columns: 0,
            created: 0, // 0 forces min_ts=0 → snapshot_table_since returns ALL rows
        };
        if let Err(e) = LOG.with(|cell| cell.borrow_mut().append(&record)) {
            eprintln!("[STARTUP] Warning: failed to emit resync for table {table}: {e}");
        }
    }
    tracing::info!("[STARTUP] Emitted subject-catalog + topics resync changelog records");

    tokio::spawn(crate::services::generation::run_generation_scheduler());

    let authenticator = Authenticator::new(config.clone());
    let sync = SyncService::new(config.clone());
    let ai_marking = AiMarkingService::new(config.clone());
    let question_bank = QuestionBankService::new(config.clone());
    let event_service = EventServiceImpl::new(config.clone());
    let paper_service = PaperServiceImpl::new(config.clone());
    let paper_management = PaperManagementServiceImpl::new(config.clone());
    Server::builder()
        .add_service(authenticator)
        .add_service(sync)
        .add_service(ai_marking)
        .add_service(question_bank)
        .add_service(event_service)
        .add_service(paper_service)
        .add_service(paper_management)
        .serve(addr)
        .await?;
    Ok(())
}
