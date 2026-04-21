-- Convert topic grade numbering from relative year (1–4) to absolute
-- 8-4-4 Form codes (41–44). All existing topics are secondary school
-- subjects (Forms 1–4 of the Kenyan 8-4-4 curriculum). This aligns
-- topic grades with the grade codes stored in the papers table.
UPDATE topics
SET grade   = grade + 40,
    updated = unixepoch('now')
WHERE grade BETWEEN 1 AND 4;
