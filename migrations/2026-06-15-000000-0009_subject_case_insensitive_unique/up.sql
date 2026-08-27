DROP INDEX IF EXISTS uq_subjects_name_curriculum;
CREATE UNIQUE INDEX uq_subjects_name_curriculum ON subjects(name COLLATE NOCASE, curriculum);
