-- Fix REQ-PROP-002 title and description that were switched
-- Update the requirement where req_reference = 'REQ-PROP-002'
UPDATE requirements 
SET 
    req_title = 'Adequate thrust is required for attitude control during various mission phases.',
    req_description = 'The thrusters shall provide minimum thrust of 1N for attitude control'
WHERE req_reference = 'REQ-PROP-002';
