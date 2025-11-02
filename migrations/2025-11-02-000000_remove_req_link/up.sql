-- Remove req_link column from requirements table
ALTER TABLE requirements DROP COLUMN IF EXISTS req_link;
