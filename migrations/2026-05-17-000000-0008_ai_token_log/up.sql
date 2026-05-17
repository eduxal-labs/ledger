-- ============================================================
-- 0008 up: AI token usage logging per student per paper
-- ============================================================

CREATE TABLE IF NOT EXISTS ai_token_log (
    paper           TEXT    NOT NULL,
    student         INTEGER NOT NULL,
    input_tokens    INTEGER NOT NULL DEFAULT 0,
    output_tokens   INTEGER NOT NULL DEFAULT 0,
    thinking_tokens INTEGER NOT NULL DEFAULT 0,
    cached_tokens   INTEGER NOT NULL DEFAULT 0,
    total_tokens    INTEGER NOT NULL DEFAULT 0,
    created         BIGINT  NOT NULL DEFAULT (unixepoch('now')),
    PRIMARY KEY (paper, student),
    FOREIGN KEY (paper) REFERENCES papers(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_ai_token_log_paper ON ai_token_log(paper);
