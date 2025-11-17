-- Your SQL goes here

CREATE TABLE logs (
    log_id SERIAL PRIMARY KEY,
    id INTEGER NOT NULL,
    action_type VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id INTEGER,
    project_id INTEGER,
    old_values JSONB,
    new_values JSONB,
    description TEXT,
    ip_address VARCHAR(45),
    user_agent TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Create indexes for better performance
CREATE INDEX idx_logs_user_id ON logs(id);
CREATE INDEX idx_logs_entity_type ON logs(entity_type);
CREATE INDEX idx_logs_entity_id ON logs(entity_id);
CREATE INDEX idx_logs_project_id ON logs(project_id);
CREATE INDEX idx_logs_created_at ON logs(created_at);
CREATE INDEX idx_logs_action_type ON logs(action_type);

-- Add foreign key constraints
ALTER TABLE logs ADD CONSTRAINT fk_logs_user_id FOREIGN KEY (id) REFERENCES users(id) ON DELETE CASCADE;
ALTER TABLE logs ADD CONSTRAINT fk_logs_project_id FOREIGN KEY (project_id) REFERENCES projects(project_id) ON DELETE CASCADE;
