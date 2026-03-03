// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Status enumeration definitions matching the database schema.
//!
//! These enums provide type-safe representations of requirement and test statuses,
//! ensuring consistency across the application and matching the exact definitions
//! in the database initialization script (`scripts/init_complete.sql`).
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
//! This module defines the canonical status variants. Statuses are stored
//! per-project in the database (tables `requirement_status`, `test_status`)
//! with auto-increment IDs that vary between projects. Business logic must
//! **never** rely on numeric IDs — use `from_title()` or `from_tag()` to
//! resolve a database row back to its semantic enum variant.
//!
//! **Requirement Statuses** (in lifecycle order):
//! - Draft: Initial state, editable by users
//! - Proposal: Awaiting approval, editable by users
//! - Accepted: Approved, counts toward coverage
//! - Rejected: Not accepted, needs revision
//! - Cancelled: Will not be implemented
//! - Finished: Completed
//!
//! **Test Statuses**:
//! - Passed: Test successful
//! - Failed: Test failed
//! - Pending: Awaiting execution
//! - In Progress: Currently executing
//!
//! **Project Statuses**:
//! - Active: Project is currently active and in progress
//! - Completed: Project has been completed
//! - OnHold: Project is temporarily paused
//! - Cancelled: Project has been cancelled

use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, Output, ToSql};
use diesel::{AsExpression, FromSqlRow};
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
    Draft,
    Proposal,
    Accepted,
    Rejected,
    Cancelled,
    Finished,
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

    /// Convert from a tag string (short name) to the enum variant.
    pub fn from_tag(tag: &str) -> Option<Self> {
        let normalized = tag.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "drf" => Some(RequirementStatusEnum::Draft),
            "pro" => Some(RequirementStatusEnum::Proposal),
            "acc" => Some(RequirementStatusEnum::Accepted),
            "rej" => Some(RequirementStatusEnum::Rejected),
            "can" => Some(RequirementStatusEnum::Cancelled),
            "fsh" => Some(RequirementStatusEnum::Finished),
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

    /// Canonical ordering for display (stable across projects, not a DB id).
    pub fn canonical_order(&self) -> i32 {
        match self {
            RequirementStatusEnum::Draft => 0,
            RequirementStatusEnum::Proposal => 1,
            RequirementStatusEnum::Accepted => 2,
            RequirementStatusEnum::Rejected => 3,
            RequirementStatusEnum::Cancelled => 4,
            RequirementStatusEnum::Finished => 5,
        }
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
    Passed,
    Failed,
    Pending,
    InProgress,
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

    /// Convert from a tag string (short name) to the enum variant.
    pub fn from_tag(tag: &str) -> Option<Self> {
        let normalized = tag.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "pass" => Some(TestStatusEnum::Passed),
            "fail" => Some(TestStatusEnum::Failed),
            "pend" => Some(TestStatusEnum::Pending),
            "prog" => Some(TestStatusEnum::InProgress),
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

    /// Canonical ordering for display (stable across projects, not a DB id).
    pub fn canonical_order(&self) -> i32 {
        match self {
            TestStatusEnum::Passed => 0,
            TestStatusEnum::Failed => 1,
            TestStatusEnum::Pending => 2,
            TestStatusEnum::InProgress => 3,
        }
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

/// Approval workflow state for requirement versions (stored in requirement_versions.approval_state).
///
/// Valid transitions (enforced at API layer):
/// - draft → reviewed
/// - reviewed → approved
/// - (no backwards transitions by default)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ApprovalState {
    #[default]
    Draft,
    Reviewed,
    Approved,
}

impl ApprovalState {
    /// Database string (lowercase).
    pub fn to_db_string(&self) -> &'static str {
        match self {
            ApprovalState::Draft => "draft",
            ApprovalState::Reviewed => "reviewed",
            ApprovalState::Approved => "approved",
        }
    }

    /// Parse from database or API string.
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "draft" => Some(ApprovalState::Draft),
            "reviewed" => Some(ApprovalState::Reviewed),
            "approved" => Some(ApprovalState::Approved),
            _ => None,
        }
    }

    /// Whether a transition from `self` to `target` is allowed.
    /// Same-state (idempotent) transitions are allowed so stale UI or double-submit does not error.
    pub fn can_transition_to(&self, target: ApprovalState) -> bool {
        if *self == target {
            return true;
        }
        matches!(
            (self, target),
            (ApprovalState::Draft, ApprovalState::Reviewed)
                | (ApprovalState::Reviewed, ApprovalState::Approved)
        )
    }

    /// Whether this state is considered approved for baselines.
    pub fn is_approved(&self) -> bool {
        matches!(self, ApprovalState::Approved)
    }
}

impl std::fmt::Display for ApprovalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_db_string())
    }
}

/// Project status values representing the lifecycle states of a project.
///
/// These statuses are stored directly in the database as VARCHAR text values
/// but are represented as a type-safe enum in Rust code. Diesel automatically
/// handles the conversion between the database string and the enum.
///
/// - Active: Project is currently active and in progress
/// - Completed: Project has been completed
/// - OnHold: Project is temporarily paused
/// - Cancelled: Project has been cancelled
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    AsExpression,
    FromSqlRow,
    Default,
)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum ProjectStatus {
    #[default]
    Active,
    Completed,
    OnHold,
    Cancelled,
}

impl ProjectStatus {
    /// Returns the full title of the status.
    pub fn title(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "Active",
            ProjectStatus::Completed => "Completed",
            ProjectStatus::OnHold => "On Hold",
            ProjectStatus::Cancelled => "Cancelled",
        }
    }

    /// Returns the description of the status.
    pub fn description(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "The project is currently active and in progress",
            ProjectStatus::Completed => "The project has been completed",
            ProjectStatus::OnHold => "The project is temporarily on hold",
            ProjectStatus::Cancelled => "The project has been cancelled",
        }
    }

    /// Returns the normalized lowercase string representation for database storage.
    pub fn to_db_string(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Completed => "completed",
            ProjectStatus::OnHold => "on_hold",
            ProjectStatus::Cancelled => "cancelled",
        }
    }

    /// Convert from a database string to the enum variant.
    pub fn from_db_string(s: &str) -> Option<Self> {
        let normalized = s.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "active" => Some(ProjectStatus::Active),
            "completed" => Some(ProjectStatus::Completed),
            "on_hold" | "on hold" | "onhold" => Some(ProjectStatus::OnHold),
            "cancelled" | "canceled" => Some(ProjectStatus::Cancelled),
            _ => None,
        }
    }

    /// Convert from a title string to the enum variant.
    pub fn from_title(title: &str) -> Option<Self> {
        Self::from_db_string(title)
    }

    /// Returns all status variants in order.
    pub fn all() -> Vec<Self> {
        vec![
            ProjectStatus::Active,
            ProjectStatus::Completed,
            ProjectStatus::OnHold,
            ProjectStatus::Cancelled,
        ]
    }

    /// Check if the project is in an active state.
    pub fn is_active(&self) -> bool {
        matches!(self, ProjectStatus::Active)
    }

    /// Check if the project is completed or cancelled.
    pub fn is_finished(&self) -> bool {
        matches!(self, ProjectStatus::Completed | ProjectStatus::Cancelled)
    }

    /// Get the CSS badge class for this status (for HTML templates).
    pub fn badge_class(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "bg-success",
            ProjectStatus::Completed => "bg-info",
            ProjectStatus::OnHold => "bg-warning",
            ProjectStatus::Cancelled => "bg-secondary",
        }
    }
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title())
    }
}

// Diesel FromSql implementation for ProjectStatus
impl FromSql<diesel::sql_types::Text, Pg> for ProjectStatus {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let value = <String as FromSql<diesel::sql_types::Text, Pg>>::from_sql(bytes)?;
        Self::from_db_string(&value)
            .ok_or_else(|| format!("Unrecognized project status: {}", value).into())
    }
}

// Diesel ToSql implementation for ProjectStatus
impl ToSql<diesel::sql_types::Text, Pg> for ProjectStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let value = self.to_db_string();
        <str as ToSql<diesel::sql_types::Text, Pg>>::to_sql(value, out)
    }
}

// Rocket FromFormField implementation for ProjectStatus
impl<'r> rocket::form::FromFormField<'r> for ProjectStatus {
    fn from_value(field: rocket::form::ValueField<'r>) -> rocket::form::Result<'r, Self> {
        Self::from_db_string(field.value).ok_or_else(|| {
            rocket::form::Error::validation(format!("Invalid project status: {}", field.value))
                .into()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_status_tag_round_trip() {
        for status in RequirementStatusEnum::all() {
            let tag = status.short_name();
            let recovered = RequirementStatusEnum::from_tag(tag);
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
        assert_eq!(RequirementStatusEnum::Draft.canonical_order(), 0);
        assert!(!RequirementStatusEnum::Draft.is_verified());
        assert!(RequirementStatusEnum::Draft.is_editable_by_user());

        assert_eq!(RequirementStatusEnum::Accepted.title(), "Accepted");
        assert!(RequirementStatusEnum::Accepted.is_verified());
        assert!(!RequirementStatusEnum::Accepted.is_editable_by_user());
    }

    #[test]
    fn test_status_tag_round_trip() {
        for status in TestStatusEnum::all() {
            let tag = status.short_name();
            let recovered = TestStatusEnum::from_tag(tag);
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
        assert_eq!(TestStatusEnum::Passed.canonical_order(), 0);
        assert!(TestStatusEnum::Passed.is_passed());

        assert_eq!(TestStatusEnum::InProgress.title(), "In Progress");
        assert!(TestStatusEnum::InProgress.is_active());
    }

    #[test]
    fn requirement_status_from_invalid_tag() {
        assert_eq!(RequirementStatusEnum::from_tag("xyz"), None);
        assert_eq!(RequirementStatusEnum::from_tag(""), None);
        assert_eq!(RequirementStatusEnum::from_tag("   "), None);
    }

    #[test]
    fn test_status_from_invalid_title() {
        assert_eq!(TestStatusEnum::from_title("invalid"), None);
        assert_eq!(TestStatusEnum::from_title(""), None);
    }

    #[test]
    fn project_status_db_string_round_trip() {
        for status in ProjectStatus::all() {
            let db_str = status.to_db_string();
            let recovered = ProjectStatus::from_db_string(db_str);
            assert_eq!(Some(status), recovered);
        }
    }

    #[test]
    fn project_status_title_round_trip() {
        for status in ProjectStatus::all() {
            let title = status.title();
            let recovered = ProjectStatus::from_title(title);
            assert_eq!(Some(status), recovered);
        }
    }

    #[test]
    fn project_status_properties() {
        assert_eq!(ProjectStatus::Active.title(), "Active");
        assert_eq!(ProjectStatus::Active.to_db_string(), "active");
        assert!(ProjectStatus::Active.is_active());
        assert!(!ProjectStatus::Active.is_finished());

        assert_eq!(ProjectStatus::Completed.title(), "Completed");
        assert!(!ProjectStatus::Completed.is_active());
        assert!(ProjectStatus::Completed.is_finished());

        assert_eq!(ProjectStatus::OnHold.title(), "On Hold");
        assert_eq!(ProjectStatus::OnHold.to_db_string(), "on_hold");
        assert!(!ProjectStatus::OnHold.is_active());

        assert_eq!(ProjectStatus::Cancelled.badge_class(), "bg-secondary");
    }

    #[test]
    fn project_status_from_db_string_variations() {
        // Test case insensitivity
        assert_eq!(
            ProjectStatus::from_db_string("ACTIVE"),
            Some(ProjectStatus::Active)
        );
        assert_eq!(
            ProjectStatus::from_db_string("Active"),
            Some(ProjectStatus::Active)
        );

        // Test "on hold" variations
        assert_eq!(
            ProjectStatus::from_db_string("on hold"),
            Some(ProjectStatus::OnHold)
        );
        assert_eq!(
            ProjectStatus::from_db_string("On Hold"),
            Some(ProjectStatus::OnHold)
        );
        assert_eq!(
            ProjectStatus::from_db_string("onhold"),
            Some(ProjectStatus::OnHold)
        );
        assert_eq!(
            ProjectStatus::from_db_string("on_hold"),
            Some(ProjectStatus::OnHold)
        );

        // Test cancelled/canceled
        assert_eq!(
            ProjectStatus::from_db_string("cancelled"),
            Some(ProjectStatus::Cancelled)
        );
        assert_eq!(
            ProjectStatus::from_db_string("canceled"),
            Some(ProjectStatus::Cancelled)
        );
    }

    #[test]
    fn project_status_from_invalid() {
        assert_eq!(ProjectStatus::from_db_string("invalid"), None);
        assert_eq!(ProjectStatus::from_db_string(""), None);
        assert_eq!(ProjectStatus::from_db_string("pending"), None);
    }
}
