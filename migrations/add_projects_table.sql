-- Migration: Add projects table and project relationships
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

-- Add project_id column to requirements table
ALTER TABLE requirements ADD COLUMN project_id INTEGER REFERENCES projects(project_id);

-- Add project_id column to tests table
ALTER TABLE tests ADD COLUMN project_id INTEGER REFERENCES projects(project_id);

-- Add project_id column to users table (for project-specific users)
ALTER TABLE users ADD COLUMN project_id INTEGER REFERENCES projects(project_id);

-- Add project_id column to categories table
ALTER TABLE categories ADD COLUMN project_id INTEGER REFERENCES projects(project_id);

-- Add project_id column to applicability table
ALTER TABLE applicability ADD COLUMN project_id INTEGER REFERENCES projects(project_id);

-- Add project_id column to matrix table
ALTER TABLE matrix ADD COLUMN project_id INTEGER REFERENCES projects(project_id);

-- Create indexes for better performance
CREATE INDEX idx_requirements_project_id ON requirements(project_id);
CREATE INDEX idx_tests_project_id ON tests(project_id);
CREATE INDEX idx_users_project_id ON users(project_id);
CREATE INDEX idx_categories_project_id ON categories(project_id);
CREATE INDEX idx_applicability_project_id ON applicability(project_id);
CREATE INDEX idx_matrix_project_id ON matrix(project_id);

-- Insert a default project for existing data
INSERT INTO projects (name, description, creation_date, status_id)
VALUES ('Default Project', 'Default project for existing data', CURRENT_TIMESTAMP, 'active');

-- Update existing data to belong to the default project
UPDATE requirements SET project_id = (SELECT project_id FROM projects WHERE name = 'Default Project') WHERE project_id IS NULL;
UPDATE tests SET project_id = (SELECT project_id FROM projects WHERE name = 'Default Project') WHERE project_id IS NULL;
UPDATE users SET project_id = (SELECT project_id FROM projects WHERE name = 'Default Project') WHERE project_id IS NULL;
UPDATE categories SET project_id = (SELECT project_id FROM projects WHERE name = 'Default Project') WHERE project_id IS NULL;
UPDATE applicability SET project_id = (SELECT project_id FROM projects WHERE name = 'Default Project') WHERE project_id IS NULL;
UPDATE matrix SET project_id = (SELECT project_id FROM projects WHERE name = 'Default Project') WHERE project_id IS NULL;

-- Make project_id NOT NULL after setting default values
ALTER TABLE requirements ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE tests ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE categories ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE applicability ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE matrix ALTER COLUMN project_id SET NOT NULL;

-- Note: users.project_id remains nullable to allow global users 