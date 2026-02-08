-- Revert immutable baselines

DROP TRIGGER IF EXISTS baseline_traceability_immutable ON baseline_traceability;
DROP TRIGGER IF EXISTS baseline_requirements_immutable ON baseline_requirements;
DROP TRIGGER IF EXISTS baselines_immutable ON baselines;
DROP FUNCTION IF EXISTS forbid_baseline_update_delete();

DROP TABLE IF EXISTS baseline_traceability;
DROP TABLE IF EXISTS baseline_requirements;
DROP TABLE IF EXISTS baselines;
