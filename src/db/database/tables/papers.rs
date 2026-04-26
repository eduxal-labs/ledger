#![allow(dead_code, unused_imports)]

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Float, Integer, Nullable, SmallInt, Text};
use diesel::sqlite::SqliteConnection;

use crate::db::schema::{
    enrollments, grades, paper_questions, paper_topics, papers, student_pdf_keys,
};
use crate::types::error::{Error, Result};
use crate::types::paper::{GenerationMode, Paper, PaperStatus, PaperTopic, PaperUpdate};

// ── sql_query helper structs ─────────────────────────────────────────────────

#[derive(diesel::QueryableByName)]
struct IdRow {
    #[diesel(sql_type = Text)]
    pub id: String,
}

#[derive(diesel::QueryableByName)]
struct AdmRow {
    #[diesel(sql_type = Integer)]
    pub student: i32,
}

// ── Paper CRUD ───────────────────────────────────────────────────────────────

pub fn insert_paper(conn: &mut SqliteConnection, paper: &Paper) -> Result<Paper> {
    diesel::insert_into(papers::table)
        .values(paper)
        .get_result(conn)
        .map_err(Error::internal)
}

pub fn get_paper(conn: &mut SqliteConnection, id: &str) -> Result<Option<Paper>> {
    papers::table
        .filter(papers::id.eq(id))
        .first(conn)
        .optional()
        .map_err(Error::internal)
}

pub fn list_papers(
    conn: &mut SqliteConnection,
    school: &str,
    event: Option<&str>,
    grade: Option<i16>,
    subject: Option<i32>,
) -> Result<Vec<Paper>> {
    let mut query = papers::table.filter(papers::school.eq(school)).into_boxed();

    if let Some(e) = event {
        query = query.filter(papers::event.eq(e));
    }
    if let Some(g) = grade {
        query = query.filter(papers::grade.eq(g));
    }
    if let Some(s) = subject {
        query = query.filter(papers::subject.eq(s));
    }

    query
        .order(papers::created.desc())
        .load(conn)
        .map_err(Error::internal)
}

pub fn update_paper(conn: &mut SqliteConnection, id: &str, update: PaperUpdate) -> Result<Paper> {
    diesel::update(papers::table.filter(papers::id.eq(id)))
        .set(&update)
        .get_result(conn)
        .map_err(Error::internal)
}

pub fn force_set_paper_status(
    conn: &mut SqliteConnection,
    id: &str,
    status: PaperStatus,
) -> Result<Paper> {
    diesel::update(papers::table.filter(papers::id.eq(id)))
        .set(papers::status.eq(status))
        .get_result(conn)
        .map_err(Error::internal)
}

pub fn transition_paper_status(
    conn: &mut SqliteConnection,
    id: &str,
    new_status: PaperStatus,
) -> Result<Paper> {
    let paper = get_paper(conn, id)?.ok_or(Error::PaperNotFound)?;

    let valid = match paper.status {
        PaperStatus::Draft => matches!(new_status, PaperStatus::QuestionsSet),
        PaperStatus::QuestionsSet => {
            matches!(new_status, PaperStatus::Finalized | PaperStatus::Draft)
        }
        PaperStatus::Finalized => {
            matches!(
                new_status,
                PaperStatus::Revealed | PaperStatus::QuestionsSet
            )
        }
        PaperStatus::Revealed => matches!(new_status, PaperStatus::Active),
        PaperStatus::Active => matches!(new_status, PaperStatus::Completed),
        PaperStatus::Completed => matches!(new_status, PaperStatus::Marked),
        PaperStatus::Marked => false,
    };

    if !valid {
        return Err(Error::InvalidStatusTransition);
    }

    diesel::update(papers::table.filter(papers::id.eq(id)))
        .set(papers::status.eq(new_status))
        .get_result(conn)
        .map_err(Error::internal)
}

// ── Paper topics ─────────────────────────────────────────────────────────────

pub fn set_paper_topics(
    conn: &mut SqliteConnection,
    paper_id: &str,
    topics: &[(i32, f32)],
) -> Result<()> {
    diesel::delete(paper_topics::table.filter(paper_topics::paper.eq(paper_id)))
        .execute(conn)
        .map_err(Error::internal)?;

    if !topics.is_empty() {
        let records: Vec<PaperTopic> = topics
            .iter()
            .map(|(topic, weight)| PaperTopic {
                paper: paper_id.to_string(),
                topic: *topic,
                weight: *weight,
            })
            .collect();

        diesel::insert_into(paper_topics::table)
            .values(&records)
            .execute(conn)
            .map_err(Error::internal)?;
    }

    Ok(())
}

pub fn get_paper_topics(conn: &mut SqliteConnection, paper_id: &str) -> Result<Vec<PaperTopic>> {
    paper_topics::table
        .filter(paper_topics::paper.eq(paper_id))
        .load(conn)
        .map_err(Error::internal)
}

// ── Student PDF keys ──────────────────────────────────────────────────────────

pub fn upsert_student_pdf_key(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: i32,
    pdf_key: &str,
) -> Result<()> {
    diesel::sql_query(
        "INSERT OR REPLACE INTO student_pdf_keys(paper, student, pdf_key) VALUES(?,?,?)",
    )
    .bind::<Text, _>(paper_id)
    .bind::<Integer, _>(student)
    .bind::<Text, _>(pdf_key)
    .execute(conn)
    .map_err(Error::internal)?;

    Ok(())
}

pub fn get_student_pdf_key(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: i32,
) -> Result<Option<String>> {
    student_pdf_keys::table
        .filter(student_pdf_keys::paper.eq(paper_id))
        .filter(student_pdf_keys::student.eq(student))
        .select(student_pdf_keys::pdf_key)
        .first(conn)
        .optional()
        .map_err(Error::internal)
}

pub fn list_student_pdf_keys(
    conn: &mut SqliteConnection,
    paper_id: &str,
) -> Result<Vec<(i32, String)>> {
    student_pdf_keys::table
        .filter(student_pdf_keys::paper.eq(paper_id))
        .select((student_pdf_keys::student, student_pdf_keys::pdf_key))
        .load(conn)
        .map_err(Error::internal)
}

// ── Grades ────────────────────────────────────────────────────────────────────

pub fn upsert_grade(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: i32,
    score: f32,
) -> Result<()> {
    diesel::sql_query("INSERT OR REPLACE INTO grades(paper, student, score) VALUES(?,?,?)")
        .bind::<Text, _>(paper_id)
        .bind::<Integer, _>(student)
        .bind::<Float, _>(score)
        .execute(conn)
        .map_err(Error::internal)?;

    Ok(())
}

pub fn get_grade(conn: &mut SqliteConnection, paper_id: &str, student: i32) -> Result<Option<f32>> {
    grades::table
        .filter(grades::paper.eq(paper_id))
        .filter(grades::student.eq(student))
        .select(grades::score)
        .first(conn)
        .optional()
        .map_err(Error::internal)
}

// ── Scheduler helpers ─────────────────────────────────────────────────────────

/// Returns paper IDs where status=Finalized(2) AND linked schedule.reveal_at <= now.
pub fn get_papers_due_for_reveal(conn: &mut SqliteConnection) -> Result<Vec<String>> {
    let rows: Vec<IdRow> = diesel::sql_query(
        "SELECT DISTINCT p.id FROM papers p \
         JOIN paper_schedules ps ON ps.paper = p.id \
         WHERE ps.reveal_at <= unixepoch('now') AND p.status = 2",
    )
    .load(conn)
    .map_err(Error::internal)?;

    Ok(rows.into_iter().map(|r| r.id).collect())
}

/// Returns distinct student admission numbers enrolled in school+grade+stream.
pub fn get_enrolled_students(
    conn: &mut SqliteConnection,
    school: &str,
    grade: i16,
    stream: Option<i16>,
) -> Result<Vec<i32>> {
    let rows: Vec<AdmRow> = if let Some(s) = stream {
        diesel::sql_query(
            "SELECT DISTINCT student FROM enrollments \
             WHERE school = ? AND grade = ? AND stream = ?",
        )
        .bind::<Text, _>(school)
        .bind::<SmallInt, _>(grade)
        .bind::<SmallInt, _>(s)
        .load(conn)
        .map_err(Error::internal)?
    } else {
        diesel::sql_query(
            "SELECT DISTINCT student FROM enrollments \
             WHERE school = ? AND grade = ?",
        )
        .bind::<Text, _>(school)
        .bind::<SmallInt, _>(grade)
        .load(conn)
        .map_err(Error::internal)?
    };

    Ok(rows.into_iter().map(|r| r.student).collect())
}
