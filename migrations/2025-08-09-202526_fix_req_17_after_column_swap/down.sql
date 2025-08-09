-- Rollback: Restore requirement 17 to its incorrect state after column swap
-- This reverses the fix and puts the title and reference back to their swapped positions

UPDATE requirements 
SET 
    req_title = 'REQ-PROP-002',
    req_reference = 'Adequate thrust is required for attitude control during various mission phases.'
WHERE req_id = 17;
