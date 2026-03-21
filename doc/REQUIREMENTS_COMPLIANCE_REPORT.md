# Requirements compliance report

Generated against [doc/requirements.csv](requirements.csv). A requirement is considered **accomplished** here if (1) every path in its Justification "Trace: …" exists in the repo, and (2) the project's automated test suite passes.

*Last updated: after running trace compliance and full test suite.*

## Scope

- **Trace compliance:** Each requirement's Trace paths are resolved (with documented path aliases for renamed files) and checked for existence (file or directory). Globs (e.g. `src/services/*_service.rs`) are accepted if at least one match exists.
- **Test run:** The full Rust test suite (`cargo test`) and the JS test suite (`npm test`) are run. Requirements verified by Unit Test or Integration Test are not re-verified per requirement; the entire suite acts as the proxy. Manual Test, System Test, and Code Review requirements are not re-executed beyond trace existence and suite health.

## Trace compliance

Total requirements: 432  
Requirements with at least one missing trace path: 0  
Requirements with all trace paths present: 432  

**All traced paths exist.**

## Test run

| Suite        | Result   | Notes |
| ------------ | -------- | ----- |
| **Rust**     | **Pass** | `cargo test` — 1386+ tests passed (unit and integration: API, auth, repo, templates, etc.). |
| **JS**       | **Pass** | `npm test` (Vitest) — 22 test files, 421 passed, 7 skipped. |

Both suites must pass for requirements to be considered accomplished under this definition.

## How to re-run

1. **Trace-path compliance:**  
   `python3 scripts/check_requirements_compliance.py`  
   Exit 0 if all traces exist; exit 1 and list missing paths otherwise.

2. **Rust tests:**  
   `cargo test`

3. **JS tests:**  
   `npm test`

Re-run these after changing requirements (CSV or trace paths) or after code changes that might affect traced files or test behavior.
