-- Fix the swapped title and reference_code columns in the requirements table
-- Currently: title contains references, reference_code contains titles
-- Need to swap them back to their correct positions

-- First, let's create a temporary table to store the current data
CREATE TEMP TABLE temp_requirements AS 
SELECT 
    id,
    reference_code as temp_title,  -- Current reference becomes title
    title as temp_reference   -- Current title becomes reference
FROM requirements;

-- Now update the requirements table with the swapped values
UPDATE requirements 
SET 
    title = temp_requirements.temp_title,
    reference_code = temp_requirements.temp_reference
FROM temp_requirements 
WHERE requirements.id = temp_requirements.id;

-- Drop the temporary table
DROP TABLE temp_requirements;
