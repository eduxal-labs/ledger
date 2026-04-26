use super::enums::*;
use crate::db::schema::questions;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};
use serde::{Deserialize, Serialize};

/// Stored as JSON TEXT in the stimulus column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StimulusImage {
    pub filename: String,
    pub caption: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stimulus {
    #[serde(rename = "type")]
    pub type_: StimulusType,
    pub body: String,
    pub body_format: u8,
    pub caption: String,
    pub image: Option<StimulusImage>,
}

/// Stored as JSON TEXT in the example_answer column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleAnswerImage {
    pub filename: String,
    pub caption: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleAnswer {
    pub format: ExampleAnswerFormat,
    pub content: Option<String>,
    pub image: Option<ExampleAnswerImage>,
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Insertable)]
#[diesel(table_name = questions)]
pub struct Question {
    pub id: Option<i32>,
    pub topic: i32,
    pub body: String,
    pub body_format: BodyFormat,
    pub stimulus: Option<String>,
    pub type_: QuestionType,
    pub difficulty: i16,
    pub cognitive_level: CognitiveLevel,
    pub marks: i16,
    pub max_marks: Option<i16>,
    pub answer_space_type: AnswerSpaceType,
    pub answer_lines: Option<i16>,
    pub answer_box_height_mm: Option<i16>,
    pub example_answer: Option<String>,
    pub created: i64,
    pub updated: i64,
    pub created_by: String,
}

/// Changeset for updating a question.
#[derive(Debug, Default, diesel::AsChangeset)]
#[diesel(table_name = questions)]
pub struct QuestionUpdate {
    pub topic: Option<i32>,
    pub body: Option<String>,
    pub body_format: Option<BodyFormat>,
    pub stimulus: Option<Option<String>>,
    pub type_: Option<QuestionType>,
    pub difficulty: Option<i16>,
    pub cognitive_level: Option<CognitiveLevel>,
    pub marks: Option<i16>,
    pub max_marks: Option<Option<i16>>,
    pub answer_space_type: Option<AnswerSpaceType>,
    pub answer_lines: Option<Option<i16>>,
    pub answer_box_height_mm: Option<Option<i16>>,
    pub example_answer: Option<Option<String>>,
    pub updated: Option<i64>,
}
