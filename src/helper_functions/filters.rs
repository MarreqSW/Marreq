use crate::models::{Requirement, Test};

pub fn filter_requirements(
    requirements: Vec<Requirement>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
) -> Vec<Requirement> {
    let mut filtered_requirements: Vec<Requirement> = requirements
        .into_iter()
        .filter(|req| {
            let status_match = status_filter.map_or(true, |status_id| req.req_current_status == status_id);
            let verification_match = verification_filter.map_or(true, |verification_id| req.req_verification == verification_id);
            let category_match = category_filter.map_or(true, |category_id| req.req_category == category_id);
            status_match && verification_match && category_match
        })
        .collect();

    filtered_requirements.sort_by(|a, b| {
        match (a.req_reference.is_empty(), b.req_reference.is_empty()) {
            (false, false) => a.req_reference.cmp(&b.req_reference),
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            (true, true) => a.req_id.cmp(&b.req_id),
        }
    });

    filtered_requirements
}

pub fn filter_tests(
    tests: Vec<Test>,
    status_filter: Option<i32>,
    _verification_filter: Option<i32>,
    _category_filter: Option<i32>,
) -> Vec<Test> {
    tests
        .into_iter()
        .filter(|test| {
            let status_match = status_filter.map_or(true, |status_id| test.test_status == status_id);
            status_match
        })
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Requirement, Test};
    use chrono::NaiveDate;

    fn dummy_datetime() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap()
    }

    fn sample_requirement(
        id: i32,
        status: i32,
        verification: i32,
        category: i32,
        reference: &str,
    ) -> Requirement {
        Requirement {
            req_id: id,
            req_title: format!("Req {}", id),
            req_description: String::new(),
            req_verification: verification,
            req_current_status: status,
            req_author: 0,
            req_reviewer: 0,
            req_link: String::new(),
            req_reference: reference.to_string(),
            req_category: category,
            req_parent: 0,
            req_creation_date: dummy_datetime(),
            req_update_date: dummy_datetime(),
            req_deadline_date: dummy_datetime(),
            req_applicability: 0,
            req_justification: None,
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
        assert_eq!(filtered[0].req_id, 1);
        assert_eq!(filtered[1].req_id, 2);

        let filtered2 = filter_requirements(reqs.clone(), None, Some(1), Some(1));
        assert_eq!(filtered2.len(), 1);
        assert_eq!(filtered2[0].req_id, 1);

        let filtered3 = filter_requirements(reqs, None, None, None);
        assert_eq!(filtered3[0].req_id, 1);
        assert_eq!(filtered3[1].req_id, 3);
        assert_eq!(filtered3[2].req_id, 2);
    }

    #[test]
    fn filter_tests_filters_by_status() {
        let only_status1 = filter_tests(
            vec![
                Test { test_id: 1, test_name: "T1".into(), test_description: String::new(), test_source: String::new(), test_reference: "TEST-1".into(), test_status: 1, test_parent: 0, project_id: 0 },
                Test { test_id: 2, test_name: "T2".into(), test_description: String::new(), test_source: String::new(), test_reference: "TEST-2".into(), test_status: 2, test_parent: 0, project_id: 0 },
            ],
            Some(1), None, None
        );
        assert_eq!(only_status1.len(), 1);
        assert_eq!(only_status1[0].test_id, 1);

        let all = filter_tests(
            vec![
                Test { test_id: 1, test_name: "T1".into(), test_description: String::new(), test_source: String::new(), test_reference: "TEST-1".into(), test_status: 1, test_parent: 0, project_id: 0 },
                Test { test_id: 2, test_name: "T2".into(), test_description: String::new(), test_source: String::new(), test_reference: "TEST-2".into(), test_status: 2, test_parent: 0, project_id: 0 },
            ],
            None, None, None
        );
        assert_eq!(all.len(), 2);
    }
}


