-- Fix requirement 17 after the column swap
-- Currently: title = 'REQ-PROP-002', reference_code = long description
-- Need to swap them back to correct positions

UPDATE requirements 
SET 
    title = 'Adequate thrust is required for attitude control during various mission phases.',
    reference_code = 'REQ-PROP-002'
WHERE id = 17;
