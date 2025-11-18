-- Create separate status tables for requirements and tests

-- Create requirement_status table
CREATE TABLE requirement_status (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    short_name VARCHAR NOT NULL
);

-- Create status_id table  
CREATE TABLE status_id (
    id SERIAL PRIMARY KEY,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    short_name VARCHAR NOT NULL
);

-- Insert requirement status values
INSERT INTO requirement_status (title, description, short_name) VALUES
    ('Draft', 'The requirement is still being edited', 'Drf'),
    ('Proposal', 'The requirement is still to be approved', 'Pro'),
    ('Accepted', 'The requirement is accepted and must be processed', 'Acc'),
    ('Rejected', 'The requirement is not accepted', 'Rej'),
    ('Cancelled', 'The requirement is cancelled', 'Can'),
    ('Finished', 'The requirement is finished', 'Fsh');

-- Insert test status values
INSERT INTO status_id (title, description, short_name) VALUES
    ('Draft', 'The test is still being edited', 'Drf'),
    ('Proposal', 'The test is still to be approved', 'Pro'),
    ('Accepted', 'The test is accepted and must be processed', 'Acc'),
    ('Rejected', 'The test is not accepted', 'Rej'),
    ('Passed', 'The test has passed', 'Pass'),
    ('Failed', 'The test has failed', 'Fail'),
    ('Cancelled', 'The test is cancelled', 'Can');

-- Update requirements table to use new requirement_status table
-- Map old status IDs to new requirement status IDs
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

-- Update tests table to use new status_id table
-- Map old status IDs to new test status IDs
UPDATE tests SET status_id = 
    CASE status_id
        WHEN 1 THEN 1  -- Draft
        WHEN 2 THEN 2  -- Proposal
        WHEN 3 THEN 3  -- Accepted
        WHEN 4 THEN 4  -- Rejected
        WHEN 5 THEN 5  -- Cancelled (mapped to new position)
        WHEN 6 THEN 6  -- Finished -> Passed
        WHEN 7 THEN 5  -- Passed -> Passed (new position)
        WHEN 8 THEN 6  -- Failed -> Failed (new position)
        ELSE 1         -- Default to Draft
    END;

-- Drop the old status table
DROP TABLE status;