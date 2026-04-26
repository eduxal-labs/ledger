use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::schema::{exam_coverage, paper_schedules, taught_topics};
use crate::types::error::{Error, Result};
use crate::types::paper::{
    ExamCoverage, GenerationStatus, PaperSchedule, PaperScheduleUpdate, TaughtTopic,
};

// ── PaperSchedule functions ──────────────────────────────────────────────────

pub fn insert_schedule(
    conn: &mut SqliteConnection,
    schedule: &PaperSchedule,
) -> Result<PaperSchedule> {
    diesel::insert_into(paper_schedules::table)
        .values(schedule)
        .get_result(conn)
        .map_err(Error::internal)
}

pub fn get_schedule(conn: &mut SqliteConnection, id: &str) -> Result<Option<PaperSchedule>> {
    paper_schedules::table
        .filter(paper_schedules::id.eq(id))
        .first(conn)
        .optional()
        .map_err(Error::internal)
}

pub fn list_schedules(conn: &mut SqliteConnection, event_id: &str) -> Result<Vec<PaperSchedule>> {
    paper_schedules::table
        .filter(paper_schedules::event.eq(event_id))
        .order(paper_schedules::date.asc())
        .load(conn)
        .map_err(Error::internal)
}

pub fn update_schedule(
    conn: &mut SqliteConnection,
    id: &str,
    update: PaperScheduleUpdate,
) -> Result<PaperSchedule> {
    diesel::update(paper_schedules::table.filter(paper_schedules::id.eq(id)))
        .set(&update)
        .get_result(conn)
        .map_err(Error::internal)
}

/// UPDATE paper_schedules SET invigilator = ? WHERE id = ?
pub fn assign_invigilator(
    conn: &mut SqliteConnection,
    schedule_id: &str,
    invigilator: Option<&str>,
) -> Result<()> {
    diesel::update(paper_schedules::table.filter(paper_schedules::id.eq(schedule_id)))
        .set(paper_schedules::invigilator.eq(invigilator))
        .execute(conn)
        .map_err(Error::internal)?;
    Ok(())
}

/// UPDATE paper_schedules SET paper = ? WHERE id = ?
pub fn link_paper_to_schedule(
    conn: &mut SqliteConnection,
    schedule_id: &str,
    paper_id: &str,
) -> Result<()> {
    diesel::update(paper_schedules::table.filter(paper_schedules::id.eq(schedule_id)))
        .set(paper_schedules::paper.eq(paper_id))
        .execute(conn)
        .map_err(Error::internal)?;
    Ok(())
}

/// SELECT * FROM paper_schedules WHERE generation_status = 0 AND generate_at <= unixepoch('now')
pub fn get_pending_generation(conn: &mut SqliteConnection) -> Result<Vec<PaperSchedule>> {
    use diesel::dsl::sql;
    use diesel::sql_types::BigInt;

    paper_schedules::table
        .filter(paper_schedules::generation_status.eq(GenerationStatus::Pending))
        .filter(paper_schedules::generate_at.le(sql::<BigInt>("unixepoch('now')")))
        .load(conn)
        .map_err(Error::internal)
}

/// UPDATE paper_schedules SET generation_status = ? WHERE id = ?
/// The `_error` parameter is accepted for API compatibility but not stored
/// (the schema has no `error` column on paper_schedules).
pub fn set_generation_status(
    conn: &mut SqliteConnection,
    id: &str,
    status: GenerationStatus,
    _error: Option<&str>,
) -> Result<()> {
    diesel::update(paper_schedules::table.filter(paper_schedules::id.eq(id)))
        .set(paper_schedules::generation_status.eq(status))
        .execute(conn)
        .map_err(Error::internal)?;
    Ok(())
}

// ── TaughtTopics functions ───────────────────────────────────────────────────

/// INSERT OR REPLACE INTO taught_topics(...)
pub fn upsert_taught_topic(conn: &mut SqliteConnection, topic: &TaughtTopic) -> Result<()> {
    diesel::replace_into(taught_topics::table)
        .values(topic)
        .execute(conn)
        .map_err(Error::internal)?;
    Ok(())
}

/// Return all taught topics for the given school + subject + grade.
/// If `stream` is `Some`, additionally filter by stream.
/// If `stream` is `None`, return rows for all streams.
pub fn get_taught_topics(
    conn: &mut SqliteConnection,
    school: &str,
    subject: i32,
    grade: i16,
    stream: Option<i16>,
) -> Result<Vec<TaughtTopic>> {
    let mut query = taught_topics::table
        .filter(taught_topics::school.eq(school))
        .filter(taught_topics::subject.eq(subject))
        .filter(taught_topics::grade.eq(grade))
        .into_boxed();

    if let Some(s) = stream {
        query = query.filter(taught_topics::stream.eq(s));
    }

    query.load(conn).map_err(Error::internal)
}

/// Return topic IDs (i32) that are completed (status = 2) for the exam schedule's
/// school + subject + grade + stream combination.
///
/// Uses a three-way JOIN:
///   taught_topics → paper_schedules (on subject/grade/stream)
///                 → events          (on school)
pub fn get_completed_topics_for_schedule(
    conn: &mut SqliteConnection,
    schedule_id: &str,
) -> Result<Vec<i32>> {
    #[derive(QueryableByName)]
    struct TopicRow {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        topic: i32,
    }

    let rows: Vec<TopicRow> = diesel::sql_query(
        "SELECT tt.topic \
         FROM taught_topics tt \
         JOIN paper_schedules ps \
           ON ps.event IS NOT NULL \
          AND tt.subject = ps.subject \
          AND tt.grade   = ps.grade \
          AND (tt.stream IS NULL OR tt.stream = ps.stream) \
         JOIN events e \
           ON e.id = ps.event \
          AND tt.school = e.school \
         WHERE ps.id = ? AND tt.status = 2",
    )
    .bind::<diesel::sql_types::Text, _>(schedule_id)
    .load(conn)
    .map_err(Error::internal)?;

    Ok(rows.into_iter().map(|r| r.topic).collect())
}

// ── ExamCoverage functions ───────────────────────────────────────────────────

/// Atomically replace all exam coverage for a schedule:
///   DELETE FROM exam_coverage WHERE schedule = ?
///   INSERT INTO exam_coverage(schedule, topic, confirmed_by, confirmed_at) VALUES ...
///
/// Returns the count of topics confirmed.
pub fn confirm_exam_coverage(
    conn: &mut SqliteConnection,
    schedule_id: &str,
    topic_ids: &[i32],
    confirmed_by: &str,
) -> Result<usize> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    conn.transaction(|conn| {
        diesel::delete(exam_coverage::table.filter(exam_coverage::schedule.eq(schedule_id)))
            .execute(conn)
            .map_err(Error::internal)?;

        if topic_ids.is_empty() {
            return Ok(0);
        }

        let records: Vec<ExamCoverage> = topic_ids
            .iter()
            .map(|&topic| ExamCoverage {
                schedule: schedule_id.to_string(),
                topic,
                confirmed_by: confirmed_by.to_string(),
                confirmed_at: now,
            })
            .collect();

        let count = diesel::insert_into(exam_coverage::table)
            .values(&records)
            .execute(conn)
            .map_err(Error::internal)?;

        Ok(count)
    })
}

/// SELECT topic FROM exam_coverage WHERE schedule = ? ORDER BY topic
pub fn get_exam_coverage(conn: &mut SqliteConnection, schedule_id: &str) -> Result<Vec<i32>> {
    exam_coverage::table
        .filter(exam_coverage::schedule.eq(schedule_id))
        .order(exam_coverage::topic.asc())
        .select(exam_coverage::topic)
        .load(conn)
        .map_err(Error::internal)
}
