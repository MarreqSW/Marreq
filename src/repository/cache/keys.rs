use std::fmt::Display;

// Navigation and overview data
pub const PROJECTS_NAV: &str = "projects:nav";
pub const PROJECTS_ALL: &str = "projects:all";
pub const STATUS_ALL: &str = "status:all";
pub const REQUIREMENT_STATUS_ALL: &str = "requirement_status:all";
pub const TEST_STATUS_ALL: &str = "test_status:all";
pub const CATEGORIES_ALL: &str = "categories:all";
pub const APPLICABILITY_ALL: &str = "applicability:all";
pub const VERIFICATION_ALL: &str = "verification:all";
pub const USERS_ALL: &str = "users:all";

// Cache metadata
pub const CACHE_STATS: &str = "cache:stats";
pub const CACHE_HEALTH: &str = "cache:health";
pub const CACHE_PERFORMANCE: &str = "cache:performance";

// Global lists
pub const REQUIREMENTS_ALL: &str = "requirements:all";
pub const TESTS_ALL: &str = "tests:all";


/// Generic builder for "prefix[:project]:id" style keys.
pub trait Keyspace {
    const PREFIX: &'static str;

    #[inline]
    fn by_id<I: Display>(id: I) -> String {
        format!("{}:{}", Self::PREFIX, id)
    }

    #[inline]
    fn by_project<I: Display>(project_id: I) -> String {
        format!("{}:project:{}", Self::PREFIX, project_id)
    }

    #[inline]
    fn for_requirement<I: Display>(project_id: I) -> String {
        format!("{}:requirement:{}", Self::PREFIX, project_id)
    }

    #[inline]
    fn for_test<I: Display>(project_id: I) -> String {
        format!("{}:test:{}", Self::PREFIX, project_id)
    }

    #[inline]
    fn for_user<I: Display>(user_id: I) -> String {
        format!("{}:user:{}", Self::PREFIX, user_id)
    }
}

// Zero-sized marker types for each namespace
pub struct Projects;
pub struct Status;
pub struct Categories;
pub struct Applicability;
pub struct Verification;
pub struct Users;
pub struct Requirements;
pub struct RequirementTitle;
pub struct RequirementStatus;
pub struct Tests;
pub struct TestStatus;
pub struct Matrix;
pub struct LinkedTests;
pub struct LinkedRequirements;
pub struct ProjectMembers;

// Implement the prefix per namespace
impl Keyspace for Projects      { const PREFIX: &'static str = "project"; }
impl Keyspace for Status        { const PREFIX: &'static str = "status"; }
impl Keyspace for Categories    { const PREFIX: &'static str = "category"; }
impl Keyspace for Applicability { const PREFIX: &'static str = "applicability"; }
impl Keyspace for Verification  { const PREFIX: &'static str = "verification"; }
impl Keyspace for Users         { const PREFIX: &'static str = "user"; }
impl Keyspace for Tests         { const PREFIX: &'static str = "test"; }
impl Keyspace for TestStatus    { const PREFIX: &'static str = "test_status"; }
impl Keyspace for Matrix        { const PREFIX: &'static str = "matrix"; }
impl Keyspace for Requirements  { const PREFIX: &'static str = "requirement"; }
impl Keyspace for RequirementStatus { const PREFIX: &'static str = "requirement_status"; }
impl Keyspace for RequirementTitle   { const PREFIX: &'static str = "requirement_title"; }
impl Keyspace for LinkedRequirements { const PREFIX: &'static str = "linked_tests"; }
impl Keyspace for LinkedTests { const PREFIX: &'static str = "linked_requirements"; }
impl Keyspace for ProjectMembers { const PREFIX: &'static str = "project_member"; }

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! check_keyspace {
        ($ty:ty, $prefix:expr) => {
            assert_eq!(<$ty as Keyspace>::by_id(1), format!("{}:1", $prefix));
            assert_eq!(<$ty as Keyspace>::by_project("p"), format!("{}:project:p", $prefix));
            assert_eq!(<$ty as Keyspace>::for_requirement(2), format!("{}:requirement:2", $prefix));
            assert_eq!(<$ty as Keyspace>::for_test(3), format!("{}:test:3", $prefix));
        };
    }

    #[test]
    fn constants_are_correct() {
        assert_eq!(PROJECTS_NAV, "projects:nav");
        assert_eq!(PROJECTS_ALL, "projects:all");
        assert_eq!(STATUS_ALL, "status:all");
        assert_eq!(CATEGORIES_ALL, "categories:all");
        assert_eq!(APPLICABILITY_ALL, "applicability:all");
        assert_eq!(VERIFICATION_ALL, "verification:all");
        assert_eq!(USERS_ALL, "users:all");
        assert_eq!(CACHE_STATS, "cache:stats");
        assert_eq!(CACHE_HEALTH, "cache:health");
        assert_eq!(CACHE_PERFORMANCE, "cache:performance");
        assert_eq!(REQUIREMENTS_ALL, "requirements:all");
        assert_eq!(TESTS_ALL, "tests:all");
    }

    #[test]
    fn prefixes_generate_expected_keys() {
        check_keyspace!(Projects, "project");
        check_keyspace!(Status, "status");
        check_keyspace!(Categories, "category");
        check_keyspace!(Applicability, "applicability");
        check_keyspace!(Verification, "verification");
        check_keyspace!(Users, "user");
        check_keyspace!(Requirements, "requirement");
        check_keyspace!(RequirementTitle, "requirement_title");
        check_keyspace!(Tests, "test");
        check_keyspace!(TestStatus, "test_status");
        check_keyspace!(Matrix, "matrix");
        check_keyspace!(LinkedRequirements, "linked_tests");
        check_keyspace!(LinkedTests, "linked_requirements");
        check_keyspace!(ProjectMembers, "project_member");

        assert_eq!(ProjectMembers::for_user(5), "project_member:user:5");
    }
}
