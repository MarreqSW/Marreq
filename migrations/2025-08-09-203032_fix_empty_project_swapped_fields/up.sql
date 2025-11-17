-- Fix the swapped title and reference_code fields in the Empty project (project_id = 5)
-- Currently: title contains reference codes, reference_code contains descriptions
-- Need to swap them back to their correct positions

-- First, let's create a temporary table to store the current data
CREATE TEMP TABLE temp_empty_project AS 
SELECT 
    id,
    reference_code as temp_title,  -- Current reference becomes title
    title as temp_reference   -- Current title becomes reference
FROM requirements 
WHERE project_id = 5;

-- Now update the requirements table with the swapped values
UPDATE requirements 
SET 
    title = temp_empty_project.temp_title,
    reference_code = temp_empty_project.temp_reference
FROM temp_empty_project 
WHERE requirements.id = temp_empty_project.id;

-- Drop the temporary table
DROP TABLE temp_empty_project;
