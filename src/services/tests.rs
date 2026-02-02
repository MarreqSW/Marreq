#[cfg(test)]
mod tests {
    use super::super::base_service::{
        check_project_permission, serialize_for_logging, validate_entity_access,
    };
    use crate::models::{NewRequirement, Requirement, User};
    use chrono::{NaiveDate, NaiveDateTime};

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn sample_requirement() -> Requirement {
        Requirement {
            id: 1,
            title: "Sample".into(),
            description: "Description".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "REQ-1".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: None,
            project_id: 1,
        }
    }

    fn sample_requirement_payload() -> NewRequirement {
        NewRequirement {
            id: Some(1),
            title: "Sample".into(),
            description: "Description".into(),
            author_id: 1,
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "  REQ-123  ".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        }
    }

    fn sample_user() -> User {
        User {
            id: 1,
            username: "tester".into(),
            name: "Tester".into(),
            email: "tester@example.com".into(),
            creation_date: timestamp(),
            last_login: timestamp(),
            password_hash: "secret".into(),
            is_admin: false,
        }
    }

    #[test]
    fn serialize_requirement_for_logging() {
        let json = serialize_for_logging(&sample_requirement()).unwrap();
        assert!(json.contains("Sample"));
    }

    #[test]
    fn serialize_payload_for_logging() {
        let json = serialize_for_logging(&sample_requirement_payload()).unwrap();
        assert!(json.contains("REQ-1"));
    }

    #[test]
    fn admin_user_has_access() {
        let mut admin = sample_user();
        admin.is_admin = true;
        assert!(check_project_permission(&admin, 42).is_ok());
    }

    #[test]
    fn regular_user_validation_passes() {
        let user = sample_user();
        assert!(validate_entity_access(&user, 1).is_ok());
    }
}
