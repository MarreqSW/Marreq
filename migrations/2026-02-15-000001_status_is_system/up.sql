-- Mark built-in requirement and test statuses as system (immutable).
-- Match by (title, tag) to the enum-defined default set.

ALTER TABLE requirement_status ADD COLUMN is_system BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE test_status ADD COLUMN is_system BOOLEAN NOT NULL DEFAULT false;

-- Backfill requirement_status: default set (Draft/Drf, Proposal/Pro, Accepted/Acc, Rejected/Rej, Cancelled/Can, Finished/Fsh)
UPDATE requirement_status SET is_system = true
WHERE (title, tag) IN (
    ('Draft', 'Drf'),
    ('Proposal', 'Pro'),
    ('Accepted', 'Acc'),
    ('Rejected', 'Rej'),
    ('Cancelled', 'Can'),
    ('Finished', 'Fsh')
);

-- Backfill test_status: default set (Passed/Pass, Failed/Fail, Pending/Pend, In Progress/Prog)
UPDATE test_status SET is_system = true
WHERE (title, tag) IN (
    ('Passed', 'Pass'),
    ('Failed', 'Fail'),
    ('Pending', 'Pend'),
    ('In Progress', 'Prog')
);
