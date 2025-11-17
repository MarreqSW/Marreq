-- Rollback: Swap the title and reference_code columns back to their incorrect state
-- This reverses the fix and puts the columns back to their swapped positions

-- First, let's create a temporary table to store the current data
CREATE TEMP TABLE temp_requirements AS 
SELECT 
    id,
    reference_code as temp_title,  -- Current reference becomes title
    title as temp_reference   -- Current title becomes reference
FROM requirements;

-- Now update the requirements table with the swapped values (reverse of the fix)
UPDATE requirements 
SET 
    title = temp_requirements.temp_title,
    reference_code = temp_requirements.temp_reference
FROM temp_requirements 
WHERE requirements.id = temp_requirements.id;

-- Drop the temporary table
DROP TABLE temp_requirements;
