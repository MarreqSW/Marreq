-- Fix requirement 17 after the column swap
-- Currently: req_title = 'REQ-PROP-002', req_reference = long description
-- Need to swap them back to correct positions

UPDATE requirements 
SET 
    req_title = 'Adequate thrust is required for attitude control during various mission phases.',
    req_reference = 'REQ-PROP-002'
WHERE req_id = 17;
