-- Optional tag color for requirement and test statuses (e.g. #3366cc for badge display).
ALTER TABLE requirement_status ADD COLUMN tag_color VARCHAR(20) NULL;
ALTER TABLE test_status ADD COLUMN tag_color VARCHAR(20) NULL;
