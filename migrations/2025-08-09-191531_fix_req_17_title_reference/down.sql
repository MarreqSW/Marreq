-- Rollback: Restore original (incorrect) title and reference for requirement 17
-- This reverses the fix and puts the title and reference back to their corrupted state

UPDATE requirements 
SET 
    title = 'Adequate thrust is required for attitude control during various mission phases.',
    reference_code = 'Adequate thrust is required for attitude control during various mission phases.'
WHERE id = 17;
