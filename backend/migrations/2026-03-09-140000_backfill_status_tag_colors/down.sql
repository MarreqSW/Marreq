-- Revert only rows this migration would have set (same tag/title + exact hex).

UPDATE requirement_status SET tag_color = NULL
WHERE tag = 'Drf' AND title = 'Draft' AND tag_color = '#64748b';

UPDATE requirement_status SET tag_color = NULL
WHERE tag = 'Pro' AND title = 'Proposal' AND tag_color = '#7c3aed';

UPDATE requirement_status SET tag_color = NULL
WHERE tag = 'Acc' AND title = 'Accepted' AND tag_color = '#15803d';

UPDATE requirement_status SET tag_color = NULL
WHERE tag = 'Rej' AND title = 'Rejected' AND tag_color = '#b91c1c';

UPDATE requirement_status SET tag_color = NULL
WHERE tag = 'Can' AND title = 'Cancelled' AND tag_color = '#57534e';

UPDATE requirement_status SET tag_color = NULL
WHERE tag = 'Fsh' AND title = 'Finished' AND tag_color = '#0e7490';

UPDATE verification_status SET tag_color = NULL
WHERE tag = 'Pass' AND title = 'Passed' AND tag_color = '#15803d';

UPDATE verification_status SET tag_color = NULL
WHERE tag = 'Fail' AND title = 'Failed' AND tag_color = '#b91c1c';

UPDATE verification_status SET tag_color = NULL
WHERE tag = 'Pend' AND title = 'Pending' AND tag_color = '#b45309';

UPDATE verification_status SET tag_color = NULL
WHERE tag = 'Prog' AND title = 'In Progress' AND tag_color = '#1d4ed8';
