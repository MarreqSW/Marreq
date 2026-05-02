-- One-time backfill: catalog colors for canonical requirement / verification statuses.
-- Matches marreq-core/scripts/init_complete.sql. Only rows with NULL/empty tag_color are updated
-- so manually chosen colors in the UI are preserved.

-- Requirement statuses (tag + title as seeded)
UPDATE requirement_status SET tag_color = '#64748b'
WHERE tag = 'Drf' AND title = 'Draft' AND (tag_color IS NULL OR btrim(tag_color) = '');

UPDATE requirement_status SET tag_color = '#7c3aed'
WHERE tag = 'Pro' AND title = 'Proposal' AND (tag_color IS NULL OR btrim(tag_color) = '');

UPDATE requirement_status SET tag_color = '#15803d'
WHERE tag = 'Acc' AND title = 'Accepted' AND (tag_color IS NULL OR btrim(tag_color) = '');

UPDATE requirement_status SET tag_color = '#b91c1c'
WHERE tag = 'Rej' AND title = 'Rejected' AND (tag_color IS NULL OR btrim(tag_color) = '');

UPDATE requirement_status SET tag_color = '#57534e'
WHERE tag = 'Can' AND title = 'Cancelled' AND (tag_color IS NULL OR btrim(tag_color) = '');

UPDATE requirement_status SET tag_color = '#0e7490'
WHERE tag = 'Fsh' AND title = 'Finished' AND (tag_color IS NULL OR btrim(tag_color) = '');

-- Verification statuses
UPDATE verification_status SET tag_color = '#15803d'
WHERE tag = 'Pass' AND title = 'Passed' AND (tag_color IS NULL OR btrim(tag_color) = '');

UPDATE verification_status SET tag_color = '#b91c1c'
WHERE tag = 'Fail' AND title = 'Failed' AND (tag_color IS NULL OR btrim(tag_color) = '');

UPDATE verification_status SET tag_color = '#b45309'
WHERE tag = 'Pend' AND title = 'Pending' AND (tag_color IS NULL OR btrim(tag_color) = '');

UPDATE verification_status SET tag_color = '#1d4ed8'
WHERE tag = 'Prog' AND title = 'In Progress' AND (tag_color IS NULL OR btrim(tag_color) = '');
