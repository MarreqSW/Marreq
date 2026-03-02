# Test Coverage Recommendations for Marreq

## Executive Summary

This document analyzes the current test coverage and recommends files that would benefit from additional test coverage. The project has good integration test coverage for API endpoints (~85-90%), but several areas need more unit tests and edge case coverage.

## Current Test Coverage Status

### ✅ Well-Covered Areas
- **API Integration Tests**: Excellent coverage (200+ tests across 14 test files)
- **Validation Module**: Comprehensive unit tests (160+ tests)
- **Repository Layer**: Good mock-based tests (2400+ lines of tests)
- **Importers Module**: Comprehensive tests (1600+ lines)
- **Generators Module**: Good test coverage
- **Errors Module**: Comprehensive error handling tests
- **Auth Password Module**: Good unit tests

### ⚠️ Areas Needing More Coverage

## Priority 1: High Priority - Service Layer Unit Tests

### 1.1 Service Layer Files (Most Critical)

The service layer has minimal unit test coverage. While integration tests cover API endpoints, unit tests would catch bugs earlier and test edge cases more efficiently.

#### **`src/services/requirement_service.rs`** ⚠️ HIGH PRIORITY
**Current Coverage**: Only basic integration tests via API endpoints
**Missing Tests**:
- `list_by_project_filtered()` - All filter combinations
- Edge cases for filtering (empty results, invalid filters)
- Error handling when repository fails
- Validation error propagation
- Sanitization of input strings

**Recommended**: Create `src/services/tests/requirement_service_test.rs`

#### **`src/services/test_service.rs`** ⚠️ HIGH PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- `get_by_status()` - Currently marked as `todo!()` - needs implementation and tests
- `get_by_parent()` - Currently marked as `todo!()` - needs implementation and tests
- Hierarchical test structure handling
- Error cases for invalid status/parent IDs
- Logging verification for create/update/delete operations

**Recommended**: Create `src/services/tests/test_service_test.rs`

#### **`src/services/matrix_service.rs`** ⚠️ HIGH PRIORITY
**Current Coverage**: Only basic integration tests
**Missing Tests**:
- `export_matrix_csv()` - CSV generation logic
- `link()` method - Matrix link creation/validation
- Duplicate link prevention
- Error handling for invalid requirement/test IDs
- Project scoping validation
- Edge cases (empty matrix, single link, many links)

**Recommended**: Create `src/services/tests/matrix_service_test.rs`

#### **`src/services/decorated_requirement_service.rs`** ⚠️ MEDIUM PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- Decoration logic (joining with status, category, users, etc.)
- Missing foreign key handling (what if status_id doesn't exist?)
- Null/None value handling in decorated fields
- Performance with large datasets

**Recommended**: Create `src/services/tests/decorated_requirement_service_test.rs`

#### **`src/services/decorated_test_service.rs`** ⚠️ MEDIUM PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- Similar to decorated_requirement_service
- Test decoration with missing relationships
- Hierarchical parent-child decoration

**Recommended**: Create `src/services/tests/decorated_test_service_test.rs`

#### **`src/services/requirement_analytics_service.rs`** ✅ PARTIALLY COVERED
**Current Coverage**: Has some unit tests
**Missing Tests**:
- `metrics_via_sql()` path (currently only tests repository path)
- SQL query error handling
- Edge cases: zero requirements, all same status, division by zero in coverage calculation
- Filter combinations with SQL path
- Performance with large datasets

**Recommended**: Add more tests to existing test module

#### **`src/services/log_service.rs`** ✅ PARTIALLY COVERED
**Current Coverage**: Has basic tests
**Missing Tests**:
- `analytics()` method - Date range calculations
- `cleanup_old_logs()` - Cleanup logic and logging
- `log_export_action()` - Export logging
- Error handling when user lookup fails during enrichment
- Edge cases: empty logs, logs with missing users

**Recommended**: Expand existing test module

#### **`src/services/cache_service.rs`** ⚠️ MEDIUM PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- Cache hit/miss logic
- Cache invalidation strategies
- Cache statistics calculation
- Error handling when cache operations fail
- Cache expiration logic

**Recommended**: Create `src/services/tests/cache_service_test.rs`

#### **`src/services/project_service.rs`** ⚠️ MEDIUM PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- Project member management
- Permission checking logic
- Project status transitions
- Owner validation

**Recommended**: Create `src/services/tests/project_service_test.rs`

#### **`src/services/user_service.rs`** ⚠️ MEDIUM PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- User creation validation
- Password update logic
- Admin privilege checks
- Project membership queries

**Recommended**: Create `src/services/tests/user_service_test.rs`

#### **`src/services/verification_service.rs`** ⚠️ LOW PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- Sanitization of title/description
- Validation error handling
- Project scoping

**Recommended**: Create `src/services/tests/verification_service_test.rs`

#### **`src/services/category_service.rs`** ⚠️ LOW PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- Tag uniqueness validation within project
- Sanitization logic
- Project scoping

**Recommended**: Create `src/services/tests/category_service_test.rs`

#### **`src/services/applicability_service.rs`** ⚠️ LOW PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- Similar to category_service
- Tag uniqueness validation

**Recommended**: Create `src/services/tests/applicability_service_test.rs`

#### **`src/services/status_service.rs`** ⚠️ LOW PRIORITY
**Current Coverage**: Only integration tests
**Missing Tests**:
- Status creation/validation
- Project scoping

**Recommended**: Create `src/services/tests/status_service_test.rs`

## Priority 2: Helper Functions

### **`src/helper_functions/`** ⚠️ HIGH PRIORITY
**Current Coverage**: No visible unit tests
**Files to Test**:
- `decorators.rs` - Decoration logic for HTML templates
- `filters.rs` - Template filters
- `reports.rs` - Report generation helpers
- `utils.rs` - Utility functions

**Missing Tests**:
- All helper functions need unit tests
- Edge cases for formatting functions
- Error handling

**Recommended**: Create `src/helper_functions/tests.rs` or individual test files

## Priority 3: Authentication Module

### **`src/auth/`** ⚠️ MEDIUM PRIORITY
**Current Coverage**: Only `password.rs` has tests

#### **`src/auth/login.rs`** ⚠️ MEDIUM PRIORITY
**Missing Tests**:
- Login success/failure scenarios
- Session creation
- Cookie handling
- Error cases (invalid credentials, locked accounts, etc.)

**Recommended**: Create `src/auth/tests/login_test.rs`

#### **`src/auth/logout.rs`** ⚠️ LOW PRIORITY
**Missing Tests**:
- Session invalidation
- Cookie clearing

**Recommended**: Create `src/auth/tests/logout_test.rs`

#### **`src/auth/session.rs`** ⚠️ MEDIUM PRIORITY
**Missing Tests**:
- Session creation/validation
- Session expiration
- Cookie encryption/decryption
- Session storage/retrieval

**Recommended**: Create `src/auth/tests/session_test.rs`

#### **`src/auth/guards.rs`** ⚠️ MEDIUM PRIORITY
**Missing Tests**:
- Authentication guard logic
- Authorization checks
- Admin privilege verification
- Project access checks

**Recommended**: Create `src/auth/tests/guards_test.rs`

#### **`src/auth/errors.rs`** ⚠️ LOW PRIORITY
**Missing Tests**:
- Error display/formatting
- Error conversion

**Recommended**: Add tests to existing module

## Priority 4: Repository Layer Edge Cases

### **`src/repository/cache/`** ⚠️ MEDIUM PRIORITY
**Current Coverage**: Has tests but could be expanded
**Missing Tests**:
- Cache key generation edge cases
- Cache statistics edge cases
- Concurrent access scenarios
- Cache corruption recovery

**Recommended**: Expand existing tests

### **`src/repository/cache_middleware.rs`** ✅ WELL COVERED
**Current Coverage**: Good test coverage (1400+ lines)
**Note**: Already well tested, but could add:
- More concurrent access tests
- Performance tests

## Priority 5: Route Handlers (HTML Routes)

### **`src/routes/html/`** ⚠️ LOW PRIORITY
**Current Coverage**: Some files have basic tests, many don't
**Files Needing Tests**:
- Template rendering logic
- Form data parsing
- Error page generation
- Redirect logic

**Note**: These are often tested via integration tests, but unit tests for helper functions would be valuable.

## Priority 6: Database Constraint Violations

### **`tests/api_constraint_violation_test.rs`** ⚠️ HIGH PRIORITY
**Status**: File exists but may need expansion
**Missing Tests** (from API_TEST_COVERAGE_ANALYSIS.md):
- Unique constraint violations (duplicate usernames, emails, reference codes)
- Foreign key constraint violations
- NOT NULL constraint violations
- Cascading delete scenarios

**Recommended**: Review and expand existing test file

## Priority 7: Edge Cases and Error Scenarios

### **PATCH Operation Edge Cases** ⚠️ MEDIUM PRIORITY
**Location**: `tests/api_requirements_integration_test.rs`
**Missing Tests**:
- Null value handling in PATCH
- Partial update combinations
- Concurrent update scenarios
- Invalid field combinations

**Recommended**: Add to existing test file

### **Cache API Error Scenarios** ⚠️ LOW PRIORITY
**Location**: `tests/api_cache_integration_test.rs`
**Missing Tests**:
- Cache disabled scenarios
- High load scenarios
- Cache corruption

**Recommended**: Add to existing test file

## Test Coverage Statistics

### Current State
- **Total Test Files**: ~20+ (integration + unit)
- **Total Test Cases**: ~300+
- **API Endpoint Coverage**: ~85-90%
- **Service Layer Unit Tests**: ~10-15%
- **Helper Functions Tests**: ~0%
- **Auth Module Tests**: ~30%

### Target State
- **Service Layer Unit Tests**: 80%+ (currently ~10-15%)
- **Helper Functions Tests**: 80%+ (currently ~0%)
- **Auth Module Tests**: 80%+ (currently ~30%)
- **Edge Case Coverage**: Expand existing tests

## Implementation Recommendations

### Phase 1: Critical Service Layer Tests (High Priority)
1. `requirement_service.rs` - Filtering logic
2. `test_service.rs` - Complete TODO methods
3. `matrix_service.rs` - CSV export and linking
4. `requirement_analytics_service.rs` - SQL path testing

### Phase 2: Helper Functions and Auth (Medium Priority)
1. All `helper_functions/` modules
2. `auth/login.rs`, `auth/session.rs`, `auth/guards.rs`

### Phase 3: Remaining Services (Lower Priority)
1. Remaining service layer files
2. Route handler unit tests

### Phase 4: Edge Cases and Error Scenarios
1. Expand constraint violation tests
2. Add PATCH edge case tests
3. Add cache error scenario tests

## Testing Best Practices to Follow

1. **Use Mock Repositories**: Leverage `DieselRepoMock` for service layer tests
2. **Test Error Paths**: Don't just test happy paths
3. **Edge Cases**: Test boundary conditions, empty inputs, null values
4. **Integration vs Unit**: Use unit tests for business logic, integration tests for API contracts
5. **Test Documentation**: Document what each test verifies
6. **Test Organization**: Group related tests in modules

## Files Summary

### High Priority (Start Here)
1. `src/services/requirement_service.rs`
2. `src/services/test_service.rs`
3. `src/services/matrix_service.rs`
4. `src/helper_functions/` (all files)
5. `tests/api_constraint_violation_test.rs` (expand)

### Medium Priority
1. `src/services/decorated_requirement_service.rs`
2. `src/services/decorated_test_service.rs`
3. `src/services/log_service.rs` (expand)
4. `src/services/cache_service.rs`
5. `src/services/project_service.rs`
6. `src/services/user_service.rs`
7. `src/auth/login.rs`
8. `src/auth/session.rs`
9. `src/auth/guards.rs`

### Low Priority
1. `src/services/verification_service.rs`
2. `src/services/category_service.rs`
3. `src/services/applicability_service.rs`
4. `src/services/status_service.rs`
5. `src/auth/logout.rs`
6. `src/auth/errors.rs`
7. `src/routes/html/` (selective)

## Conclusion

The project has excellent API integration test coverage, but would benefit significantly from:
1. **Service layer unit tests** - Catch bugs earlier, test business logic in isolation
2. **Helper function tests** - Currently untested utility code
3. **Auth module expansion** - More comprehensive authentication/authorization testing
4. **Edge case expansion** - More thorough testing of error scenarios and boundary conditions

Focusing on the High Priority items would provide the most value for improving overall test coverage and code quality.
