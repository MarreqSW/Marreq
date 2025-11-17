-- Create applicability table
CREATE TABLE applicability
(
    id            SERIAL PRIMARY KEY,
    title         VARCHAR NOT NULL DEFAULT ' ',
    description   VARCHAR NOT NULL DEFAULT ' ',
    tag           VARCHAR NOT NULL DEFAULT ' '
);

-- Insert default applicability values
INSERT INTO applicability (title, description, tag) VALUES
    ('All Products', 'Applicable to all products in the system', 'ALL'),
    ('Product A', 'Applicable only to Product A', 'PROD_A'),
    ('Product B', 'Applicable only to Product B', 'PROD_B'),
    ('Legacy Systems', 'Applicable to legacy systems only', 'LEGACY'),
    ('New Systems', 'Applicable to new systems only', 'NEW');

-- Add applicability column to requirements table
ALTER TABLE requirements ADD COLUMN applicability_id INTEGER NOT NULL DEFAULT 1;

-- Add foreign key constraint
ALTER TABLE requirements ADD CONSTRAINT fk_requirements_applicability 
    FOREIGN KEY (applicability_id) REFERENCES applicability(id);
