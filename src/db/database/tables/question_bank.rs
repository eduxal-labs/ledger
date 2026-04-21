#![allow(dead_code)]

use super::rows::*;
use crate::types::error::Result;
use diesel::RunQueryDsl;
use diesel::SqliteConnection as Conn;
use diesel::result::OptionalExtension;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Float, Integer, Nullable, SmallInt, Text};

// ---------------------------------------------------------------------------
// Helper for COUNT(*) queries
// ---------------------------------------------------------------------------

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    pub count: i64,
}

// ---------------------------------------------------------------------------
// Helper for last_insert_rowid()
// ---------------------------------------------------------------------------

#[derive(diesel::QueryableByName)]
struct LastId {
    #[diesel(sql_type = Integer)]
    pub id: i32,
}

// =========================================================================
// Questions
// =========================================================================

/// Insert a new question. Returns the new question's auto-increment ID.
pub fn insert_question(
    conn: &mut Conn,
    topic: i32,
    text: &str,
    marks: i16,
    example_answer: Option<&str>,
    created_by: &str,
) -> Result<i32> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO questions (topic, text, marks, example_answer, created, updated, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Integer, _>(topic)
    .bind::<Text, _>(text)
    .bind::<SmallInt, _>(marks)
    .bind::<Nullable<Text>, _>(example_answer)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(created_by)
    .execute(conn)?;

    let row: LastId = sql_query("SELECT last_insert_rowid() AS id").get_result(conn)?;
    Ok(row.id)
}

/// Get-or-create a question by `(topic, text)`.
///
/// Returns `(id, is_new)`:
/// - `is_new = true`  — question was just inserted
/// - `is_new = false` — a question with the same (topic, text) already existed; the
///                      existing row's id is returned and nothing is written
///
/// Intended for use inside a transaction (e.g. bulk import) where
/// duplicate rows must be silently resolved rather than rejected.
pub fn find_or_insert_question(
    conn: &mut Conn,
    topic: i32,
    text: &str,
    marks: i16,
    example_answer: Option<&str>,
    created_by: &str,
) -> Result<(i32, bool)> {
    // Check for an existing question with the same (topic, text)
    let existing: Option<LastId> =
        sql_query("SELECT id FROM questions WHERE topic = ? AND text = ? LIMIT 1")
            .bind::<Integer, _>(topic)
            .bind::<Text, _>(text)
            .get_result(conn)
            .optional()?;

    if let Some(row) = existing {
        return Ok((row.id, false));
    }

    // No duplicate — insert the new question
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO questions \
         (topic, text, marks, example_answer, created, updated, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Integer, _>(topic)
    .bind::<Text, _>(text)
    .bind::<SmallInt, _>(marks)
    .bind::<Nullable<Text>, _>(example_answer)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(created_by)
    .execute(conn)?;

    let row: LastId = sql_query("SELECT last_insert_rowid() AS id").get_result(conn)?;
    Ok((row.id, true))
}

/// Update a question. Only updates fields that are `Some`. Always bumps `updated`.
///
/// `example_answer` uses a tri-state:
///   - `None`        → don't change
///   - `Some(None)`  → set to NULL
///   - `Some(Some(v))` → set to v
pub fn update_question(
    conn: &mut Conn,
    id: i32,
    text: Option<&str>,
    marks: Option<i16>,
    example_answer: Option<Option<&str>>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let ea_flag: i32 = if example_answer.is_some() { 1 } else { 0 };
    let ea_val: Option<&str> = example_answer.flatten();

    sql_query(
        "UPDATE questions SET \
         text = COALESCE(?, text), \
         marks = COALESCE(?, marks), \
         example_answer = CASE WHEN ? = 1 THEN ? ELSE example_answer END, \
         updated = ? \
         WHERE id = ?",
    )
    .bind::<Nullable<Text>, _>(text)
    .bind::<Nullable<SmallInt>, _>(marks)
    .bind::<Integer, _>(ea_flag)
    .bind::<Nullable<Text>, _>(ea_val)
    .bind::<BigInt, _>(now)
    .bind::<Integer, _>(id)
    .execute(conn)?;
    Ok(())
}

/// Delete a question by ID. CASCADE handles children tables.
pub fn delete_question(conn: &mut Conn, id: i32) -> Result<()> {
    sql_query("DELETE FROM questions WHERE id = ?")
        .bind::<Integer, _>(id)
        .execute(conn)?;
    Ok(())
}

/// Get a single question by ID.
pub fn get_question(conn: &mut Conn, id: i32) -> Result<QuestionRow> {
    let row: QuestionRow = sql_query("SELECT * FROM questions WHERE id = ?")
        .bind::<Integer, _>(id)
        .get_result(conn)?;
    Ok(row)
}

/// List questions for a topic with optional marks range filter.
/// Returns `(rows, total_count)`.
pub fn list_questions(
    conn: &mut Conn,
    topic: i32,
    min_marks: Option<i16>,
    max_marks: Option<i16>,
    offset: i32,
    limit: i32,
) -> Result<(Vec<QuestionRow>, i64)> {
    let count_row: CountRow = sql_query(
        "SELECT COUNT(*) AS count FROM questions \
         WHERE topic = ? \
         AND marks >= COALESCE(?, marks) \
         AND marks <= COALESCE(?, marks)",
    )
    .bind::<Integer, _>(topic)
    .bind::<Nullable<SmallInt>, _>(min_marks)
    .bind::<Nullable<SmallInt>, _>(max_marks)
    .get_result(conn)?;

    let rows: Vec<QuestionRow> = sql_query(
        "SELECT * FROM questions \
         WHERE topic = ? \
         AND marks >= COALESCE(?, marks) \
         AND marks <= COALESCE(?, marks) \
         ORDER BY id \
         LIMIT ? OFFSET ?",
    )
    .bind::<Integer, _>(topic)
    .bind::<Nullable<SmallInt>, _>(min_marks)
    .bind::<Nullable<SmallInt>, _>(max_marks)
    .bind::<Integer, _>(limit)
    .bind::<Integer, _>(offset)
    .load(conn)?;

    Ok((rows, count_row.count))
}

/// Count questions for a given topic.
pub fn count_questions_by_topic(conn: &mut Conn, topic: i32) -> Result<i64> {
    let row: CountRow = sql_query("SELECT COUNT(*) AS count FROM questions WHERE topic = ?")
        .bind::<Integer, _>(topic)
        .get_result(conn)?;
    Ok(row.count)
}

/// Select random questions for a topic, greedily filling up to `target_marks`.
/// Questions whose IDs appear in `exclude_ids` are skipped.
pub fn select_random_questions(
    conn: &mut Conn,
    topic: i32,
    target_marks: i16,
    exclude_ids: &[i32],
) -> Result<Vec<QuestionRow>> {
    let exclude_clause = if exclude_ids.is_empty() {
        String::new()
    } else {
        let ids: Vec<String> = exclude_ids.iter().map(|id| id.to_string()).collect();
        format!(" AND id NOT IN ({})", ids.join(","))
    };

    let sql = format!(
        "SELECT * FROM questions WHERE topic = ?{} ORDER BY RANDOM()",
        exclude_clause
    );

    let rows: Vec<QuestionRow> = sql_query(&sql).bind::<Integer, _>(topic).load(conn)?;

    // Greedy fill: pick questions in random order until cumulative marks >= target.
    // Allows the last question to overshoot the target by its own mark value so
    // that we never fail to fill a target just because no single small question
    // fits the remainder.
    let mut selected = Vec::new();
    let mut current_marks = 0i32;
    for row in rows {
        if current_marks >= target_marks as i32 {
            break;
        }
        current_marks += row.marks as i32;
        selected.push(row);
    }
    Ok(selected)
}

// =========================================================================
// Rubric Criteria
// =========================================================================

/// Bulk-insert rubric criteria for a question.
/// Each tuple is `(position, criterion, marks)`.
pub fn insert_rubric_criteria(
    conn: &mut Conn,
    question: i32,
    criteria: &[(i16, &str, i16)],
) -> Result<()> {
    for (position, criterion, marks) in criteria {
        sql_query(
            "INSERT INTO rubric_criteria (question, position, criterion, marks) \
             VALUES (?, ?, ?, ?)",
        )
        .bind::<Integer, _>(question)
        .bind::<SmallInt, _>(*position)
        .bind::<Text, _>(*criterion)
        .bind::<SmallInt, _>(*marks)
        .execute(conn)?;
    }
    Ok(())
}

/// Replace all rubric criteria for a question: deletes existing, then inserts new.
/// Each tuple is `(position, criterion, marks)`.
pub fn replace_rubric_criteria(
    conn: &mut Conn,
    question: i32,
    criteria: &[(i16, &str, i16)],
) -> Result<()> {
    sql_query("DELETE FROM rubric_criteria WHERE question = ?")
        .bind::<Integer, _>(question)
        .execute(conn)?;
    insert_rubric_criteria(conn, question, criteria)
}

/// Get all rubric criteria for a question, ordered by position.
pub fn get_rubric_criteria(conn: &mut Conn, question: i32) -> Result<Vec<RubricCriterionRow>> {
    let rows: Vec<RubricCriterionRow> =
        sql_query("SELECT * FROM rubric_criteria WHERE question = ? ORDER BY position")
            .bind::<Integer, _>(question)
            .load(conn)?;
    Ok(rows)
}

// =========================================================================
// Question Images
// =========================================================================

/// Insert a question image. Returns the new auto-increment ID.
pub fn insert_question_image(
    conn: &mut Conn,
    question: i32,
    position: i16,
    context: i16,
    key: &str,
    caption: Option<&str>,
) -> Result<i32> {
    sql_query(
        "INSERT INTO question_images (question, position, context, key, caption) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind::<Integer, _>(question)
    .bind::<SmallInt, _>(position)
    .bind::<SmallInt, _>(context)
    .bind::<Text, _>(key)
    .bind::<Nullable<Text>, _>(caption)
    .execute(conn)?;

    let row: LastId = sql_query("SELECT last_insert_rowid() AS id").get_result(conn)?;
    Ok(row.id)
}

/// Get all images for a question, ordered by position.
pub fn get_question_images(conn: &mut Conn, question: i32) -> Result<Vec<QuestionImageRow>> {
    let rows: Vec<QuestionImageRow> =
        sql_query("SELECT * FROM question_images WHERE question = ? ORDER BY position")
            .bind::<Integer, _>(question)
            .load(conn)?;
    Ok(rows)
}

/// Delete all images for a question.
pub fn delete_question_images(conn: &mut Conn, question: i32) -> Result<()> {
    sql_query("DELETE FROM question_images WHERE question = ?")
        .bind::<Integer, _>(question)
        .execute(conn)?;
    Ok(())
}

// =========================================================================
// Question Grades
// =========================================================================

/// Upsert a question grade (INSERT ... ON CONFLICT DO UPDATE).
pub fn upsert_question_grade(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    student: i32,
    question: i32,
    score: f32,
    feedback: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT INTO question_grades (school, exam, student, question, score, feedback, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(school, exam, student, question) DO UPDATE SET \
         score = excluded.score, \
         feedback = excluded.feedback, \
         updated = excluded.updated",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(question)
    .bind::<Float, _>(score)
    .bind::<Nullable<Text>, _>(feedback)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

/// Get question grades for a specific student, filtered to a set of question IDs.
pub fn get_question_grades_for_student(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    student: i32,
    question_ids: &[i32],
) -> Result<Vec<QuestionGradeRow>> {
    if question_ids.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<String> = question_ids.iter().map(|id| id.to_string()).collect();
    let sql = format!(
        "SELECT * FROM question_grades \
         WHERE school = ? AND exam = ? AND student = ? AND question IN ({})",
        ids.join(",")
    );

    let rows: Vec<QuestionGradeRow> = sql_query(&sql)
        .bind::<Text, _>(school)
        .bind::<Text, _>(exam)
        .bind::<Integer, _>(student)
        .load(conn)?;
    Ok(rows)
}

// =========================================================================
// Paper Questions
// =========================================================================

/// Bulk-insert paper questions. Each tuple is `(question_id, position)`.
pub fn insert_paper_questions(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
    questions: &[(i32, i16)],
) -> Result<()> {
    for (question_id, position) in questions {
        sql_query(
            "INSERT INTO paper_questions (school, exam, subject, paper, grade, stream, question, position) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(school)
        .bind::<Text, _>(exam)
        .bind::<Integer, _>(subject)
        .bind::<Nullable<SmallInt>, _>(paper)
        .bind::<SmallInt, _>(grade)
        .bind::<Nullable<SmallInt>, _>(stream)
        .bind::<Integer, _>(*question_id)
        .bind::<SmallInt, _>(*position)
        .execute(conn)?;
    }
    Ok(())
}

/// Get all paper questions, ordered by position.
/// Uses `COALESCE(col, -1) = COALESCE(?, -1)` for nullable paper/stream columns.
pub fn get_paper_questions(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
) -> Result<Vec<PaperQuestionRow>> {
    let rows: Vec<PaperQuestionRow> = sql_query(
        "SELECT * FROM paper_questions \
         WHERE school = ? AND exam = ? AND subject = ? \
         AND COALESCE(paper, -1) = COALESCE(?, -1) \
         AND grade = ? \
         AND COALESCE(stream, -1) = COALESCE(?, -1) \
         ORDER BY position",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .bind::<SmallInt, _>(grade)
    .bind::<Nullable<SmallInt>, _>(stream)
    .load(conn)?;
    Ok(rows)
}

/// Load all paper questions for a paper with full question data (rubric + images),
/// ordered by position. Returns proto-ready structs ready for the gRPC response.
pub fn get_full_paper_questions(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
) -> Result<
    Vec<(
        i16,
        Option<String>,
        QuestionRow,
        Vec<RubricCriterionRow>,
        Vec<QuestionImageRow>,
    )>,
> {
    let pqs = get_paper_questions(conn, school, exam, subject, paper, grade, stream)?;
    let mut result = Vec::with_capacity(pqs.len());
    for pq in &pqs {
        let row = get_question(conn, pq.question)?;
        let rubric = get_rubric_criteria(conn, pq.question)?;
        let images = get_question_images(conn, pq.question)?;
        result.push((pq.position, pq.section.clone(), row, rubric, images));
    }
    Ok(result)
}

/// Update the section label for a specific paper question by position.
/// Returns true if a row was updated, false if not found.
pub fn set_paper_question_section(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
    position: i16,
    section: Option<&str>,
) -> Result<bool> {
    let affected = sql_query(
        "UPDATE paper_questions SET section = ? \
         WHERE school = ? AND exam = ? AND subject = ? \
         AND COALESCE(paper, -1) = COALESCE(?, -1) \
         AND grade = ? \
         AND COALESCE(stream, -1) = COALESCE(?, -1) \
         AND position = ?",
    )
    .bind::<Nullable<Text>, _>(section)
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .bind::<SmallInt, _>(grade)
    .bind::<Nullable<SmallInt>, _>(stream)
    .bind::<SmallInt, _>(position)
    .execute(conn)?;
    Ok(affected > 0)
}

/// Delete all paper questions matching the given paper identity.
pub fn delete_paper_questions(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
) -> Result<()> {
    sql_query(
        "DELETE FROM paper_questions \
         WHERE school = ? AND exam = ? AND subject = ? \
         AND COALESCE(paper, -1) = COALESCE(?, -1) \
         AND grade = ? \
         AND COALESCE(stream, -1) = COALESCE(?, -1)",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .bind::<SmallInt, _>(grade)
    .bind::<Nullable<SmallInt>, _>(stream)
    .execute(conn)?;
    Ok(())
}

/// Replace the question at a specific position in a paper.
pub fn replace_paper_question_at_position(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
    position: i16,
    new_question_id: i32,
) -> Result<()> {
    sql_query(
        "UPDATE paper_questions SET question = ? \
         WHERE school = ? AND exam = ? AND subject = ? \
         AND COALESCE(paper, -1) = COALESCE(?, -1) \
         AND grade = ? \
         AND COALESCE(stream, -1) = COALESCE(?, -1) \
         AND position = ?",
    )
    .bind::<Integer, _>(new_question_id)
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .bind::<SmallInt, _>(grade)
    .bind::<Nullable<SmallInt>, _>(stream)
    .bind::<SmallInt, _>(position)
    .execute(conn)?;
    Ok(())
}

// =========================================================================
// Marking Queue
// =========================================================================

/// Upsert a marking queue entry. Returns the row ID.
///
/// Because the unique index includes nullable columns (paper, stream),
/// we use a manual SELECT + INSERT/UPDATE instead of ON CONFLICT.
pub fn upsert_marking_queue(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
    phase: i16,
    total_students: i32,
) -> Result<i32> {
    let now = chrono::Utc::now().timestamp();

    // Check for existing row using COALESCE sentinel for nullable columns
    let existing: Option<MarkingQueueRow> = sql_query(
        "SELECT * FROM marking_queue \
         WHERE school = ? AND exam = ? AND subject = ? \
         AND COALESCE(paper, -1) = COALESCE(?, -1) \
         AND grade = ? \
         AND COALESCE(stream, -1) = COALESCE(?, -1)",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .bind::<SmallInt, _>(grade)
    .bind::<Nullable<SmallInt>, _>(stream)
    .get_result(conn)
    .optional()?;

    if let Some(row) = existing {
        sql_query(
            "UPDATE marking_queue SET phase = ?, total_students = ?, updated = ? \
             WHERE id = ?",
        )
        .bind::<SmallInt, _>(phase)
        .bind::<Integer, _>(total_students)
        .bind::<BigInt, _>(now)
        .bind::<Integer, _>(row.id)
        .execute(conn)?;
        Ok(row.id)
    } else {
        sql_query(
            "INSERT INTO marking_queue \
             (school, exam, subject, paper, grade, stream, phase, total_students, created, updated) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind::<Text, _>(school)
        .bind::<Text, _>(exam)
        .bind::<Integer, _>(subject)
        .bind::<Nullable<SmallInt>, _>(paper)
        .bind::<SmallInt, _>(grade)
        .bind::<Nullable<SmallInt>, _>(stream)
        .bind::<SmallInt, _>(phase)
        .bind::<Integer, _>(total_students)
        .bind::<BigInt, _>(now)
        .bind::<BigInt, _>(now)
        .execute(conn)?;

        let last: LastId = sql_query("SELECT last_insert_rowid() AS id").get_result(conn)?;
        Ok(last.id)
    }
}

/// Update the marking status of a queue entry.
pub fn update_marking_status(
    conn: &mut Conn,
    id: i32,
    phase: i16,
    progress: &str,
    marked_students: i32,
    error: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE marking_queue SET \
         phase = ?, progress = ?, marked_students = ?, error = ?, updated = ? \
         WHERE id = ?",
    )
    .bind::<SmallInt, _>(phase)
    .bind::<Text, _>(progress)
    .bind::<Integer, _>(marked_students)
    .bind::<Nullable<Text>, _>(error)
    .bind::<BigInt, _>(now)
    .bind::<Integer, _>(id)
    .execute(conn)?;
    Ok(())
}

/// Get the marking status for a specific paper identity. Returns `None` if not found.
pub fn get_marking_status(
    conn: &mut Conn,
    school: &str,
    exam: &str,
    subject: i32,
    paper: Option<i16>,
    grade: i16,
    stream: Option<i16>,
) -> Result<Option<MarkingQueueRow>> {
    let row: Option<MarkingQueueRow> = sql_query(
        "SELECT * FROM marking_queue \
         WHERE school = ? AND exam = ? AND subject = ? \
         AND COALESCE(paper, -1) = COALESCE(?, -1) \
         AND grade = ? \
         AND COALESCE(stream, -1) = COALESCE(?, -1)",
    )
    .bind::<Text, _>(school)
    .bind::<Text, _>(exam)
    .bind::<Integer, _>(subject)
    .bind::<Nullable<SmallInt>, _>(paper)
    .bind::<SmallInt, _>(grade)
    .bind::<Nullable<SmallInt>, _>(stream)
    .get_result(conn)
    .optional()?;
    Ok(row)
}
