// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::models::{Category, RequirementStatus};
use printpdf::{
    BuiltinFont, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, PdfWarnMsg, Point, Pt, TextItem,
};
use std::collections::HashMap;

pub struct Metrics {
    pub categories: Vec<Category>,
    pub statuses: Vec<RequirementStatus>,
    pub users_len: usize,

    // totals
    pub total_requirements: usize,
    pub total_tests: usize,
    pub total_categories: usize,

    // groupings (i32 to match generate_pdf_content)
    pub requirements_by_status: HashMap<String, i32>,
    pub tests_by_status: HashMap<String, i32>,
    pub requirements_by_category: HashMap<String, i32>,

    // coverage
    pub covered_requirements: usize,
    pub total_links: usize,
    pub coverage_percentage: f64,
    pub avg_tests_per_requirement: f64,

    // placeholders
    pub recent_requirements: usize,
    pub recent_tests: usize,
}

pub fn generate_pdf_content(metrics: &Metrics) -> String {
    let mut content = String::new();

    // Header
    content.push_str("
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset='utf-8'>
        <title>Marreq - Project Report</title>
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; }
            .header { text-align: center; border-bottom: 2px solid #333; padding-bottom: 20px; margin-bottom: 30px; }
            .section { margin-bottom: 30px; }
            .section h2 { color: #2c3e50; border-bottom: 1px solid #bdc3c7; padding-bottom: 10px; }
            .metric-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin: 20px 0; }
            .metric-card { background: #f8f9fa; border: 1px solid #dee2e6; border-radius: 8px; padding: 20px; text-align: center; }
            .metric-value { font-size: 2em; font-weight: bold; color: #007bff; margin-bottom: 10px; }
            .metric-label { color: #6c757d; font-size: 0.9em; }
            .chart-container { margin: 20px 0; }
            .status-item { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #eee; }
            .status-name { font-weight: bold; }
            .status-count { color: #007bff; }
            .coverage-bar { background: #e9ecef; height: 20px; border-radius: 10px; overflow: hidden; margin: 10px 0; }
            .coverage-fill { background: #28a745; height: 100%; transition: width 0.3s ease; }
            .footer { margin-top: 40px; text-align: center; color: #6c757d; font-size: 0.8em; }
        </style>
    </head>
    <body>
        <div class='header'>
            <h1>Marreq Project Report</h1>
            <p>Generated on: ");

    content.push_str(
        &chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
    );
    content.push_str(
        "</p>
        </div>
        
        <div class='section'>
            <h2>Executive Summary</h2>
            <div class='metric-grid'>
                <div class='metric-card'>
                    <div class='metric-value'>",
    );
    content.push_str(&metrics.total_requirements.to_string());
    content.push_str(
        "</div>
                    <div class='metric-label'>Total Requirements</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>",
    );
    content.push_str(&metrics.total_tests.to_string());
    content.push_str(
        "</div>
                    <div class='metric-label'>Total Tests</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>",
    );
    content.push_str(&format!("{:.1}%", metrics.coverage_percentage));
    content.push_str(
        "</div>
                    <div class='metric-label'>Coverage</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>",
    );
    content.push_str(&format!("{:.1}", metrics.avg_tests_per_requirement));
    content.push_str(
        "</div>
                    <div class='metric-label'>Avg Tests/Req</div>
                </div>
            </div>
        </div>
        
        <div class='section'>
            <h2>Requirements by Status</h2>",
    );

    for (status, count) in metrics.requirements_by_status.clone() {
        content.push_str(&format!(
            "
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>",
            status, count
        ));
    }

    content.push_str(
        "
        </div>
        
        <div class='section'>
            <h2>Tests by Status</h2>",
    );

    for (status, count) in metrics.tests_by_status.clone() {
        content.push_str(&format!(
            "
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>",
            status, count
        ));
    }

    content.push_str(
        "
        </div>
        
        <div class='section'>
            <h2>Requirements by Category</h2>",
    );

    for (category, count) in metrics.requirements_by_category.clone() {
        content.push_str(&format!(
            "
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>",
            category, count
        ));
    }

    content.push_str(&format!(
        "
        </div>
        
        <div class='section'>
            <h2>Coverage Analysis</h2>
            <p><strong>Covered Requirements:</strong> {} out of {} ({:.1}%)</p>
            <div class='coverage-bar'>
                <div class='coverage-fill' style='width: {:.1}%'></div>
            </div>
            <p><strong>Total Test Links:</strong> {}</p>
            <p><strong>Average Tests per Requirement:</strong> {:.1}</p>
        </div>
        
        <div class='section'>
            <h2>Project Statistics</h2>
            <div class='metric-grid'>
                <div class='metric-card'>
                    <div class='metric-value'>{}</div>
                    <div class='metric-label'>Categories</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>{}</div>
                    <div class='metric-label'>Users</div>
                </div>
            </div>
        </div>
        
        <div class='footer'>
            <p>This report was generated automatically by Marreq</p>
        </div>
    </body>
    </html>",
        metrics.covered_requirements,
        metrics.total_requirements,
        metrics.coverage_percentage,
        metrics.coverage_percentage,
        metrics.total_links,
        metrics.avg_tests_per_requirement,
        metrics.total_categories,
        metrics.users_len
    ));

    content
}

// PDF helper functions and types
const PAGE_WIDTH: f32 = 210.0;
const PAGE_HEIGHT: f32 = 297.0;
const INITIAL_Y: f32 = 280.0;
const CONTENT_START_Y: f32 = 250.0;
const PAGE_BREAK_Y: f32 = 50.0;

fn push_text(ops: &mut Vec<Op>, font: BuiltinFont, size: f32, x: Mm, y: Mm, text: &str) {
    ops.push(Op::StartTextSection);
    ops.push(Op::SetTextCursor {
        pos: Point::new(x, y),
    });
    ops.push(Op::SetFontSizeBuiltinFont {
        size: Pt(size),
        font,
    });
    ops.push(Op::WriteTextBuiltinFont {
        items: vec![TextItem::Text(text.to_string())],
        font,
    });
    ops.push(Op::EndTextSection);
}

fn add_header(ops: &mut Vec<Op>) {
    push_text(
        ops,
        BuiltinFont::HelveticaBold,
        18.0,
        Mm(105.0),
        Mm(INITIAL_Y),
        "Marreq Project Report",
    );
    let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    push_text(
        ops,
        BuiltinFont::Helvetica,
        10.0,
        Mm(20.0),
        Mm(270.0),
        &format!("Generated on: {}", current_date),
    );
}

fn add_footer(ops: &mut Vec<Op>) {
    push_text(
        ops,
        BuiltinFont::Helvetica,
        8.0,
        Mm(20.0),
        Mm(20.0),
        "Generated by Marreq - Requirements Management System",
    );
}

fn add_page_number(ops: &mut Vec<Op>, page_number: usize) {
    push_text(
        ops,
        BuiltinFont::Helvetica,
        12.0,
        Mm(20.0),
        Mm(20.0),
        &format!("Page {}", page_number),
    );
}

fn save_pdf(
    mut doc: PdfDocument,
    pages_ops: Vec<Vec<Op>>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let pages = pages_ops
        .into_iter()
        .map(|ops| PdfPage::new(Mm(PAGE_WIDTH), Mm(PAGE_HEIGHT), ops))
        .collect();

    let mut warnings = Vec::<PdfWarnMsg>::new();
    let bytes = doc
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    Ok(bytes)
}

fn ensure_page_space(
    pages: &mut Vec<Vec<Op>>,
    current_page: &mut usize,
    y: &mut Mm,
    page_num: &mut usize,
) -> bool {
    if *y < Mm(PAGE_BREAK_Y) {
        *page_num += 1;
        pages.push(Vec::new());
        *current_page = pages.len() - 1;
        add_page_number(&mut pages[*current_page], *page_num);
        *y = Mm(INITIAL_Y);
        return true;
    }
    false
}

fn add_list_section(
    pages: &mut Vec<Vec<Op>>,
    current_page: &mut usize,
    title: &str,
    items: Vec<String>,
    y: &mut Mm,
    page_num: &mut usize,
) {
    push_text(
        &mut pages[*current_page],
        BuiltinFont::HelveticaBold,
        14.0,
        Mm(20.0),
        *y,
        title,
    );
    *y -= Mm(12.0);
    for item in items {
        push_text(
            &mut pages[*current_page],
            BuiltinFont::Helvetica,
            12.0,
            Mm(25.0),
            *y,
            &item,
        );
        *y -= Mm(8.0);
        if ensure_page_space(pages, current_page, y, page_num) {
            push_text(
                &mut pages[*current_page],
                BuiltinFont::HelveticaBold,
                14.0,
                Mm(20.0),
                *y,
                &format!("{} (continued)", title),
            );
            *y -= Mm(12.0);
        }
    }
    *y -= Mm(8.0);
}

fn add_status_section(
    pages: &mut Vec<Vec<Op>>,
    current_page: &mut usize,
    title: &str,
    data: &std::collections::HashMap<String, i32>,
    y: &mut Mm,
    page_num: &mut usize,
) {
    let items: Vec<String> = data.iter().map(|(s, c)| format!("{}: {}", s, c)).collect();
    add_list_section(pages, current_page, title, items, y, page_num);
}

pub fn generate_pdf_from_html(_html_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let doc = PdfDocument::new("Marreq Report");
    let mut pages = vec![Vec::new()];
    let mut current_page = 0usize;
    let mut page_number = 1usize;

    add_header(&mut pages[current_page]);

    let mut y_position = Mm(CONTENT_START_Y);
    let sections = vec![
        (
            "Project Overview",
            "This report contains project metrics and statistics",
        ),
        (
            "Requirements",
            "Total requirements and their status distribution",
        ),
        ("Tests", "Total tests and their status distribution"),
        ("Coverage", "Requirements coverage analysis"),
        ("Categories", "Requirements categorized by type"),
    ];

    for (title, desc) in sections {
        push_text(
            &mut pages[current_page],
            BuiltinFont::HelveticaBold,
            14.0,
            Mm(20.0),
            y_position,
            title,
        );
        y_position -= Mm(8.0);
        push_text(
            &mut pages[current_page],
            BuiltinFont::Helvetica,
            12.0,
            Mm(20.0),
            y_position,
            desc,
        );
        y_position -= Mm(15.0);

        ensure_page_space(
            &mut pages,
            &mut current_page,
            &mut y_position,
            &mut page_number,
        );
    }

    add_footer(&mut pages[0]);
    save_pdf(doc, pages)
}

pub fn generate_pdf_report_data(metrics: &Metrics) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let doc = PdfDocument::new("Marreq Report");
    let mut pages = vec![Vec::new()];
    let mut current_page = 0usize;
    let mut page_number = 1usize;

    add_header(&mut pages[current_page]);

    let mut y_position = Mm(CONTENT_START_Y);

    let overview_items = vec![
        format!("Total Requirements: {}", metrics.total_requirements),
        format!("Total Tests: {}", metrics.total_tests),
        format!("Total Categories: {}", metrics.total_categories),
        format!("Total Users: {}", metrics.users_len),
    ];
    add_list_section(
        &mut pages,
        &mut current_page,
        "Project Overview",
        overview_items,
        &mut y_position,
        &mut page_number,
    );

    let coverage_items = vec![
        format!(
            "Covered Requirements: {} out of {} ({:.1}%)",
            metrics.covered_requirements, metrics.total_requirements, metrics.coverage_percentage
        ),
        format!("Total Test Links: {}", metrics.total_links),
        format!(
            "Average Tests per Requirement: {:.1}",
            metrics.avg_tests_per_requirement
        ),
    ];
    add_list_section(
        &mut pages,
        &mut current_page,
        "Coverage Analysis",
        coverage_items,
        &mut y_position,
        &mut page_number,
    );

    add_status_section(
        &mut pages,
        &mut current_page,
        "Requirements by Status",
        &metrics.requirements_by_status,
        &mut y_position,
        &mut page_number,
    );

    add_status_section(
        &mut pages,
        &mut current_page,
        "Tests by Status",
        &metrics.tests_by_status,
        &mut y_position,
        &mut page_number,
    );

    add_footer(&mut pages[0]);
    save_pdf(doc, pages)
}

/// One row for the requirements PDF table: (id, title, reference, status, custom_values in definition order).
pub type RequirementsPdfRow = (i32, String, String, String, Vec<String>);

/// Generate a PDF document with a table of requirements (fixed columns + one per custom field).
pub fn generate_requirements_pdf_report(
    project_name: &str,
    rows: &[RequirementsPdfRow],
    custom_headers: &[String],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let doc = PdfDocument::new("Marreq Requirements");
    let mut pages = vec![Vec::new()];
    let mut current_page = 0usize;
    let mut page_number = 1usize;

    push_text(
        &mut pages[current_page],
        BuiltinFont::HelveticaBold,
        16.0,
        Mm(20.0),
        Mm(INITIAL_Y),
        &format!("Requirements - {}", project_name),
    );
    let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC");
    push_text(
        &mut pages[current_page],
        BuiltinFont::Helvetica,
        9.0,
        Mm(20.0),
        Mm(270.0),
        &format!("Generated on: {}", current_date),
    );

    let mut y = Mm(CONTENT_START_Y - 10.0);
    let header_cells: Vec<String> = ["ID", "Title", "Reference", "Status"]
        .into_iter()
        .map(String::from)
        .chain(custom_headers.iter().cloned())
        .collect();
    push_text(
        &mut pages[current_page],
        BuiltinFont::HelveticaBold,
        8.0,
        Mm(20.0),
        y,
        &header_cells.join(" | "),
    );
    y -= Mm(8.0);

    fn truncate(s: &str, max_chars: usize) -> String {
        let s = s.trim();
        if s.chars().count() <= max_chars {
            s.to_string()
        } else {
            format!("{}…", s.chars().take(max_chars - 1).collect::<String>())
        }
    }

    for (id, title, reference, status, custom_vals) in rows {
        if ensure_page_space(&mut pages, &mut current_page, &mut y, &mut page_number) {
            push_text(
                &mut pages[current_page],
                BuiltinFont::HelveticaBold,
                9.0,
                Mm(20.0),
                y,
                "Requirements (continued)",
            );
            y -= Mm(8.0);
        }
        let mut cells = vec![
            id.to_string(),
            truncate(title, 35),
            truncate(reference, 18),
            truncate(status, 12),
        ];
        cells.extend(custom_vals.iter().map(|v| truncate(v, 15)));
        push_text(
            &mut pages[current_page],
            BuiltinFont::Helvetica,
            7.0,
            Mm(20.0),
            y,
            &cells.join(" | "),
        );
        y -= Mm(6.0);
    }

    add_footer(&mut pages[0]);
    save_pdf(doc, pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, vec};

    #[test]
    fn test_generate_pdf_content() {
        let mut req_status = HashMap::new();
        req_status.insert("Open".to_string(), 5);
        req_status.insert("Closed".to_string(), 3);

        let mut status_id = HashMap::new();
        status_id.insert("Passed".to_string(), 10);
        status_id.insert("Failed".to_string(), 2);

        let mut category_id = HashMap::new();
        category_id.insert("Functional".to_string(), 4);
        category_id.insert("Performance".to_string(), 1);

        let html = generate_pdf_content(&Metrics {
            categories: vec![],
            statuses: vec![],
            users_len: 2,
            total_requirements: 8,
            total_tests: 12,
            total_categories: 3,
            requirements_by_status: req_status.clone(),
            tests_by_status: status_id.clone(),
            requirements_by_category: category_id.clone(),
            recent_requirements: 0,
            recent_tests: 0,
            coverage_percentage: 75.0,
            avg_tests_per_requirement: 1.5,
            covered_requirements: 6,
            total_links: 20,
        });

        assert!(html.contains("Total Requirements"));
        assert!(html.contains("Functional"));
        assert!(html.contains("Passed"));
        assert!(html.contains("Average Tests per Requirement"));
    }

    #[test]
    fn test_generate_pdf_from_html() {
        let pdf_bytes = generate_pdf_from_html("<html></html>").unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_generate_pdf_report_data_with_page_break() {
        let mut req_status = HashMap::new();
        for i in 0..15_i32 {
            req_status.insert(format!("Status{}", i), i);
        }
        let mut status_id = HashMap::new();
        for i in 0..15_i32 {
            status_id.insert(format!("TestStatus{}", i), i);
        }
        let pdf_bytes = generate_pdf_report_data(&Metrics {
            categories: vec![],
            statuses: vec![],
            users_len: 3,
            total_requirements: 100,
            total_tests: 50,
            total_categories: 5,
            requirements_by_status: req_status.clone(),
            tests_by_status: status_id.clone(),
            requirements_by_category: HashMap::new(),
            recent_requirements: 0,
            recent_tests: 0,
            coverage_percentage: 60.0,
            avg_tests_per_requirement: 2.0,
            covered_requirements: 60,
            total_links: 80,
        })
        .unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_generate_pdf_report_data_adds_new_page() {
        let mut req_status = HashMap::new();
        for i in 0..20_i32 {
            req_status.insert(format!("Status{}", i), i);
        }

        let pdf_bytes = generate_pdf_report_data(&Metrics {
            categories: vec![],
            statuses: vec![],
            users_len: 3,
            total_requirements: 100,
            total_tests: 50,
            total_categories: 5,
            requirements_by_status: req_status.clone(),
            tests_by_status: HashMap::new(),
            requirements_by_category: HashMap::new(),
            recent_requirements: 0,
            recent_tests: 0,
            coverage_percentage: 60.0,
            avg_tests_per_requirement: 2.0,
            covered_requirements: 60,
            total_links: 80,
        })
        .unwrap();

        let pdf_text = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_text.contains("Page 2"));
    }
}
