-- Rollback: Restore requirement 17 to its incorrect state after column swap
-- This reverses the fix and puts the title and reference back to their swapped positions

UPDATE requirements 
SET 
    title = 'REQ-PROP-002',
    reference_code = 'Adequate thrust is required for attitude control during various mission phases.'
WHERE id = 17;
