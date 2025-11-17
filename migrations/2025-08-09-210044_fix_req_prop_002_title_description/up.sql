-- Fix REQ-PROP-002 title and description that were switched
-- Update the requirement where id = 17 (which corresponds to REQ-PROP-002)
UPDATE requirements 
SET 
    title = 'Adequate thrust is required for attitude control during various mission phases.',
    description = 'The thrusters shall provide minimum thrust of 1N for attitude control'
WHERE id = 17;

-- Also update by reference as backup
UPDATE requirements 
SET 
    title = 'Adequate thrust is required for attitude control during various mission phases.',
    description = 'The thrusters shall provide minimum thrust of 1N for attitude control'
WHERE reference_code = 'REQ-PROP-002';
