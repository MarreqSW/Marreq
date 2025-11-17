-- Create applicability table
CREATE TABLE applicability
(
    app_id            SERIAL PRIMARY KEY,
    app_title         VARCHAR NOT NULL DEFAULT ' ',
    app_description   VARCHAR NOT NULL DEFAULT ' ',
    app_tag           VARCHAR NOT NULL DEFAULT ' '
);

-- Insert default applicability values
INSERT INTO applicability (app_title, app_description, app_tag) VALUES
    ('All Products', 'Applicable to all products in the system', 'ALL'),
    ('Product A', 'Applicable only to Product A', 'PROD_A'),
    ('Product B', 'Applicable only to Product B', 'PROD_B'),
    ('Legacy Systems', 'Applicable to legacy systems only', 'LEGACY'),
    ('New Systems', 'Applicable to new systems only', 'NEW');

-- Add applicability column to requirements table
ALTER TABLE requirements ADD COLUMN applicability_id INTEGER NOT NULL DEFAULT 1;

-- Add foreign key constraint
ALTER TABLE requirements ADD CONSTRAINT fk_requirements_applicability 
    FOREIGN KEY (applicability_id) REFERENCES applicability(app_id);
