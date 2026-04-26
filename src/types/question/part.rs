use crate::db::schema::{part_rubric_criteria, question_parts, rubric_criteria};
use crate::types::question::enums::{AnswerSpaceType, BodyFormat};
use diesel::{Insertable, Queryable, QueryableByName, Selectable};

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = question_parts)]
pub struct QuestionPart {
    pub question: i32,
    pub position: i16,
    pub label: String,
    pub body: String,
    pub body_format: BodyFormat,
    pub marks: i16,
    pub max_marks: Option<i16>,
    pub answer_space_type: AnswerSpaceType,
    pub answer_lines: Option<i16>,
    pub answer_box_height_mm: Option<i16>,
    pub example_answer: Option<String>,
    pub stimulus: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = rubric_criteria)]
pub struct RubricCriterion {
    pub question: i32,
    pub position: i16,
    pub criterion: String,
    pub marks: i16,
    pub max_marks: Option<i16>,
    pub required: bool,
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = part_rubric_criteria)]
pub struct PartRubricCriterion {
    pub question: i32,
    pub part: i16,
    pub position: i16,
    pub criterion: String,
    pub marks: i16,
    pub max_marks: Option<i16>,
    pub required: bool,
}
