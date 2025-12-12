-- Additional composite indexes that improve dashboard level queries.

CREATE INDEX IF NOT EXISTS idx_requirements_project_status
    ON requirements (project_id, req_current_status);

CREATE INDEX IF NOT EXISTS idx_tests_project_status
    ON tests (project_id, test_status);

CREATE INDEX IF NOT EXISTS idx_matrix_project_req
    ON matrix (project_id, matrix_req_id);
