-- Rollback: Swap the req_title and req_reference columns back to their incorrect state
-- This reverses the fix and puts the columns back to their swapped positions

-- First, let's create a temporary table to store the current data
CREATE TEMP TABLE temp_requirements AS 
SELECT 
    req_id,
    req_reference as temp_title,  -- Current reference becomes title
    req_title as temp_reference   -- Current title becomes reference
FROM requirements;

-- Now update the requirements table with the swapped values (reverse of the fix)
UPDATE requirements 
SET 
    req_title = temp_requirements.temp_title,
    req_reference = temp_requirements.temp_reference
FROM temp_requirements 
WHERE requirements.req_id = temp_requirements.req_id;

-- Drop the temporary table
DROP TABLE temp_requirements;
