-- Rollback: Restore original (incorrect) title and reference for requirement 17
-- This reverses the fix and puts the title and reference back to their corrupted state

UPDATE requirements 
SET 
    req_title = 'Adequate thrust is required for attitude control during various mission phases.',
    req_reference = 'Adequate thrust is required for attitude control during various mission phases.'
WHERE req_id = 17;
