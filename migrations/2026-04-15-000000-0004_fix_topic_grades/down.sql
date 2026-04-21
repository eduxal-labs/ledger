UPDATE topics
SET grade   = grade - 40,
    updated = unixepoch('now')
WHERE grade BETWEEN 41 AND 44;
