-- Create projects table if it doesn't exist
CREATE TABLE IF NOT EXISTS projects (
    project_id SERIAL PRIMARY KEY,
    project_name VARCHAR(255) NOT NULL,
    project_description TEXT,
    project_creation_date TIMESTAMP,
    project_update_date TIMESTAMP,
    project_status VARCHAR(50),
    project_owner_id INTEGER
);

-- Insert default projects if they don't exist
INSERT INTO projects (project_id, project_name, project_description, project_creation_date, project_status)
VALUES 
    (1, 'Space project', 'Space exploration requirements', NOW(), 'Active'),
    (3, 'ReqMan Project', 'Requirements management system', NOW(), 'Active'),
    (5, 'Empty project', 'Empty project for testing', NOW(), 'Active')
ON CONFLICT (project_id) DO NOTHING;
