//! Status enumeration definitions matching the database schema.
//!
//! These enums provide type-safe representations of requirement and test statuses,
//! ensuring consistency across the application and matching the exact definitions
//! in the database initialization script (`init_complete.sql`).
//!
//! # Coverage Calculation
//!
//! **Requirements Coverage**: Only requirements with status `Accepted` are considered
//! "verified" for coverage purposes. The coverage percentage is calculated as:
//! ```text
//! coverage_percent = (accepted_requirements / total_requirements) * 100
//! ```
//!
//! **Test Coverage**: Tests with status `Passed` are considered successful.
//! The pass rate is calculated as:
//! ```text
//! pass_rate_percent = (passed_tests / total_tests) * 100
//! ```
//!
//! # Status Definitions
//!
//! This module enforces the canonical status definitions from the database:
//!
//! **Requirement Statuses** (in lifecycle order):
//! - Draft (1): Initial state, editable by users
//! - Proposal (2): Awaiting approval, editable by users
//! - Accepted (3): Approved, counts toward coverage
//! - Rejected (4): Not accepted, needs revision
//! - Cancelled (5): Will not be implemented
//! - Finished (6): Completed
//!
//! **Test Statuses**:
//! - Passed (1): Test successful
//! - Failed (2): Test failed
//! - Pending (3): Awaiting execution
//! - In Progress (4): Currently executing

use serde::{Deserialize, Serialize};

/// Requirement status values as defined in the database.
///
/// These statuses represent the lifecycle states of a requirement:
/// - Draft: Initial state, still being edited
/// - Proposal: Awaiting approval
/// - Accepted: Approved and must be processed
/// - Rejected: Not accepted, needs revision
/// - Cancelled: Will not be implemented
/// - Finished: Completed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RequirementStatusEnum {
    Draft = 1,
    Proposal = 2,
    Accepted = 3,
    Rejected = 4,
    Cancelled = 5,
    Finished = 6,
}

impl RequirementStatusEnum {
    /// Returns the full title of the status.
    pub fn title(&self) -> &'static str {
        match self {
            RequirementStatusEnum::Draft => "Draft",
            RequirementStatusEnum::Proposal => "Proposal",
            RequirementStatusEnum::Accepted => "Accepted",
            RequirementStatusEnum::Rejected => "Rejected",
            RequirementStatusEnum::Cancelled => "Cancelled",
            RequirementStatusEnum::Finished => "Finished",
        }
    }

    /// Returns the description of the status.
    pub fn description(&self) -> &'static str {
        match self {
            RequirementStatusEnum::Draft => "The requirement is still being edited and developed",
            RequirementStatusEnum::Proposal => "The requirement is proposed and awaiting approval",
            RequirementStatusEnum::Accepted => "The requirement is accepted and must be processed",
            RequirementStatusEnum::Rejected => "The requirement is not accepted and needs revision",
            RequirementStatusEnum::Cancelled => {
                "The requirement is cancelled and will not be implemented"
            }
            RequirementStatusEnum::Finished => "The requirement is finished and completed",
        }
    }

    /// Returns the short name/abbreviation of the status.
    pub fn short_name(&self) -> &'static str {
        match self {
            RequirementStatusEnum::Draft => "Drf",
            RequirementStatusEnum::Proposal => "Pro",
            RequirementStatusEnum::Accepted => "Acc",
            RequirementStatusEnum::Rejected => "Rej",
            RequirementStatusEnum::Cancelled => "Can",
            RequirementStatusEnum::Finished => "Fsh",
        }
    }

    /// Convert from a database ID to the enum variant.
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            1 => Some(RequirementStatusEnum::Draft),
            2 => Some(RequirementStatusEnum::Proposal),
            3 => Some(RequirementStatusEnum::Accepted),
            4 => Some(RequirementStatusEnum::Rejected),
            5 => Some(RequirementStatusEnum::Cancelled),
            6 => Some(RequirementStatusEnum::Finished),
            _ => None,
        }
    }

    /// Convert from a title string to the enum variant.
    pub fn from_title(title: &str) -> Option<Self> {
        let normalized = title.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "draft" => Some(RequirementStatusEnum::Draft),
            "proposal" => Some(RequirementStatusEnum::Proposal),
            "accepted" => Some(RequirementStatusEnum::Accepted),
            "rejected" => Some(RequirementStatusEnum::Rejected),
            "cancelled" => Some(RequirementStatusEnum::Cancelled),
            "finished" => Some(RequirementStatusEnum::Finished),
            _ => None,
        }
    }

    /// Get the database ID for this status.
    pub fn id(&self) -> i32 {
        *self as i32
    }

    /// Returns all status variants in order.
    pub fn all() -> Vec<Self> {
        vec![
            RequirementStatusEnum::Draft,
            RequirementStatusEnum::Proposal,
            RequirementStatusEnum::Accepted,
            RequirementStatusEnum::Rejected,
            RequirementStatusEnum::Cancelled,
            RequirementStatusEnum::Finished,
        ]
    }

    /// Check if this status represents a verified/completed requirement.
    /// For coverage calculation, only "Accepted" requirements are considered verified.
    pub fn is_verified(&self) -> bool {
        matches!(self, RequirementStatusEnum::Accepted)
    }

    /// Check if the status can be edited by non-admin users.
    pub fn is_editable_by_user(&self) -> bool {
        matches!(
            self,
            RequirementStatusEnum::Draft | RequirementStatusEnum::Proposal
        )
    }
}

impl std::fmt::Display for RequirementStatusEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title())
    }
}

/// Test status values as defined in the database.
///
/// These statuses represent the execution state of a test:
/// - Passed: Test passed all criteria
/// - Failed: Test failed one or more criteria
/// - Pending: Test is pending execution
/// - In Progress: Test is currently being executed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestStatusEnum {
    Passed = 1,
    Failed = 2,
    Pending = 3,
    InProgress = 4,
}

impl TestStatusEnum {
    /// Returns the full title of the status.
    pub fn title(&self) -> &'static str {
        match self {
            TestStatusEnum::Passed => "Passed",
            TestStatusEnum::Failed => "Failed",
            TestStatusEnum::Pending => "Pending",
            TestStatusEnum::InProgress => "In Progress",
        }
    }

    /// Returns the description of the status.
    pub fn description(&self) -> &'static str {
        match self {
            TestStatusEnum::Passed => "The test has passed all criteria",
            TestStatusEnum::Failed => "The test has failed one or more criteria",
            TestStatusEnum::Pending => "The test is pending execution",
            TestStatusEnum::InProgress => "The test is currently being executed",
        }
    }

    /// Returns the short name/abbreviation of the status.
    pub fn short_name(&self) -> &'static str {
        match self {
            TestStatusEnum::Passed => "Pass",
            TestStatusEnum::Failed => "Fail",
            TestStatusEnum::Pending => "Pend",
            TestStatusEnum::InProgress => "Prog",
        }
    }

    /// Convert from a database ID to the enum variant.
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            1 => Some(TestStatusEnum::Passed),
            2 => Some(TestStatusEnum::Failed),
            3 => Some(TestStatusEnum::Pending),
            4 => Some(TestStatusEnum::InProgress),
            _ => None,
        }
    }

    /// Convert from a title string to the enum variant.
    pub fn from_title(title: &str) -> Option<Self> {
        let normalized = title.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "passed" => Some(TestStatusEnum::Passed),
            "failed" => Some(TestStatusEnum::Failed),
            "pending" => Some(TestStatusEnum::Pending),
            "in progress" => Some(TestStatusEnum::InProgress),
            _ => None,
        }
    }

    /// Get the database ID for this status.
    pub fn id(&self) -> i32 {
        *self as i32
    }

    /// Returns all status variants in order.
    pub fn all() -> Vec<Self> {
        vec![
            TestStatusEnum::Passed,
            TestStatusEnum::Failed,
            TestStatusEnum::Pending,
            TestStatusEnum::InProgress,
        ]
    }

    /// Check if this status represents a passed test (for coverage calculation).
    pub fn is_passed(&self) -> bool {
        matches!(self, TestStatusEnum::Passed)
    }

    /// Check if the test is in an active execution state.
    pub fn is_active(&self) -> bool {
        matches!(self, TestStatusEnum::InProgress | TestStatusEnum::Pending)
    }
}

impl std::fmt::Display for TestStatusEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_status_id_round_trip() {
        for status in RequirementStatusEnum::all() {
            let id = status.id();
            let recovered = RequirementStatusEnum::from_id(id);
            assert_eq!(Some(status), recovered);
        }
    }

    #[test]
    fn requirement_status_title_round_trip() {
        for status in RequirementStatusEnum::all() {
            let title = status.title();
            let recovered = RequirementStatusEnum::from_title(title);
            assert_eq!(Some(status), recovered);
        }
    }

    #[test]
    fn requirement_status_properties() {
        assert_eq!(RequirementStatusEnum::Draft.title(), "Draft");
        assert_eq!(RequirementStatusEnum::Draft.short_name(), "Drf");
        assert_eq!(RequirementStatusEnum::Draft.id(), 1);
        assert!(!RequirementStatusEnum::Draft.is_verified());
        assert!(RequirementStatusEnum::Draft.is_editable_by_user());

        assert_eq!(RequirementStatusEnum::Accepted.title(), "Accepted");
        assert!(RequirementStatusEnum::Accepted.is_verified());
        assert!(!RequirementStatusEnum::Accepted.is_editable_by_user());
    }

    #[test]
    fn test_status_id_round_trip() {
        for status in TestStatusEnum::all() {
            let id = status.id();
            let recovered = TestStatusEnum::from_id(id);
            assert_eq!(Some(status), recovered);
        }
    }

    #[test]
    fn test_status_title_round_trip() {
        for status in TestStatusEnum::all() {
            let title = status.title();
            let recovered = TestStatusEnum::from_title(title);
            assert_eq!(Some(status), recovered);
        }
    }

    #[test]
    fn test_status_properties() {
        assert_eq!(TestStatusEnum::Passed.title(), "Passed");
        assert_eq!(TestStatusEnum::Passed.short_name(), "Pass");
        assert_eq!(TestStatusEnum::Passed.id(), 1);
        assert!(TestStatusEnum::Passed.is_passed());

        assert_eq!(TestStatusEnum::InProgress.title(), "In Progress");
        assert!(TestStatusEnum::InProgress.is_active());
    }

    #[test]
    fn requirement_status_from_invalid_id() {
        assert_eq!(RequirementStatusEnum::from_id(999), None);
        assert_eq!(RequirementStatusEnum::from_id(0), None);
        assert_eq!(RequirementStatusEnum::from_id(-1), None);
    }

    #[test]
    fn test_status_from_invalid_title() {
        assert_eq!(TestStatusEnum::from_title("invalid"), None);
        assert_eq!(TestStatusEnum::from_title(""), None);
    }
}
