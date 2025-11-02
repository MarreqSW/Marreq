# Requirements Page Testing Documentation

This document describes the comprehensive test suite for the requirements management pages in ReqMan.

## Test Organization

Tests are organized into three main categories:

### 1. Unit Tests (Module-level)
**Location**: `src/routes/html/project/requirements.rs` (in `#[cfg(test)] mod tests`)

These tests verify individual route handlers and their core logic.

**Coverage**:
- ✅ Basic CRUD operations (Create, Read, Update, Delete)
- ✅ Filtering by status, category, and verification
- ✅ Form validation and input handling
- ✅ Reference generation and validation
- ✅ Project ownership enforcement
- ✅ Permission checks (admin vs regular users)
- ✅ Parent-child relationship handling
- ✅ Linked test display
- ✅ Inline resource creation (categories, applicability, verification)
- ✅ Requirements tree view
- ✅ Error handling for edge cases
- ✅ Security validation

**Running Unit Tests**:
```bash
cargo test --lib routes::html::project::requirements::tests
```

### 2. Frontend Integration Tests
**Location**: `tests/frontend_requirements_test.rs`

These tests verify the complete HTML rendering, form behavior, and frontend-backend integration.

**Coverage**:
- ✅ HTML structure and semantic elements
- ✅ Data attributes for JavaScript hooks
- ✅ Form field presence and validation attributes
- ✅ Breadcrumb navigation
- ✅ Action buttons and links
- ✅ Filter controls and search inputs
- ✅ Metrics display
- ✅ Sortable table headers
- ✅ Empty state handling
- ✅ HTTP redirects after form submissions
- ✅ Error page responses
- ✅ Accessibility attributes (ARIA)

**Running Frontend Tests**:
```bash
cargo test --test frontend_requirements_test
```

### 3. JavaScript Tests
**Location**: `tests/js/*.test.js`

These tests verify client-side JavaScript functionality using Vitest.

**Coverage**:

#### `requirements.test.js` - Requirements List Page
- ✅ Table rendering and row collection
- ✅ Status badge decoration
- ✅ Search/filter functionality with debouncing
- ✅ Client-side sorting (key, title, status, date, author)
- ✅ Row detail toggle
- ✅ Filter chip rendering and removal
- ✅ Keyboard shortcuts (/ for search, n for new)
- ✅ Empty state and no results display
- ✅ Duplicate requirement action

#### `requirementForm.test.js` - Requirement Forms
- ✅ Form validation
- ✅ Reference format validation
- ✅ Combobox enhancement for selects
- ✅ Status menu controls
- ✅ Data attribute presence
- ✅ Flash message handling
- ✅ Autosave indicators
- ✅ Category tag integration
- ✅ Parent requirement selection

**Running JavaScript Tests**:
```bash
npm test                    # Run once
npm run test:watch          # Watch mode
npm run test:ui             # Visual UI
npm run test:coverage       # With coverage report
```

### 4. End-to-End Workflow Tests
**Location**: `tests/workflow_requirements_test.rs`

These tests verify complete user workflows across multiple pages and operations.

**Coverage**:
- ✅ Complete lifecycle: Create → View → Edit → Delete
- ✅ Parent-child hierarchy creation and navigation
- ✅ Multi-criteria filtering and search
- ✅ Permission enforcement (admin vs regular user)
- ✅ Inline resource creation during requirement creation
- ✅ "Add another" workflow for batch creation
- ✅ Template-based requirement creation
- ✅ Data consistency across operations

**Running Workflow Tests**:
```bash
cargo test --test workflow_requirements_test
```

## Test Helpers and Utilities

### Rust Test Helpers
**Location**: `src/routes/html/project/test_helpers.rs`

Provides common utilities for Rust-based tests:
- `test_client()` - Creates a Rocket client with routes
- `session_cookie()` - Generates authenticated session cookies
- `get_with_session()`, `post_form_with_session()`, `delete_with_session()` - HTTP helpers
- `managed_state()` - Test application state with mock repository
- `timestamp()` - Consistent timestamp for test data

### JavaScript Test Setup
**Location**: `tests/js/setup.js`

Global setup for JavaScript tests:
- Mock console methods
- localStorage/sessionStorage mocks
- Automatic cleanup between tests

## Running All Tests

### Backend (Rust) Tests
```bash
# All tests including unit and integration
cargo test

# Only unit tests in the requirements module
cargo test --lib requirements::tests

# Only integration tests
cargo test --test '*'

# With output
cargo test -- --nocapture

# Specific test
cargo test complete_requirement_lifecycle
```

### Frontend (JavaScript) Tests
```bash
# Install dependencies (first time)
npm install

# Run all JS tests
npm test

# Watch mode for development
npm run test:watch

# Visual UI
npm run test:ui

# Generate coverage report
npm run test:coverage
```

### Complete Test Suite
```bash
# Run everything
cargo test && npm test
```

## Test Coverage

### Route Handler Coverage
- `show_requirements` ✅ (list page with filters)
- `show_requirement_id` ✅ (detail view)
- `new_requirement` ✅ (form display)
- `post_requirement` ✅ (create action)
- `get_edit_requirement` ✅ (edit form)
- `post_edit_requirement` ✅ (update action)
- `delete_requirement_route` ✅ (delete action)
- `show_requirements_tree` ✅ (tree view)
- `create_category_inline` ✅ (inline creation)
- `create_applicability_inline` ✅ (inline creation)
- `create_verification_inline` ✅ (inline creation)

### Feature Coverage
- ✅ Authentication and authorization
- ✅ Form validation (client and server)
- ✅ Reference auto-generation
- ✅ Parent-child relationships
- ✅ Filtering and searching
- ✅ Sorting
- ✅ Inline resource creation
- ✅ Template-based creation
- ✅ Batch operations
- ✅ Error handling
- ✅ Redirects
- ✅ Accessibility
- ✅ Metrics calculation

## Testing Best Practices

### When Writing Tests

1. **Use Descriptive Names**: Test names should clearly describe what they verify
   ```rust
   #[rocket::async_test]
   async fn post_requirement_with_invalid_reference_format() { ... }
   ```

2. **Arrange-Act-Assert Pattern**: Structure tests clearly
   ```rust
   // Arrange
   let client = test_client(base_repo()).await;
   
   // Act
   let response = post_form_with_session(...).await;
   
   // Assert
   assert_eq!(response.status(), Status::Ok);
   ```

3. **Test One Thing**: Each test should verify a single behavior
   
4. **Use Meaningful Assertions**: Check the right things
   ```rust
   assert!(html.contains("data-requirement-id=\"1\""), "Missing requirement ID");
   ```

5. **Clean Up**: Use `beforeEach()` in JS tests to reset state

### Mock Data Consistency

All tests use consistent mock data:
- User ID 1: Admin user
- User ID 2: Regular user  
- Project ID 1: "Test Project"
- Category 1: "Systems" (SYS)
- Category 2: "Network" (NET)
- Status 1: Draft
- Status 2: Accepted
- Status 3: Released

## Continuous Integration

Tests should be run in CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Run Rust tests
  run: cargo test --all-features

- name: Install JS dependencies
  run: npm ci

- name: Run JS tests
  run: npm test
```

## Coverage Goals

Target coverage metrics:
- **Unit Tests**: >80% of route handler logic
- **Integration Tests**: All critical user paths
- **JavaScript Tests**: >70% of client-side code
- **Workflow Tests**: All major feature workflows

## Debugging Tests

### Rust Tests
```bash
# Show println! output
cargo test -- --nocapture

# Run specific test with output
cargo test show_requirements_lists_project_items -- --nocapture

# Show backtrace on failure
RUST_BACKTRACE=1 cargo test
```

### JavaScript Tests
```bash
# Run tests in UI mode for easier debugging
npm run test:ui

# Run specific test file
npx vitest requirements.test.js

# Enable console logs
# (Edit tests/js/setup.js to remove console mocks)
```

## Adding New Tests

### For New Route Handler
1. Add unit test in `src/routes/html/project/requirements.rs`
2. Add frontend integration test in `tests/frontend_requirements_test.rs`
3. If it involves JS, add test in `tests/js/`
4. Document in this README

### For New Feature
1. Add workflow test in `tests/workflow_requirements_test.rs`
2. Add component tests as needed
3. Update coverage goals

## Known Issues and Limitations

- JavaScript tests use mocked DOM (happy-dom), not real browser
- Some async timing issues may require setTimeout in JS tests
- Mock repository doesn't fully simulate database constraints
- Tests don't cover WebSocket/real-time features

## Resources

- [Rocket Testing Guide](https://rocket.rs/v0.5/guide/testing/)
- [Vitest Documentation](https://vitest.dev/)
- [Rust Test Documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
