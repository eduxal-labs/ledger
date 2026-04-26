use crate::db::schema::events;
use crate::types::error::{Error, Result};
use crate::types::event::{Event, EventStatus, EventUpdate};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

pub fn insert_event(conn: &mut SqliteConnection, event: &Event) -> Result<Event> {
    diesel::insert_into(events::table)
        .values(event)
        .get_result(conn)
        .map_err(Error::internal)
}

pub fn get_event(conn: &mut SqliteConnection, id: &str) -> Result<Option<Event>> {
    events::table
        .filter(events::id.eq(id))
        .first(conn)
        .optional()
        .map_err(Error::internal)
}

pub fn list_events(
    conn: &mut SqliteConnection,
    school: &str,
    year: Option<i32>,
    term: Option<i16>,
) -> Result<Vec<Event>> {
    let mut query = events::table.filter(events::school.eq(school)).into_boxed();
    if let Some(y) = year {
        query = query.filter(events::year.eq(y));
    }
    if let Some(t) = term {
        query = query.filter(events::term.eq(t));
    }
    query
        .order(events::start_date.desc())
        .load(conn)
        .map_err(Error::internal)
}

pub fn update_event(conn: &mut SqliteConnection, id: &str, update: EventUpdate) -> Result<Event> {
    diesel::update(events::table.filter(events::id.eq(id)))
        .set(&update)
        .get_result(conn)
        .map_err(Error::internal)
}

pub fn delete_event(conn: &mut SqliteConnection, id: &str) -> Result<bool> {
    let affected = diesel::update(events::table.filter(events::id.eq(id)))
        .set(events::status.eq(EventStatus::Cancelled))
        .execute(conn)
        .map_err(Error::internal)?;
    Ok(affected > 0)
}
