use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::SmallInt;
use diesel::sqlite::Sqlite;

// ── BodyFormat ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum BodyFormat {
    #[default]
    Plain = 0,
    Tiptap = 1,
}

impl TryFrom<i16> for BodyFormat {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Plain),
            1 => Ok(Self::Tiptap),
            _ => Err(crate::types::error::Error::InvalidQuestionText),
        }
    }
}
impl From<BodyFormat> for i16 {
    fn from(v: BodyFormat) -> i16 {
        v as i16
    }
}
impl ToSql<SmallInt, Sqlite> for BodyFormat {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}
impl FromSql<SmallInt, Sqlite> for BodyFormat {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── QuestionType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum QuestionType {
    #[default]
    Definition = 0,
    Explanation = 1,
    Calculation = 2,
    Structured = 3,
    Experiment = 4,
    DataResponse = 5,
    Diagram = 6,
}

impl TryFrom<i16> for QuestionType {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Definition),
            1 => Ok(Self::Explanation),
            2 => Ok(Self::Calculation),
            3 => Ok(Self::Structured),
            4 => Ok(Self::Experiment),
            5 => Ok(Self::DataResponse),
            6 => Ok(Self::Diagram),
            _ => Err(crate::types::error::Error::InvalidQuestionText),
        }
    }
}
impl From<QuestionType> for i16 {
    fn from(v: QuestionType) -> i16 {
        v as i16
    }
}
impl ToSql<SmallInt, Sqlite> for QuestionType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}
impl FromSql<SmallInt, Sqlite> for QuestionType {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── CognitiveLevel ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum CognitiveLevel {
    #[default]
    Recall = 0,
    Comprehension = 1,
    Application = 2,
    Analysis = 3,
}

impl TryFrom<i16> for CognitiveLevel {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Recall),
            1 => Ok(Self::Comprehension),
            2 => Ok(Self::Application),
            3 => Ok(Self::Analysis),
            _ => Err(crate::types::error::Error::InvalidQuestionText),
        }
    }
}
impl From<CognitiveLevel> for i16 {
    fn from(v: CognitiveLevel) -> i16 {
        v as i16
    }
}
impl ToSql<SmallInt, Sqlite> for CognitiveLevel {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}
impl FromSql<SmallInt, Sqlite> for CognitiveLevel {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── AnswerSpaceType ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, FromSqlRow, AsExpression)]
#[diesel(sql_type = SmallInt)]
pub enum AnswerSpaceType {
    #[default]
    Lines = 0,
    PlainBox = 1,
    DiagramBox = 2,
    ConstructionBox = 3,
    GridBox = 4,
}

impl TryFrom<i16> for AnswerSpaceType {
    type Error = crate::types::error::Error;
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Lines),
            1 => Ok(Self::PlainBox),
            2 => Ok(Self::DiagramBox),
            3 => Ok(Self::ConstructionBox),
            4 => Ok(Self::GridBox),
            _ => Err(crate::types::error::Error::InvalidQuestionText),
        }
    }
}
impl From<AnswerSpaceType> for i16 {
    fn from(v: AnswerSpaceType) -> i16 {
        v as i16
    }
}
impl ToSql<SmallInt, Sqlite> for AnswerSpaceType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(*self as i32);
        Ok(IsNull::No)
    }
}
impl FromSql<SmallInt, Sqlite> for AnswerSpaceType {
    fn from_sql(
        bytes: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let v = <i16 as FromSql<SmallInt, Sqlite>>::from_sql(bytes)?;
        Self::try_from(v).map_err(|e| e.to_string().into())
    }
}

// ── StimulusType ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StimulusType {
    Passage = 0,
    Table = 1,
    Graph = 2,
    Diagram = 3,
}

// ── ExampleAnswerFormat ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExampleAnswerFormat {
    Plain = 0,
    Tiptap = 1,
    Svg = 2,
    Image = 3,
}
