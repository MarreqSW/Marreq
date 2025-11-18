# Status Enums Implementation Summary

## Overview

This document summarizes the implementation of canonical status enums for Requirements and Tests, ensuring consistency between the database schema and application code.

## Status Definitions

### Requirement Statuses

Requirements have 6 possible statuses:

| ID | Title      | Short | Description                                          |
|----|------------|-------|------------------------------------------------------|
| 1  | Draft      | Drf   | Still being edited and developed                     |
| 2  | Proposal   | Pro   | Proposed and awaiting approval                       |
| 3  | Accepted   | Acc   | Accepted and must be processed                       |
| 4  | Rejected   | Rej   | Not accepted, needs revision                         |
| 5  | Cancelled  | Can   | Cancelled, will not be implemented                   |
| 6  | Finished   | Fsh   | Finished and completed                               |

### Test Statuses

Tests have 4 possible statuses:

| ID | Title       | Short | Description                           |
|----|-------------|-------|---------------------------------------|
| 1  | Passed      | Pass  | Test passed all criteria              |
| 2  | Failed      | Fail  | Test failed one or more criteria      |
| 3  | Pending     | Pend  | Test is pending execution             |
| 4  | In Progress | Prog  | Test is currently being executed      |

## Coverage Calculation

### Requirement Coverage

**Definition**: Only requirements with status = `Accepted` (ID 3) count as "verified" for coverage purposes.

**Formula**:
```
coverage_verified = count(requirements where status = Accepted)
coverage_percent = (coverage_verified / total_requirements) * 100
```

**Rationale**: "Accepted" status means the requirement has been formally approved and must be processed. Other statuses represent work-in-progress or terminal states that don't contribute to coverage.

### Test Pass Rate

**Definition**: Tests with status = `Passed` (ID 1) count as successful.

**Formula**:
```
passed_tests = count(tests where status = Passed)
pass_rate_percent = (passed_tests / total_tests) * 100
```

## Implementation Files

### Core Enum Definition

**File**: `src/status_enums.rs`

Defines:
- `RequirementStatusEnum` with all 6 statuses
- `TestStatusEnum` with all 4 statuses
- Conversion methods: `from_id()`, `from_title()`, `id()`
- Helper methods: `is_verified()`, `is_editable_by_user()`, `is_passed()`, `is_active()`
- Comprehensive unit tests

### Backend Integration

**Modified Files**:
1. **`src/lib.rs`**: Added `pub mod status_enums;`

2. **`src/services/requirement_analytics_service.rs`**:
   - Uses `RequirementStatusEnum::from_title()` to identify status types
   - Documents coverage calculation in struct comments
   - Ensures only "Accepted" requirements count toward coverage

3. **`src/routes/html/project/requirements.rs`**:
   - Uses `RequirementStatusEnum::is_editable_by_user()` for delete permissions
   - Uses `RequirementStatusEnum::Draft.id()` as default for new requirements
   - Uses `TestStatusEnum::from_title()` to count passed/failed tests

4. **`src/routes/html/project/tests.rs`**:
   - Uses `TestStatusEnum` constants for metrics calculation
   - Uses enum-based permission checking for delete operations
   - Properly tracks passed/failed/pending/in-progress counts

### Frontend Integration

**Modified Template Files**:
1. **`templates/test.html.hbs`**: Changed status check from `(eq test_status_id 1)` to `(eq status_id "Passed")`
2. **`templates/tests/_table_view.html.hbs`**: Changed status check to string comparison
3. **`templates/tests/_card_view.html.hbs`**: Changed status check to string comparison

These changes ensure delete buttons only appear for "Passed" or "Failed" tests (or for admins).

## Usage Examples

### In Rust Code

```rust
use crate::status_enums::{RequirementStatusEnum, TestStatusEnum};

// Check if requirement is editable
let status = RequirementStatusEnum::from_id(req.status_id);
if status.map(|s| s.is_editable_by_user()).unwrap_or(false) {
    // Allow editing
}

// Check if requirement counts toward coverage
if RequirementStatusEnum::from_id(id) == Some(RequirementStatusEnum::Accepted) {
    // Counts as verified
}

// Get status properties
let status = RequirementStatusEnum::Accepted;
println!("Title: {}", status.title());           // "Accepted"
println!("Short: {}", status.short_name());      // "Acc"
println!("ID: {}", status.id());                 // 3
println!("Description: {}", status.description());
```

### In Templates

```handlebars
{{!-- Check test status by string --}}
{{#if (eq status_id "Passed")}}
    <span class="badge badge-success">Test Passed</span>
{{/if}}

{{!-- Permission check for delete --}}
{{#if (or (eq status_id "Passed") (eq status_id "Failed") user.is_admin)}}
    <button data-action="delete-test">Delete</button>
{{/if}}
```

## Testing

All enum conversions and helper methods are covered by unit tests in `src/status_enums.rs`:

```bash
cargo test --lib status_enums
```

Expected output: 8 tests pass

## Benefits

1. **Type Safety**: Compile-time checking prevents invalid status values
2. **Single Source of Truth**: Enum definitions match database exactly
3. **Maintainability**: Changes to status logic happen in one place
4. **Documentation**: Clear rules for coverage and permissions
5. **Consistency**: Frontend and backend use same status definitions

## Migration Notes

- No database migration required (enums match existing schema)
- All existing numeric comparisons have been replaced with enum-based checks
- Templates now compare status strings instead of IDs for clarity
- Coverage calculation unchanged (still counts only "Accepted" requirements)
