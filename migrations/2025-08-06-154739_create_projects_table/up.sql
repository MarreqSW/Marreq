-- Create projects table if it doesn't exist
CREATE TABLE IF NOT EXISTS projects (
    project_id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    creation_date TIMESTAMP,
    update_date TIMESTAMP,
    status_id VARCHAR(50),
    owner_id INTEGER
);

-- Insert default projects if they don't exist
INSERT INTO projects (project_id, name, description, creation_date, status_id)
VALUES 
    (1, 'Space project', 'Space exploration requirements', NOW(), 'Active'),
    (3, 'ReqMan Project', 'Requirements management system', NOW(), 'Active'),
    (5, 'Empty project', 'Empty project for testing', NOW(), 'Active')
ON CONFLICT (project_id) DO NOTHING;
