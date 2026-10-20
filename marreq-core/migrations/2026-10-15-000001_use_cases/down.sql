DROP TABLE use_case_drafts;
ALTER TABLE use_cases DROP CONSTRAINT use_cases_current_version_fk;
DROP TABLE use_case_versions;
DROP TABLE use_cases;
