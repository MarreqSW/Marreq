-- Rollback: Restore original (incorrect) title and description for REQ-PROP-002
-- This reverses the fix and puts the title and description back to their switched state
UPDATE requirements 
SET 
    req_title = 'The thrusters shall provide minimum thrust of 1N for attitude control',
    req_description = 'Adequate thrust is required for attitude control during various mission phases.'
WHERE req_id = 17;

-- Also rollback by reference as backup
UPDATE requirements 
SET 
    req_title = 'The thrusters shall provide minimum thrust of 1N for attitude control',
    req_description = 'Adequate thrust is required for attitude control during various mission phases.'
WHERE req_reference = 'REQ-PROP-002';
