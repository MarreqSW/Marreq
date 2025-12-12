-- Revert: Make project status column nullable again

ALTER TABLE projects
    ALTER COLUMN status DROP NOT NULL,
    ALTER COLUMN status DROP DEFAULT;
