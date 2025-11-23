-- Reinstate the NOT NULL constraint on the requirement deadline.

UPDATE requirements
SET req_deadline_date = COALESCE(req_deadline_date, NOW());

ALTER TABLE requirements
    ALTER COLUMN req_deadline_date SET NOT NULL;
