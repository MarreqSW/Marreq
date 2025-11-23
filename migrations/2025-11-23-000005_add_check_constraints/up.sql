-- Prevent empty strings in several key textual columns.

ALTER TABLE requirements
    ADD CONSTRAINT requirements_title_not_blank CHECK (btrim(req_title) <> '');

ALTER TABLE tests
    ADD CONSTRAINT tests_name_not_blank CHECK (btrim(test_name) <> '');

ALTER TABLE projects
    ADD CONSTRAINT projects_name_not_blank CHECK (btrim(project_name) <> '');
