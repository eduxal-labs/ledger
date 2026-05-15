use std::sync::Arc;

use crate::db::changelog::{self, Record};
use crate::db::database::CONN;
use crate::db::database::tables::events as events_db;
use crate::proto::services::event_service::*;
use crate::types::error::{Error, Result};
use crate::types::event::{Event, EventUpdate};
use crate::types::id::Id;
use crate::types::token::Token;

pub struct EventServiceImpl<C> {
    #[allow(dead_code)]
    config: Arc<C>,
}

fn event_to_proto(e: &Event) -> crate::proto::types::event::Event {
    crate::proto::types::event::Event {
        id: e.id.to_string(),
        school: e.school.clone(),
        name: e.name.clone(),
        r#type: e.type_ as i32,
        term: e.term as i32,
        year: e.year,
        start_date: e.start_date,
        end_date: e.end_date,
        status: e.status as i32,
        created: e.created,
        updated: e.updated,
    }
}

impl<C: Send + Sync + 'static> EventService for EventServiceImpl<C> {
    type Config = Arc<C>;

    fn new(config: Self::Config) -> EventServiceServer<Self> {
        EventServiceServer::new(Self { config })
    }

    async fn create_event(
        &self,
        token: Token,
        req: CreateEventRequest,
    ) -> Result<CreateEventResponse> {
        let user = token.user;
        let now = chrono::Utc::now().timestamp();
        let event = CONN.with(|conn| {
            let new_event = Event {
                id: Id::default(),
                school: req.school.clone(),
                name: req.name.clone(),
                type_: (req.r#type as i16).try_into().unwrap_or_default(),
                term: req.term as i16,
                year: req.year,
                start_date: req.start_date,
                end_date: req.end_date,
                status: Default::default(),
                created: now,
                updated: now,
            };
            events_db::insert_event(conn, &new_event)
        })?;
        let _ = changelog::LOG.with(|cell| {
            cell.borrow_mut().append(&Record {
                user: user.bytes(),
                table: 38,
                op: 0,
                columns: 0,
                created: now,
            })
        });
        changelog::NOTIFY.notify_waiters();
        Ok(CreateEventResponse {
            event: Some(event_to_proto(&event)),
        })
    }

    async fn get_event(&self, _token: Token, req: GetEventRequest) -> Result<GetEventResponse> {
        let event = CONN
            .with(|conn| {
                events_db::get_event(conn, &req.event_id)
            })?
            .ok_or(Error::NotFound)?;
        Ok(GetEventResponse {
            event: Some(event_to_proto(&event)),
        })
    }

    async fn list_events(
        &self,
        _token: Token,
        req: ListEventsRequest,
    ) -> Result<ListEventsResponse> {
        let events = CONN.with(|conn| {
            events_db::list_events(conn, &req.school, req.year, req.term.map(|t| t as i16))
        })?;
        Ok(ListEventsResponse {
            events: events.iter().map(event_to_proto).collect(),
        })
    }

    async fn update_event(
        &self,
        token: Token,
        req: UpdateEventRequest,
    ) -> Result<UpdateEventResponse> {
        let user = token.user;
        let now = chrono::Utc::now().timestamp();
        let event = CONN.with(|conn| {
            let update = EventUpdate {
                name: req.name.clone(),
                type_: req.r#type.and_then(|t| (t as i16).try_into().ok()),
                term: req.term.map(|t| t as i16),
                year: req.year,
                start_date: req.start_date,
                end_date: req.end_date,
                status: req.status.and_then(|s| (s as i16).try_into().ok()),
                updated: Some(now),
            };
            events_db::update_event(conn, &req.event_id, update)
        })?;
        let _ = changelog::LOG.with(|cell| {
            cell.borrow_mut().append(&Record {
                user: user.bytes(),
                table: 38,
                op: 0,
                columns: 0,
                created: now,
            })
        });
        changelog::NOTIFY.notify_waiters();
        Ok(UpdateEventResponse {
            event: Some(event_to_proto(&event)),
        })
    }

    async fn delete_event(
        &self,
        _token: Token,
        req: DeleteEventRequest,
    ) -> Result<DeleteEventResponse> {
        CONN.with(|conn| {
            events_db::delete_event(conn, &req.event_id)
        })?;
        let _ = changelog::LOG.with(|cell| {
            cell.borrow_mut().append_delete(38, &req.event_id)
        });
        changelog::NOTIFY.notify_waiters();
        Ok(DeleteEventResponse {})
    }
}
