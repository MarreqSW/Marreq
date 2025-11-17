-- Fix requirement 17 (ID 17) title and reference
-- Currently both title and reference are the same long description
-- Need to set proper title and reference

UPDATE requirements 
SET 
    title = 'Adequate thrust is required for attitude control during various mission phases.',
    reference_code = 'REQ-PROP-002'
WHERE id = 17;
