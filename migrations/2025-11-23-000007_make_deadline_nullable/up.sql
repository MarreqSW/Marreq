-- Allow requirements to omit a deadline.

ALTER TABLE requirements
    ALTER COLUMN req_deadline_date DROP NOT NULL;
