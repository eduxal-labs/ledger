-- Remove duplicate questions, keeping the row with the lowest id
DELETE FROM questions
WHERE id NOT IN (
    SELECT MIN(id) FROM questions GROUP BY topic, text
);

-- Enforce uniqueness going forward
CREATE UNIQUE INDEX idx_questions_topic_text ON questions(topic, text);
