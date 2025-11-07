-- Restore req_link column to requirements table
ALTER TABLE requirements ADD COLUMN req_link VARCHAR NOT NULL DEFAULT ' ';
