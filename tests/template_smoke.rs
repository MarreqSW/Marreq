use handlebars::handlebars_helper;
use handlebars::Handlebars;
#[allow(unused_imports)]
use handlebars::JsonValue as Json;
use serde_json::json;
use std::fs;

handlebars_helper!(eq_helper: |a: Json, b: Json| a == b);
handlebars_helper!(ne_helper: |a: Json, b: Json| a != b);

fn register_partials(hb: &mut Handlebars) {
    let entries = [
        ("layout", "templates/layout.html.hbs"),
        ("partials/header", "templates/partials/header.html.hbs"),
        ("partials/sidebar", "templates/partials/sidebar.html.hbs"),
        (
            "partials/user_menu",
            "templates/partials/user_menu.html.hbs",
        ),
        ("partials/nav", "templates/partials/nav.html.hbs"),
        (
            "partials/project_card",
            "templates/partials/project_card.html.hbs",
        ),
        (
            "partials/quick_action_card",
            "templates/partials/quick_action_card.html.hbs",
        ),
        (
            "partials/metrics_card",
            "templates/partials/metrics_card.html.hbs",
        ),
        (
            "partials/filters_form",
            "templates/partials/filters_form.html.hbs",
        ),
        ("partials/modals", "templates/partials/modals.html.hbs"),
        ("footer", "templates/footer.html.hbs"),
        ("requirement", "templates/requirements/requirement.html.hbs"),
    ("requirements/_page_header", "templates/requirements/_page_header.html.hbs"),
    ("requirements/_filter_controls", "templates/requirements/_filter_controls.html.hbs"),
    ("requirements/_table_view", "templates/requirements/_table_view.html.hbs"),
    ("requirements/_card_view", "templates/requirements/_card_view.html.hbs"),
    ("requirements/_tree_view", "templates/requirements/_tree_view.html.hbs"),
    ("requirements/_tree_node", "templates/requirements/_tree_node.html.hbs"),
    ("requirements/_tree_child_node", "templates/requirements/_tree_child_node.html.hbs"),
    ("requirements/_empty_state", "templates/requirements/_empty_state.html.hbs"),
    ("requirements/_metrics_section", "templates/requirements/_metrics_section.html.hbs"),
    ("requirements/_view_controls", "templates/requirements/_view_controls.html.hbs"),
    ];

    for (name, path) in entries {
        let contents = fs::read_to_string(path).expect("read partial");
        hb.register_partial(name, contents)
            .unwrap_or_else(|err| panic!("register partial {}: {}", name, err));
    }
}

fn register_templates(hb: &mut Handlebars) {
    let entries = [
        ("index", "templates/index.html.hbs"),
        ("requirements", "templates/requirements/requirements.html.hbs"),
        ("requirements_table", "templates/requirements/_table_view.html.hbs"),
        ("tests_table", "templates/tests_table.html.hbs"),
        ("projects", "templates/projects.html.hbs"),
        ("project", "templates/project.html.hbs"),
    ];

    for (name, path) in entries {
        let contents = fs::read_to_string(path).expect("read template");
        hb.register_template_string(name, contents)
            .unwrap_or_else(|err| panic!("register template {}: {}", name, err));
    }
}

fn sample_user() -> serde_json::Value {
    json!({
        "user_id": 1,
        "user_name": "Alice Example",
        "user_username": "alice",
        "user_email": "alice@example.com",
        "is_admin": true
    })
}

#[test]
fn render_core_templates() {
    let mut handlebars = Handlebars::new();
    handlebars
        .render_template("{{#if cond}}yes{{else}}no{{/if}}", &json!({"cond": true}))
        .expect("basic if helper");
    handlebars.register_helper("eq", Box::new(eq_helper));
    handlebars.register_helper("ne", Box::new(ne_helper));
    register_partials(&mut handlebars);
    register_templates(&mut handlebars);

    let user = sample_user();
    let project = json!({
        "project_id": 1,
        "project_name": "Demo Project",
        "project_description": "Sample project for smoke tests",
        "project_status": "Active",
        "project_status_badge": "active",
        "project_initial": "D",
        "project_owner_id": 1,
        "project_owner_name": "Alice Example",
        "role_label": "Owner"
    });

    handlebars
        .render(
            "index",
            &json!({
                "user": user,
                "projects": [project.clone()]
            }),
        )
        .expect("render index.html.hbs");

    handlebars
        .render(
            "requirements",
            &json!({
                "user": user,
                "selected_project_id": 1,
                "statuses": [
                    { "req_st_id": 1, "req_st_title": "Draft" }
                ],
                "verifications": [
                    { "verification_id": 1, "verification_name": "Analysis" }
                ],
                "categories": [
                    { "cat_id": 1, "cat_title": "General" }
                ],
                "requirements": [],
                "current_status_filter": "",
                "current_verification_filter": "",
                "current_category_filter": ""
            }),
        )
        .expect("render requirements.html.hbs");

    handlebars
        .render(
            "requirements_table",
            &json!({
                "user": user,
                "selected_project_id": 1,
                "statuses": [
                    { "req_st_id": 1, "req_st_title": "Draft" }
                ],
                "verifications": [
                    { "verification_id": 1, "verification_name": "Analysis" }
                ],
                "categories": [
                    { "cat_id": 1, "cat_title": "General" }
                ],
                "users": [
                    { "user_id": 1, "user_name": "Alice Example" }
                ],
                "requirements": [
                    {
                        "req_id": 101,
                        "project_id": 1,
                        "req_title": "Sample requirement",
                        "req_reference": "REQ-101",
                        "req_category_id": 1,
                        "req_current_status_id": 1,
                        "req_verification_id": 1,
                        "req_author_id": 1,
                        "req_reviewer_id": 1,
                        "req_creation_date": "2024-01-01",
                        "req_deadline_date": "2024-02-01"
                    }
                ],
                "current_status_filter": "",
                "current_verification_filter": "",
                "current_category_filter": ""
            }),
        )
        .expect("render requirements_table.html.hbs");

    handlebars
        .render(
            "tests_table",
            &json!({
                "user": user,
                "selected_project_id": 1,
                "statuses": [
                    { "test_st_id": 1, "test_st_title": "Draft" }
                ],
                "verifications": [
                    { "verification_id": 1, "verification_name": "Analysis" }
                ],
                "categories": [
                    { "cat_id": 1, "cat_title": "General" }
                ],
                "tests": [
                    {
                        "test_id": 201,
                        "test_name": "Sample Test",
                        "test_reference": "TST-201",
                        "test_description": "Smoke test entry",
                        "test_status_id": 1,
                        "test_source": "Manual",
                        "test_parent_id": 0
                    }
                ],
                "current_status_filter": "",
                "current_verification_filter": "",
                "current_category_filter": ""
            }),
        )
        .expect("render tests_table.html.hbs");

    handlebars
        .render(
            "projects",
            &json!({
                "user": user,
                "projects": [project.clone()]
            }),
        )
        .expect("render projects.html.hbs");

    handlebars
        .render(
            "project",
            &json!({
                "user": user,
                "selected_project_id": 1,
                "selected_project_name": "Demo Project",
                "requirements_count": 10,
                "tests_count": 5
            }),
        )
        .expect("render project.html.hbs");
}
