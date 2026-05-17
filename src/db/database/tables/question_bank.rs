#![allow(dead_code, unused_imports)]

use diesel::prelude::*;
use diesel::result::OptionalExtension;
use diesel::sql_query;
use diesel::sql_types::{BigInt, Float, Integer, Nullable, SmallInt, Text};
use diesel::sqlite::SqliteConnection;

use crate::db::database::tables::rows::{
    MarkingQueueRow, PaperQuestionRow, QuestionGradeRow, QuestionImageRow, QuestionRow,
    RubricCriterionRow,
};
use crate::db::schema::{
    answer_pages, marking_queue, paper_questions, part_rubric_criteria, question_grades,
    question_images, question_parts, questions, rubric_criteria, scheme_pages,
};
use crate::types::error::{Error, Result};
use crate::types::question::{
    AnswerSpaceType, BodyFormat, CognitiveLevel, PartRubricCriterion, Question, QuestionPart,
    QuestionType, QuestionUpdate, RubricCriterion,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(diesel::QueryableByName)]
struct LastId {
    #[diesel(sql_type = Integer)]
    pub id: i32,
}

#[derive(diesel::QueryableByName)]
struct PageKeyRow {
    #[diesel(sql_type = SmallInt)]
    pub page: i16,
    #[diesel(sql_type = Text)]
    pub key: String,
}

#[derive(diesel::QueryableByName)]
struct StudentAdmRow {
    #[diesel(sql_type = Integer)]
    pub student: i32,
}

// =========================================================================
// Questions
// =========================================================================

/// Insert a new question. Returns the auto-incremented id.
pub fn insert_question(
    conn: &mut SqliteConnection,
    topic: i32,
    body: &str,
    body_format: BodyFormat,
    stimulus: Option<&str>,
    type_: QuestionType,
    difficulty: i16,
    cognitive_level: CognitiveLevel,
    marks: i16,
    max_marks: Option<i16>,
    answer_space_type: AnswerSpaceType,
    answer_lines: Option<i16>,
    answer_box_height_mm: Option<i16>,
    example_answer: Option<&str>,
    created_by: &str,
) -> Result<i32> {
    let now = chrono::Utc::now().timestamp();
    let q = Question {
        id: None,
        topic,
        body: body.to_owned(),
        body_format,
        stimulus: stimulus.map(|s| s.to_owned()),
        type_,
        difficulty,
        cognitive_level,
        marks,
        max_marks,
        answer_space_type,
        answer_lines,
        answer_box_height_mm,
        example_answer: example_answer.map(|s| s.to_owned()),
        created: now,
        updated: now,
        created_by: created_by.to_owned(),
    };
    diesel::insert_into(questions::table)
        .values(&q)
        .execute(conn)?;
    let row: LastId = diesel::sql_query("SELECT last_insert_rowid() AS id").get_result(conn)?;
    Ok(row.id)
}

/// Insert or find existing question (deduplicates on topic+body).
/// Returns (id, was_created).
pub fn find_or_insert_question(
    conn: &mut SqliteConnection,
    topic: i32,
    body: &str,
    body_format: BodyFormat,
    stimulus: Option<&str>,
    type_: QuestionType,
    difficulty: i16,
    cognitive_level: CognitiveLevel,
    marks: i16,
    max_marks: Option<i16>,
    answer_space_type: AnswerSpaceType,
    answer_lines: Option<i16>,
    answer_box_height_mm: Option<i16>,
    example_answer: Option<&str>,
    created_by: &str,
) -> Result<(i32, bool)> {
    let now = chrono::Utc::now().timestamp();
    let q = Question {
        id: None,
        topic,
        body: body.to_owned(),
        body_format,
        stimulus: stimulus.map(|s| s.to_owned()),
        type_,
        difficulty,
        cognitive_level,
        marks,
        max_marks,
        answer_space_type,
        answer_lines,
        answer_box_height_mm,
        example_answer: example_answer.map(|s| s.to_owned()),
        created: now,
        updated: now,
        created_by: created_by.to_owned(),
    };
    let affected = diesel::insert_into(questions::table)
        .values(&q)
        .on_conflict_do_nothing()
        .execute(conn)?;

    // Query by (topic, body) to get the ID whether we just inserted or it already existed.
    let existing: Option<LastId> =
        sql_query("SELECT id FROM questions WHERE topic = ? AND body = ? LIMIT 1")
            .bind::<Integer, _>(topic)
            .bind::<Text, _>(body)
            .get_result(conn)
            .optional()?;

    match existing {
        Some(row) => Ok((row.id, affected > 0)),
        None => Err(Error::Internal(format!(
            "question lookup failed after insert: topic={topic} body={body}"
        ))),
    }
}

/// Update a question. Only fields set to `Some` in `update` are changed.
/// Always bumps the `updated` timestamp.
pub fn update_question(
    conn: &mut SqliteConnection,
    id: i32,
    mut update: QuestionUpdate,
) -> Result<()> {
    if update.updated.is_none() {
        update.updated = Some(chrono::Utc::now().timestamp());
    }
    diesel::update(questions::table.filter(questions::id.eq(Some(id))))
        .set(&update)
        .execute(conn)?;
    Ok(())
}

/// Delete a question by ID. Returns true if a row was actually deleted.
pub fn delete_question(conn: &mut SqliteConnection, id: i32) -> Result<bool> {
    let affected = sql_query("DELETE FROM questions WHERE id = ?")
        .bind::<Integer, _>(id)
        .execute(conn)?;
    Ok(affected > 0)
}

/// Get a single question by ID.
pub fn get_question(conn: &mut SqliteConnection, id: i32) -> Result<Option<Question>> {
    let q: Option<Question> = sql_query("SELECT * FROM questions WHERE id = ?")
        .bind::<Integer, _>(id)
        .get_result(conn)
        .optional()?;
    Ok(q)
}

/// List questions for a topic with pagination.
pub fn count_questions(conn: &mut SqliteConnection, topic_id: i32) -> Result<i32> {
    use crate::db::schema::questions::dsl;
    let count: i64 = dsl::questions
        .filter(dsl::topic.eq(topic_id))
        .count()
        .get_result(conn)?;
    Ok(count as i32)
}

pub fn list_questions(
    conn: &mut SqliteConnection,
    topic_id: i32,
    page: i64,
    page_size: i64,
) -> Result<Vec<Question>> {
    let offset = page * page_size;
    let rows: Vec<Question> =
        sql_query("SELECT * FROM questions WHERE topic = ? ORDER BY id LIMIT ? OFFSET ?")
            .bind::<Integer, _>(topic_id)
            .bind::<BigInt, _>(page_size)
            .bind::<BigInt, _>(offset)
            .load(conn)?;
    Ok(rows)
}

// =========================================================================
// Rubric Criteria
// =========================================================================

/// Bulk-insert rubric criteria for a question.
/// Each tuple is `(position, criterion, marks, max_marks, required)`.
pub fn insert_rubric_criteria(
    conn: &mut SqliteConnection,
    question_id: i32,
    criteria: &[(i16, String, i16, Option<i16>, bool)],
) -> Result<()> {
    let rows: Vec<RubricCriterion> = criteria
        .iter()
        .map(|(pos, crit, marks, max_marks, required)| RubricCriterion {
            question: question_id,
            position: *pos,
            criterion: crit.clone(),
            marks: *marks,
            max_marks: *max_marks,
            required: *required,
        })
        .collect();
    if !rows.is_empty() {
        diesel::insert_into(rubric_criteria::table)
            .values(&rows)
            .execute(conn)?;
    }
    Ok(())
}

/// Replace all rubric criteria for a question: delete existing, then insert new.
pub fn replace_rubric_criteria(
    conn: &mut SqliteConnection,
    question_id: i32,
    criteria: &[(i16, String, i16, Option<i16>, bool)],
) -> Result<()> {
    diesel::delete(rubric_criteria::table.filter(rubric_criteria::question.eq(question_id)))
        .execute(conn)?;
    insert_rubric_criteria(conn, question_id, criteria)
}

/// Get all rubric criteria for a question, ordered by position.
pub fn get_rubric_criteria(
    conn: &mut SqliteConnection,
    question_id: i32,
) -> Result<Vec<RubricCriterion>> {
    let rows: Vec<RubricCriterion> = rubric_criteria::table
        .filter(rubric_criteria::question.eq(question_id))
        .order(rubric_criteria::position)
        .load(conn)?;
    Ok(rows)
}

// =========================================================================
// Question Parts
// =========================================================================

/// Bulk-insert question parts. The `question` field on each part is overridden
/// with `question_id` to ensure consistency.
pub fn insert_question_parts(
    conn: &mut SqliteConnection,
    question_id: i32,
    parts: &[QuestionPart],
) -> Result<()> {
    if parts.is_empty() {
        return Ok(());
    }
    let rows: Vec<QuestionPart> = parts
        .iter()
        .map(|p| QuestionPart {
            question: question_id,
            ..p.clone()
        })
        .collect();
    diesel::insert_into(question_parts::table)
        .values(&rows)
        .execute(conn)?;
    Ok(())
}

/// Get all parts for a question, ordered by position.
pub fn get_question_parts(
    conn: &mut SqliteConnection,
    question_id: i32,
) -> Result<Vec<QuestionPart>> {
    let rows: Vec<QuestionPart> = question_parts::table
        .filter(question_parts::question.eq(question_id))
        .order(question_parts::position)
        .load(conn)?;
    Ok(rows)
}

/// Get rubric criteria for a specific question part.
pub fn get_part_rubric_criteria(
    conn: &mut SqliteConnection,
    question_id: i32,
    part_position: i16,
) -> Result<Vec<PartRubricCriterion>> {
    let rows: Vec<PartRubricCriterion> = part_rubric_criteria::table
        .filter(part_rubric_criteria::question.eq(question_id))
        .filter(part_rubric_criteria::part.eq(part_position))
        .order(part_rubric_criteria::position)
        .load(conn)?;
    Ok(rows)
}

/// Bulk-insert rubric criteria for a specific question part.
pub fn insert_part_rubric_criteria(
    conn: &mut SqliteConnection,
    question_id: i32,
    part_position: i16,
    criteria: &[(i16, String, i16, Option<i16>, bool)],
) -> Result<()> {
    if criteria.is_empty() {
        return Ok(());
    }
    let rows: Vec<PartRubricCriterion> = criteria
        .iter()
        .map(|(pos, crit, marks, max_marks, required)| PartRubricCriterion {
            question: question_id,
            part: part_position,
            position: *pos,
            criterion: crit.clone(),
            marks: *marks,
            max_marks: *max_marks,
            required: *required,
        })
        .collect();
    diesel::insert_into(part_rubric_criteria::table)
        .values(&rows)
        .execute(conn)?;
    Ok(())
}

// =========================================================================
// Paper Questions
// =========================================================================

/// Bulk-insert paper questions. Each tuple is `(question_id, position)`.
pub fn insert_paper_questions(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: Option<i32>,
    questions: &[(i32, i16)],
) -> Result<()> {
    for (question_id, position) in questions {
        sql_query(
            "INSERT INTO paper_questions (paper, student, question, position) \
             VALUES (?, ?, ?, ?)",
        )
        .bind::<Text, _>(paper_id)
        .bind::<Nullable<Integer>, _>(student)
        .bind::<Integer, _>(*question_id)
        .bind::<SmallInt, _>(*position)
        .execute(conn)?;
    }
    Ok(())
}

/// Get paper questions ordered by position.
/// `student = None` → WHERE student IS NULL (template/class paper).
pub fn get_paper_questions(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: Option<i32>,
) -> Result<Vec<PaperQuestionRow>> {
    let rows: Vec<PaperQuestionRow> = if let Some(s) = student {
        sql_query(
            "SELECT * FROM paper_questions \
             WHERE paper = ? AND student = ? \
             ORDER BY position",
        )
        .bind::<Text, _>(paper_id)
        .bind::<Integer, _>(s)
        .load(conn)?
    } else {
        sql_query(
            "SELECT * FROM paper_questions \
             WHERE paper = ? AND student IS NULL \
             ORDER BY position",
        )
        .bind::<Text, _>(paper_id)
        .load(conn)?
    };
    Ok(rows)
}

/// Delete paper questions for the given (paper, student) combination.
/// Returns the number of rows deleted.
pub fn delete_paper_questions(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: Option<i32>,
) -> Result<usize> {
    let count = if let Some(s) = student {
        sql_query("DELETE FROM paper_questions WHERE paper = ? AND student = ?")
            .bind::<Text, _>(paper_id)
            .bind::<Integer, _>(s)
            .execute(conn)?
    } else {
        sql_query("DELETE FROM paper_questions WHERE paper = ? AND student IS NULL")
            .bind::<Text, _>(paper_id)
            .execute(conn)?
    };
    Ok(count)
}

// =========================================================================
// Question Grades
// =========================================================================

/// Upsert a question grade (INSERT OR REPLACE).
pub fn upsert_question_grade(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: i32,
    question_id: i32,
    score: f32,
    feedback: Option<&str>,
    awarded_criteria: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT OR REPLACE INTO question_grades \
         (paper, student, question, score, feedback, awarded_criteria, created, updated) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(paper_id)
    .bind::<Integer, _>(student)
    .bind::<Integer, _>(question_id)
    .bind::<Float, _>(score)
    .bind::<Nullable<Text>, _>(feedback)
    .bind::<Nullable<Text>, _>(awarded_criteria)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

/// Get all question grades for a specific student on a paper.
pub fn get_question_grades_for_student(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: i32,
) -> Result<Vec<QuestionGradeRow>> {
    let rows: Vec<QuestionGradeRow> =
        sql_query("SELECT * FROM question_grades WHERE paper = ? AND student = ?")
            .bind::<Text, _>(paper_id)
            .bind::<Integer, _>(student)
            .load(conn)?;
    Ok(rows)
}

/// Get all question grades for a paper across all students.
pub fn get_question_grades_for_paper(
    conn: &mut SqliteConnection,
    paper_id: &str,
) -> Result<Vec<QuestionGradeRow>> {
    let rows: Vec<QuestionGradeRow> = sql_query("SELECT * FROM question_grades WHERE paper = ?")
        .bind::<Text, _>(paper_id)
        .load(conn)?;
    Ok(rows)
}

// =========================================================================
// Marking Queue
// =========================================================================

/// Ensure a marking queue entry exists for the paper (INSERT OR IGNORE).
pub fn upsert_marking_queue(conn: &mut SqliteConnection, paper_id: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT OR IGNORE INTO marking_queue \
         (paper, phase, progress, total_students, marked_students, created, updated) \
         VALUES (?, 0, '', 0, 0, ?, ?)",
    )
    .bind::<Text, _>(paper_id)
    .bind::<BigInt, _>(now)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

/// Update the marking progress for a paper.
pub fn update_marking_status(
    conn: &mut SqliteConnection,
    paper_id: &str,
    phase: i16,
    progress: &str,
    error: Option<&str>,
    total: i32,
    marked: i32,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "UPDATE marking_queue \
         SET phase = ?, progress = ?, error = ?, \
             total_students = ?, marked_students = ?, updated = ? \
         WHERE paper = ?",
    )
    .bind::<SmallInt, _>(phase)
    .bind::<Text, _>(progress)
    .bind::<Nullable<Text>, _>(error)
    .bind::<Integer, _>(total)
    .bind::<Integer, _>(marked)
    .bind::<BigInt, _>(now)
    .bind::<Text, _>(paper_id)
    .execute(conn)?;
    Ok(())
}

/// Get the marking status row for a paper.
pub fn get_marking_status(
    conn: &mut SqliteConnection,
    paper_id: &str,
) -> Result<Option<MarkingQueueRow>> {
    let row: Option<MarkingQueueRow> = sql_query("SELECT * FROM marking_queue WHERE paper = ?")
        .bind::<Text, _>(paper_id)
        .get_result(conn)
        .optional()?;
    Ok(row)
}

// =========================================================================
// Paper Generation
// =========================================================================

/// Select random questions for paper generation.
///
/// - `exclude_ids`: question IDs to skip unconditionally.
/// - `exclude_recent_student`: `(student_adm, last_n)` — also exclude questions
///   seen by this student in their last `last_n` paper-question rows ordered by
///   paper date.
pub fn select_questions_for_paper(
    conn: &mut SqliteConnection,
    topic_id: i32,
    exclude_ids: &[i32],
    exclude_recent_student: Option<(i32, usize)>,
) -> Result<Vec<QuestionRow>> {
    let exclude_clause = if exclude_ids.is_empty() {
        String::new()
    } else {
        let ids: Vec<String> = exclude_ids.iter().map(|id| id.to_string()).collect();
        format!(" AND id NOT IN ({})", ids.join(","))
    };

    let student_clause = match exclude_recent_student {
        Some((student_adm, last_n)) => format!(
            " AND id NOT IN (\
                SELECT pq.question FROM paper_questions pq \
                JOIN papers p ON p.id = pq.paper \
                WHERE pq.student = {} \
                ORDER BY p.date DESC LIMIT {}\
            )",
            student_adm, last_n
        ),
        None => String::new(),
    };

    let sql = format!(
        "SELECT * FROM questions WHERE topic = ?{}{} \
         ORDER BY RANDOM() LIMIT 30",
        exclude_clause, student_clause
    );

    let rows: Vec<QuestionRow> = sql_query(&sql)
        .bind::<Integer, _>(topic_id)
        .load(conn)?;
    Ok(rows)
}

/// Select a subset of candidate questions whose marks sum as close as possible
/// to `target_marks`. Returns (selected_question_ids, actual_sum).
///
/// Uses recursive backtracking to find an exact match. When no exact match
/// exists, probes nearby targets (±1, ±2, …) up to a small tolerance, then
/// falls back to a greedy selection.
pub fn select_questions_for_marks(
    candidates: &[QuestionRow],
    target_marks: i16,
) -> (Vec<i32>, i16) {
    let items: Vec<(i32, i16)> = candidates
        .iter()
        .filter_map(|q| q.id.map(|id| (id, q.marks)))
        .collect();

    if items.is_empty() {
        return (Vec::new(), 0);
    }

    // Try exact match first, then proximity fallback.
    for delta in 0i16..=5i16 {
        for &sign in &[1i16, -1i16] {
            let t = target_marks + delta * sign;
            if t <= 0 {
                continue;
            }
            if let Some(selected) = subset_sum_exact(&items, t) {
                return (selected, t);
            }
        }
        // Also try the positive delta only (handles delta=0 and asymmetric).
        if delta > 0 {
            let t = target_marks + delta;
            if let Some(selected) = subset_sum_exact(&items, t) {
                return (selected, t);
            }
            let t2 = target_marks - delta;
            if t2 > 0 {
                if let Some(selected) = subset_sum_exact(&items, t2) {
                    return (selected, t2);
                }
            }
        }
    }

    // Fallback: greedy selection (closest to target without going under,
    // or the closest sum overall).
    let (ids, sum) = subset_sum_greedy(&items, target_marks);
    (ids, sum)
}

/// Recursive depth-first search for an exact subset sum.
/// Sorts items descending to prune early. Returns Some(ids) if exact sum found.
fn subset_sum_exact(items: &[(i32, i16)], target: i16) -> Option<Vec<i32>> {
    // Sort descending by marks for better pruning.
    let mut sorted: Vec<(i32, i16)> = items.to_vec();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let suffixes: Vec<i32> = {
        let mut s = vec![0i32; sorted.len() + 1];
        for i in (0..sorted.len()).rev() {
            s[i] = s[i + 1] + sorted[i].1 as i32;
        }
        s
    };

    let mut best: Option<Vec<i32>> = None;

    fn dfs(
        sorted: &[(i32, i16)],
        suffixes: &[i32],
        idx: usize,
        current_sum: i16,
        target: i16,
        selected: &mut Vec<i32>,
        best: &mut Option<Vec<i32>>,
    ) {
        if current_sum == target {
            *best = Some(selected.clone());
            return;
        }
        if idx >= sorted.len() || current_sum > target {
            return;
        }
        // Prune: can't reach target even with all remaining items.
        if current_sum as i32 + suffixes[idx] < target as i32 {
            return;
        }
        // Already found a solution.
        if best.is_some() {
            return;
        }

        // Include item[idx]
        let (id, marks) = sorted[idx];
        if current_sum + marks <= target {
            selected.push(id);
            dfs(
                sorted,
                suffixes,
                idx + 1,
                current_sum + marks,
                target,
                selected,
                best,
            );
            selected.pop();
        }

        // Skip item[idx]
        dfs(
            sorted,
            suffixes,
            idx + 1,
            current_sum,
            target,
            selected,
            best,
        );
    }

    let mut selected = Vec::new();
    dfs(&sorted, &suffixes, 0, 0, target, &mut selected, &mut best);
    best
}

/// Greedy fallback: picks items in mark-descending order until reaching or
/// exceeding target, then tries to trim the last item if it overshoots.
fn subset_sum_greedy(items: &[(i32, i16)], target: i16) -> (Vec<i32>, i16) {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut selected = Vec::new();
    let mut sum: i16 = 0;

    for &(id, marks) in &sorted {
        if sum >= target {
            break;
        }
        selected.push(id);
        sum += marks;
    }

    // Try to remove the last item if we overshot and a closer sum exists
    // without it.
    if sum > target && selected.len() > 1 {
        let last_marks = sorted
            .iter()
            .find(|(id, _)| *id == *selected.last().unwrap())
            .map(|(_, m)| m)
            .unwrap_or(&0);
        let without_last = sum - last_marks;
        if (without_last - target).abs() < (sum - target).abs() {
            selected.pop();
            sum = without_last;
        }
    }

    (selected, sum)
}

// =========================================================================
// Question Images
// =========================================================================

/// Insert a question image.
pub fn insert_question_image(
    conn: &mut SqliteConnection,
    question_id: i32,
    position: i16,
    context: i16,
    key: &str,
    caption: Option<&str>,
) -> Result<()> {
    sql_query(
        "INSERT INTO question_images (question, position, context, key, caption) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind::<Integer, _>(question_id)
    .bind::<SmallInt, _>(position)
    .bind::<SmallInt, _>(context)
    .bind::<Text, _>(key)
    .bind::<Nullable<Text>, _>(caption)
    .execute(conn)?;
    Ok(())
}

/// Get all images for a question, ordered by position.
pub fn get_question_images(
    conn: &mut SqliteConnection,
    question_id: i32,
) -> Result<Vec<QuestionImageRow>> {
    let rows: Vec<QuestionImageRow> =
        sql_query("SELECT * FROM question_images WHERE question = ? ORDER BY position")
            .bind::<Integer, _>(question_id)
            .load(conn)?;
    Ok(rows)
}

/// Delete all images for a question. Returns the number of deleted rows.
pub fn delete_question_images(conn: &mut SqliteConnection, question_id: i32) -> Result<usize> {
    let count = sql_query("DELETE FROM question_images WHERE question = ?")
        .bind::<Integer, _>(question_id)
        .execute(conn)?;
    Ok(count)
}

// =========================================================================
// Scheme / Answer Pages
// =========================================================================

/// Insert a marking scheme page (INSERT OR IGNORE for idempotency).
pub fn insert_scheme_page(
    conn: &mut SqliteConnection,
    paper_id: &str,
    page: i16,
    key: &str,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query("INSERT OR IGNORE INTO scheme_pages (paper, page, key, created) VALUES (?, ?, ?, ?)")
        .bind::<Text, _>(paper_id)
        .bind::<SmallInt, _>(page)
        .bind::<Text, _>(key)
        .bind::<BigInt, _>(now)
        .execute(conn)?;
    Ok(())
}

/// Delete all answer pages for a student on a paper.
pub fn delete_answer_pages_for_student(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: i32,
) -> Result<()> {
    sql_query("DELETE FROM answer_pages WHERE paper = ? AND student = ?")
        .bind::<Text, _>(paper_id)
        .bind::<Integer, _>(student)
        .execute(conn)?;
    Ok(())
}

/// Insert a student answer page (INSERT OR REPLACE so re-uploads update the S3 key).
pub fn insert_answer_page(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: i32,
    page: i16,
    key: &str,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    sql_query(
        "INSERT OR REPLACE INTO answer_pages (paper, student, page, key, created) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind::<Text, _>(paper_id)
    .bind::<Integer, _>(student)
    .bind::<SmallInt, _>(page)
    .bind::<Text, _>(key)
    .bind::<BigInt, _>(now)
    .execute(conn)?;
    Ok(())
}

/// Get all marking scheme pages for a paper, ordered by page number.
/// Returns `(page, r2_key)` pairs.
pub fn get_scheme_pages(conn: &mut SqliteConnection, paper_id: &str) -> Result<Vec<(i16, String)>> {
    let rows: Vec<PageKeyRow> =
        sql_query("SELECT page, key FROM scheme_pages WHERE paper = ? ORDER BY page")
            .bind::<Text, _>(paper_id)
            .load(conn)?;
    Ok(rows.into_iter().map(|r| (r.page, r.key)).collect())
}

/// Get all answer pages for a specific student on a paper, ordered by page number.
/// Returns `(page, r2_key)` pairs.
pub fn get_answer_pages(
    conn: &mut SqliteConnection,
    paper_id: &str,
    student: i32,
) -> Result<Vec<(i16, String)>> {
    let rows: Vec<PageKeyRow> = sql_query(
        "SELECT page, key FROM answer_pages WHERE paper = ? AND student = ? ORDER BY page",
    )
    .bind::<Text, _>(paper_id)
    .bind::<Integer, _>(student)
    .load(conn)?;
    Ok(rows.into_iter().map(|r| (r.page, r.key)).collect())
}

/// Get all distinct student admission numbers that have paper_questions rows for a paper.
pub fn get_paper_student_adms(conn: &mut SqliteConnection, paper_id: &str) -> Result<Vec<i32>> {
    let rows: Vec<StudentAdmRow> = sql_query(
        "SELECT DISTINCT student FROM paper_questions \
         WHERE paper = ? AND student IS NOT NULL",
    )
    .bind::<Text, _>(paper_id)
    .load(conn)?;
    Ok(rows.into_iter().map(|r| r.student).collect())
}
