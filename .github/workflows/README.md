# Marreq CI/CD Workflows

This directory contains GitHub Actions workflows and supporting files for automated testing, linting, and quality checks for the Marreq project.

## Table of Contents

- [Overview](#overview)
- [Workflows](#workflows)
  - [Marreq CI Pipeline](#marreq-ci-pipeline)
- [Acceptance Criteria](#acceptance-criteria)
  - [1. Code Formatting](#1-code-formatting)
  - [2. CSS Linting](#2-css-linting)
  - [3. Unused CSS Detection](#3-unused-css-detection)
  - [4. Frontend JavaScript Tests](#4-frontend-javascript-tests)
  - [5. Backend Tests and Coverage](#5-backend-tests-and-coverage)
- [Running Checks Locally](#running-checks-locally)
  - [Prerequisites](#prerequisites)
  - [Local Setup](#local-setup)
  - [Individual Checks](#individual-checks)
- [Supporting Files](#supporting-files)
- [Troubleshooting](#troubleshooting)

---

## Overview

The CI pipeline runs automatically on pull requests and ensures code quality through multiple checks:
- Rust code formatting
- CSS linting and unused selector detection
- JavaScript unit tests
- Rust backend tests with code coverage (minimum 70% line coverage required)

All checks must pass before a pull request can be merged.

---

## Workflows

### Marreq CI Pipeline

**File:** `marreq-ci.yml`

**Triggered on:**
- Pull request opened, synchronized, reopened, or marked ready for review

**Jobs:**

1. **lint** - Code formatting and CSS quality checks
2. **frontend-tests** - JavaScript unit tests with Vitest
3. **test-and-coverage** - Rust backend tests with coverage analysis

**Concurrency:** Only one workflow runs per PR at a time (newer runs cancel older ones)

---

## Acceptance Criteria

### 1. Code Formatting

**Requirement:** All Rust code must follow the project's formatting standards using `rustfmt`.

**Acceptance Criteria:**
- All `.rs` files formatted according to Rust nightly `rustfmt` rules
- No formatting differences when running `cargo fmt --all -- --check`
- CI fails if any file needs reformatting

**Rationale:** Consistent code formatting improves readability and reduces diff noise in pull requests.

---

### 2. CSS Linting

**Requirement:** All CSS files must follow the project's style guide and naming conventions.

**Acceptance Criteria:**
- All CSS files pass `stylelint` checks with `stylelint-config-standard`
- Selectors follow BEM-style naming with required prefixes: `marreq-`, `c-`, `o-`, `u-`, `is-`, `has-`, `status-`, or `js-`
- Maximum selector specificity: `0,3,0`
- No ID selectors allowed
- No `!important` declarations
- Properties must be in alphabetical order
- Maximum nesting depth: 3 levels
- Maximum compound selectors: 4

**Rationale:** Consistent CSS conventions ensure maintainable stylesheets and prevent specificity wars.

---

### 3. Unused CSS Detection

**Requirement:** No unused CSS selectors should remain in the codebase.

**Acceptance Criteria:**
- PurgeCSS analysis reports unused selectors; tolerance allows legacy pre-SPA CSS still in the tree (see `purgecss-ci.mjs`)
- All CSS classes are referenced in templates (`templates/**/*.hbs`) or JavaScript files
- Dynamic classes are properly safelisted in `purgecss.config.cjs`
- CI fails if unused selectors are detected

**Rationale:** Removing unused CSS reduces bundle size and improves application performance.

---

### 4. Frontend JavaScript Tests

**Requirement:** All JavaScript functionality must be covered by unit tests.

**Acceptance Criteria:**
- All tests in `tests/js/**/*.test.js` pass using Vitest
- Tests run in a happy-dom environment
- No test failures or errors
- Tests validate:
  - DOM manipulation functions
  - Event handlers
  - API interaction logic
  - State management
  - Utility functions

**Rationale:** Frontend tests ensure UI components work correctly and prevent regressions.

---

### 5. Backend Tests and Coverage

**Requirement:** Backend Rust code must be thoroughly tested with minimum 70% line coverage.

**Acceptance Criteria:**
- All Rust tests pass (`cargo test`)
- Minimum 70% line coverage across the workspace
- Coverage report generated and posted to PR as comment
- Database properly initialized with test data
- Integration tests can connect to PostgreSQL
- Coverage includes:
  - Unit tests
  - Integration tests
  - Doctests

**Rationale:** High test coverage ensures reliability and catches bugs before they reach production.

---

## Running Checks Locally

### Prerequisites

Before running checks locally, ensure you have:

1. **Rust toolchain** (nightly)
   ```bash
   rustup toolchain install nightly
   rustup component add rustfmt --toolchain nightly
   rustup component add llvm-tools-preview --toolchain nightly
   ```

2. **Node.js** (v20 or later)
   ```bash
   # Check version
   node --version
   ```

3. **Docker** and **Docker Compose**
   ```bash
   # Check Docker
   docker --version
   docker compose version
   ```

4. **Additional tools**
   ```bash
   # Install cargo-llvm-cov for coverage
   cargo install cargo-llvm-cov
   
   # Install Node dependencies
   npm install
   ```

### Local Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/mariusmm/Marreq.git
   cd Marreq
   ```

2. **Start the database**
   ```bash
   docker compose -f docker/docker-compose.yml up -d db
   ```

3. **Initialize the database**
   ```bash
   ./marreq-core/scripts/db_setup.sh --seed
   ```

4. **Verify database is ready**
   ```bash
   docker compose -f docker/docker-compose.yml ps
   # Should show db service as "healthy"
   ```

### Individual Checks

#### 1. Check Rust Code Formatting

```bash
# Check if formatting is correct (doesn't modify files)
cargo +nightly fmt --all -- --check

# Auto-fix formatting
cargo +nightly fmt --all
```

**Expected output:** No output means formatting is correct. If files need formatting, they will be listed.

---

#### 2. Run CSS Linting

```bash
# Lint CSS files
npx stylelint "src/html/static/**/*.css" --config .stylelintrc.json

# Auto-fix CSS issues where possible
npx stylelint "src/html/static/**/*.css" --config .stylelintrc.json --fix
```

**Expected output:** No errors means CSS follows all style rules.

---

#### 3. Check for Unused CSS

```bash
# Run PurgeCSS analysis
node .github/workflows/purgecss-ci.mjs

# Or use npm script
npm run check:unused-css
```

**Expected output:** "Total unused selectors: 0" means no unused CSS found.

**Note:** If legitimate classes are flagged as unused, add them to the safelist in `purgecss.config.cjs`.

---

#### 4. Run Frontend JavaScript Tests

```bash
# Run all tests
npm test

# Run tests in watch mode (for development)
npm run test:watch

# Run tests with UI
npm run test:ui

# Run tests with coverage report
npm run test:coverage
```

**Expected output:** All tests pass with no failures.

**Coverage report location:** `coverage/index.html`

---

#### 5. Run Backend Tests with Coverage

**Prerequisites:** Database must be running and initialized.

```bash
# Set database URL (or use .env file)
export DATABASE_URL=postgres://rust:rust@127.0.0.1:5433/marreq

# Run tests with coverage
cargo llvm-cov --workspace --all-features --doctests

# Generate HTML coverage report
cargo llvm-cov --workspace --all-features --doctests --html --output-dir coverage_html

# Generate coverage report and check minimum threshold
cargo llvm-cov --workspace --all-features --doctests --fail-under-lines 70

# Generate Cobertura XML for tooling
cargo llvm-cov --workspace --all-features --doctests --cobertura --output-path coverage.xml
```

**Expected output:** All tests pass and coverage meets or exceeds 70%.

**Coverage report location:** `coverage_html/index.html`

**Using cargo-cov alias (if configured):**
```bash
cargo cov --workspace --all-features --doctests
```

---

#### 6. Run All Checks (Full CI Simulation)

Run all checks in sequence:

```bash
bash scripts/run_ci.sh local-ci
```

---

## Supporting Files

### docker-compose.ci.yml

**Location:** `docker/docker-compose.ci.yml`

CI-specific Docker Compose overrides:
- Disables container restart policy for CI runs
- Uses tmpfs for `/tmp` to improve performance
- Removes persistent volume mounting (uses ephemeral storage)

### purgecss-ci.mjs

Node.js script that analyzes CSS files for unused selectors:
- Imports PurgeCSS configuration from `purgecss.config.cjs`
- Scans templates and JavaScript files for class usage
- Reports unused selectors with zero tolerance
- Exits with error if any unused CSS is found
- Outputs results to GitHub Step Summary in CI

### purgecss.config.cjs

Configuration for PurgeCSS:
- **Content sources:** Handlebars templates, JavaScript files
- **CSS sources:** All files in `src/html/static/**/*.css`
- **Safelist:** Dynamic classes added via JavaScript (status badges, editor states, etc.)
- **Patterns:** BEM modifiers, pseudo-classes, theme variants

### wait_for_db.sh

Database health check script used in CI:
- Waits up to 60 seconds for PostgreSQL to be healthy
- Checks container health status via Docker inspect
- Falls back to `pg_isready` if no healthcheck defined
- Shows container logs on failure
- Used before running database migrations and tests

---

## Troubleshooting

### Database Connection Issues

**Problem:** Tests fail with "connection refused" or "database doesn't exist"

**Solutions:**
```bash
# Check database is running
docker compose -f docker/docker-compose.yml ps

# Check database health
docker compose -f docker/docker-compose.yml exec db pg_isready -U rust

# Restart database
docker compose -f docker/docker-compose.yml restart db

# Reinitialize database
./marreq-core/scripts/db_setup.sh --seed
```

---

### Coverage Below Threshold

**Problem:** `cargo llvm-cov` fails with "coverage below 70%"

**Solutions:**
1. Add tests for untested code paths
2. Remove dead code that isn't being tested
3. Check which files have low coverage:
   ```bash
   cargo llvm-cov --workspace --all-features --doctests --html
   # Open coverage_html/index.html in browser
   ```

---

### Unused CSS Detected

**Problem:** PurgeCSS finds unused selectors

**Solutions:**
1. Remove unused CSS classes from stylesheets
2. If classes are dynamically added via JavaScript, add them to `safelist` in `purgecss.config.cjs`:
   ```javascript
   safelist: {
     standard: [
       /^your-dynamic-class-pattern/,
     ],
   }
   ```

---

### Formatting Failures

**Problem:** `cargo fmt --check` reports formatting issues

**Solutions:**
```bash
# Auto-fix all formatting issues
cargo +nightly fmt --all

# Commit the changes
git add -A
git commit -m "fix: Apply rustfmt formatting"
```

---

### Stylelint Errors

**Problem:** CSS files fail stylelint checks

**Solutions:**
```bash
# Auto-fix issues where possible
npx stylelint "src/html/static/**/*.css" --config .stylelintrc.json --fix

# Review and fix remaining issues manually
```

Common issues:
- Class names must follow BEM with required prefixes
- Properties must be alphabetically ordered
- Maximum specificity exceeded (refactor selectors)
- Using `!important` (remove and fix specificity)

---

### Frontend Tests Failing

**Problem:** Vitest tests fail or timeout

**Solutions:**
```bash
# Run tests in watch mode to debug
npm run test:watch

# Run with UI for better debugging
npm run test:ui

# Check test output for specific errors
npm test -- --reporter=verbose
```

Common issues:
- Missing DOM elements (check setup.js)
- Async timing issues (use proper async/await)
- Mock data not matching expected format

---

### CI Workflow Fails but Local Passes

**Possible causes:**
1. **Environment differences:** CI uses fresh environment
2. **Cached dependencies:** Clear cache and reinstall
   ```bash
   cargo clean
   rm -rf node_modules package-lock.json
   npm install
   ```
3. **Database state:** CI uses fresh database, local might have stale data
   ```bash
   ./marreq-core/scripts/db_setup.sh --seed  # Reinitialize
   ```

---

## Additional Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust Formatting Guide](https://github.com/rust-lang/rustfmt)
- [Stylelint Rules](https://stylelint.io/user-guide/rules)
- [PurgeCSS Documentation](https://purgecss.com/)
- [Vitest Documentation](https://vitest.dev/)
- [cargo-llvm-cov Documentation](https://github.com/taiki-e/cargo-llvm-cov)

---

## Contributing

When submitting a pull request:

1. Run all checks locally before pushing
2. Ensure all tests pass
3. Maintain or improve code coverage
4. Follow formatting and linting rules
5. Add tests for new functionality
6. Update documentation as needed

The CI pipeline will automatically run on your PR and report results. All checks must pass before the PR can be merged.
