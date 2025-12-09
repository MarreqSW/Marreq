-- Migration: Create projects table
-- This migration adds project support to the ReqMan application

-- Create projects table
CREATE TABLE projects (
    project_id SERIAL PRIMARY KEY,
    project_name VARCHAR(255) NOT NULL,
    project_description TEXT,
    project_creation_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    project_update_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    project_status VARCHAR(50) DEFAULT 'active',
    project_owner_id INTEGER REFERENCES users(user_id)
);

-- Create indexes for better performance
CREATE INDEX idx_projects_status ON projects(project_status);
CREATE INDEX idx_projects_owner ON projects(project_owner_id);

-- Insert a default project
INSERT INTO projects (project_name, project_description, project_creation_date, project_status)
VALUES ('Default Project', 'Default project for existing data', CURRENT_TIMESTAMP, 'active'); 