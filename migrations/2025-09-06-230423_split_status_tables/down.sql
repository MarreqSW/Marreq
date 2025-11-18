-- Recreate the original status table
CREATE TABLE status (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    short_name VARCHAR NOT NULL
);

-- Insert all status values back into the unified table
INSERT INTO status (title, description, short_name) VALUES
    ('Draft', 'The requirement is still being edited', 'Drf'),
    ('Proposal', 'The requirement is still to be approved', 'Pro'),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc'),
    ('Rejected', 'The requirement is not accepted', 'Rej'),
    ('Cancelled', 'The requirement is cancelled', 'Can'),
    ('Finished', 'The requirement is finished', 'Fsh'),
    ('Passed', 'The test has passed', 'Pass'),
    ('Failed', 'The test has failed', 'Fail');

-- Update requirements table to use original status IDs
UPDATE requirements SET status_id = 
    CASE status_id
        WHEN 1 THEN 1  -- Draft
        WHEN 2 THEN 2  -- Proposal
        WHEN 3 THEN 3  -- Accepted
        WHEN 4 THEN 4  -- Rejected
        WHEN 5 THEN 5  -- Cancelled
        WHEN 6 THEN 6  -- Finished
        ELSE 1         -- Default to Draft
    END;

-- Update tests table to use original status IDs
UPDATE tests SET status_id = 
    CASE status_id
        WHEN 1 THEN 1  -- Draft
        WHEN 2 THEN 2  -- Proposal
        WHEN 3 THEN 3  -- Accepted
        WHEN 4 THEN 4  -- Rejected
        WHEN 5 THEN 7  -- Passed
        WHEN 6 THEN 8  -- Failed
        WHEN 7 THEN 5  -- Cancelled
        ELSE 1         -- Default to Draft
    END;

-- Drop the separate status tables
DROP TABLE status_id;
DROP TABLE requirement_status;