use crate::models::{Requirement, TestCase};

pub fn filter_requirements(
    requirements: Vec<Requirement>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
) -> Vec<Requirement> {
    let mut filtered_requirements: Vec<Requirement> = requirements
        .into_iter()
        .filter(|req| {
            let status_match = status_filter.is_none_or(|status_id| req.status_id == status_id);
            let verification_match =
                verification_filter.is_none_or(|id| req.verification_method_id == id);
            let category_match =
                category_filter.is_none_or(|category_id| req.category_id == category_id);
            status_match && verification_match && category_match
        })
        .collect();

    filtered_requirements.sort_by(|a, b| {
        match (a.reference_code.is_empty(), b.reference_code.is_empty()) {
            (false, false) => a.reference_code.cmp(&b.reference_code),
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            (true, true) => a.id.cmp(&b.id),
        }
    });

    filtered_requirements
}

pub fn filter_tests(
    tests: Vec<TestCase>,
    status_filter: Option<i32>,
    _verification_filter: Option<i32>,
    _category_filter: Option<i32>,
) -> Vec<TestCase> {
    tests
        .into_iter()
        .filter(|test| status_filter.is_none_or(|status_id| test.status_id == status_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Requirement, TestCase};
    use chrono::NaiveDate;

    fn dummy_datetime() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn sample_requirement(
        id: i32,
        status: i32,
        verification: i32,
        category: i32,
        reference: &str,
    ) -> Requirement {
        Requirement {
            id,
            title: format!("Req {}", id),
            description: String::new(),
            verification_method_id: verification,
            status_id: status,
            author_id: 0,
            reviewer_id: 0,
            reference_code: reference.to_string(),
            category_id: category,
            parent_id: None,
            creation_date: dummy_datetime(),
            update_date: dummy_datetime(),
            deadline_date: Some(dummy_datetime()),
            applicability_id: 0,
            justification: None,
            project_id: 0,
        }
    }

    #[test]
    fn filter_requirements_filters_and_sorts() {
        let reqs = vec![
            sample_requirement(1, 1, 1, 1, "REF-A"),
            sample_requirement(2, 1, 2, 1, ""),
            sample_requirement(3, 2, 1, 2, "REF-B"),
        ];

        let filtered = filter_requirements(reqs.clone(), Some(1), None, None);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].id, 1);
        assert_eq!(filtered[1].id, 2);

        let filtered2 = filter_requirements(reqs.clone(), None, Some(1), Some(1));
        assert_eq!(filtered2.len(), 1);
        assert_eq!(filtered2[0].id, 1);

        let filtered3 = filter_requirements(reqs, None, None, None);
        assert_eq!(filtered3[0].id, 1);
        assert_eq!(filtered3[1].id, 3);
        assert_eq!(filtered3[2].id, 2);
    }

    #[test]
    fn filter_tests_filters_by_status() {
        let only_status1 = filter_tests(
            vec![
                TestCase {
                    id: 1,
                    name: "T1".into(),
                    description: String::new(),
                    source: String::new(),
                    reference_code: "TEST-1".into(),
                    status_id: 1,
                    parent_id: None,
                    project_id: 0,
                },
                TestCase {
                    id: 2,
                    name: "T2".into(),
                    description: String::new(),
                    source: String::new(),
                    reference_code: "TEST-2".into(),
                    status_id: 2,
                    parent_id: None,
                    project_id: 0,
                },
            ],
            Some(1),
            None,
            None,
        );
        assert_eq!(only_status1.len(), 1);
        assert_eq!(only_status1[0].id, 1);

        let all = filter_tests(
            vec![
                TestCase {
                    id: 1,
                    name: "T1".into(),
                    description: String::new(),
                    source: String::new(),
                    reference_code: "TEST-1".into(),
                    status_id: 1,
                    parent_id: None,
                    project_id: 0,
                },
                TestCase {
                    id: 2,
                    name: "T2".into(),
                    description: String::new(),
                    source: String::new(),
                    reference_code: "TEST-2".into(),
                    status_id: 2,
                    parent_id: None,
                    project_id: 0,
                },
            ],
            None,
            None,
            None,
        );
        assert_eq!(all.len(), 2);
    }
}
