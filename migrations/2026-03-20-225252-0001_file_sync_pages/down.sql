-- Undo file sync metadata tables

DROP INDEX IF EXISTS idx_answer_pages_student;
DROP INDEX IF EXISTS idx_scheme_pages_paper;

DROP TABLE IF EXISTS answer_pages;
DROP TABLE IF EXISTS scheme_pages;
