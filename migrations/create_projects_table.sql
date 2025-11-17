-- Migration: Create projects table
-- This migration adds project support to the ReqMan application

-- Create projects table
CREATE TABLE projects (
    project_id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    creation_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    update_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    status_id VARCHAR(50) DEFAULT 'active',
    owner_id INTEGER REFERENCES users(id)
);

-- Create indexes for better performance
CREATE INDEX idx_projects_status ON projects(status_id);
CREATE INDEX idx_projects_owner ON projects(owner_id);

-- Insert a default project
INSERT INTO projects (name, description, creation_date, status_id)
VALUES ('Default Project', 'Default project for existing data', CURRENT_TIMESTAMP, 'active'); 