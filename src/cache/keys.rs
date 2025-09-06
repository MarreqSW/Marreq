use std::fmt::Display;

// Navigation and overview data
pub const PROJECTS_NAV: &str = "projects:nav";
pub const PROJECTS_ALL: &str = "projects:all";
pub const STATUS_ALL: &str = "status:all";
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
pub struct Tests;
pub struct TestStatus;
pub struct Matrix;
pub struct LinkedTests;
pub struct LinkedRequirements;

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
impl Keyspace for RequirementTitle   { const PREFIX: &'static str = "requirement_title"; }
impl Keyspace for LinkedRequirements { const PREFIX: &'static str = "linked_tests"; }
impl Keyspace for LinkedTests { const PREFIX: &'static str = "linked_requirements"; }
