-- Rollback: Swap the req_title and req_reference fields back to their incorrect state for Empty project
-- This reverses the fix and puts the title and reference back to their swapped positions

-- First, let's create a temporary table to store the current data
CREATE TEMP TABLE temp_empty_project AS 
SELECT 
    req_id,
    req_reference as temp_title,  -- Current reference becomes title
    req_title as temp_reference   -- Current title becomes reference
FROM requirements 
WHERE project_id = 5;

-- Now update the requirements table with the swapped values (reverse of the fix)
UPDATE requirements 
SET 
    req_title = temp_empty_project.temp_title,
    req_reference = temp_empty_project.temp_reference
FROM temp_empty_project 
WHERE requirements.req_id = temp_empty_project.req_id;

-- Drop the temporary table
DROP TABLE temp_empty_project;
