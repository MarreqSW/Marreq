-- Fix requirement 17 (ID 17) title and reference
-- Currently both title and reference are the same long description
-- Need to set proper title and reference

UPDATE requirements 
SET 
    req_title = 'Adequate thrust is required for attitude control during various mission phases.',
    req_reference = 'REQ-PROP-002'
WHERE req_id = 17;
