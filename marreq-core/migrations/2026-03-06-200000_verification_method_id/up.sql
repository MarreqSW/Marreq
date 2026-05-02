-- Add verification method (type) to verifications; project-scoped via verification_methods.
ALTER TABLE verifications
ADD COLUMN verification_method_id INTEGER NULL REFERENCES verification_methods(id);
