pub fn generate_pdf_content(
    total_requirements: usize,
    total_tests: usize,
    total_categories: usize,
    total_users: usize,
    coverage_percentage: f64,
    avg_tests_per_requirement: f64,
    covered_requirements: usize,
    total_links: usize,
    requirements_by_status: std::collections::HashMap<String, i32>,
    tests_by_status: std::collections::HashMap<String, i32>,
    requirements_by_category: std::collections::HashMap<String, i32>,
) -> String {
    let mut content = String::new();

    // Header
    content.push_str("
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset='utf-8'>
        <title>ReqMan - Project Report</title>
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
            <h1>ReqMan Project Report</h1>
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
    content.push_str(&total_requirements.to_string());
    content.push_str(
        "</div>
                    <div class='metric-label'>Total Requirements</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>",
    );
    content.push_str(&total_tests.to_string());
    content.push_str(
        "</div>
                    <div class='metric-label'>Total Tests</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>",
    );
    content.push_str(&format!("{:.1}%", coverage_percentage));
    content.push_str(
        "</div>
                    <div class='metric-label'>Coverage</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>",
    );
    content.push_str(&format!("{:.1}", avg_tests_per_requirement));
    content.push_str(
        "</div>
                    <div class='metric-label'>Avg Tests/Req</div>
                </div>
            </div>
        </div>
        
        <div class='section'>
            <h2>Requirements by Status</h2>",
    );

    for (status, count) in requirements_by_status {
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

    for (status, count) in tests_by_status {
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

    for (category, count) in requirements_by_category {
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
            <p>This report was generated automatically by ReqMan</p>
        </div>
    </body>
    </html>",
        covered_requirements,
        total_requirements,
        coverage_percentage,
        coverage_percentage,
        total_links,
        avg_tests_per_requirement,
        total_categories,
        total_users
    ));

    content
}
use printpdf::{
    BuiltinFont, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, PdfWarnMsg, Point, Pt, TextItem,
};

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
        "ReqMan Project Report",
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
        "Generated by ReqMan - Requirements Management System",
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
    let doc = PdfDocument::new("ReqMan Report");
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

pub fn generate_pdf_report_data(
    total_requirements: usize,
    total_tests: usize,
    total_categories: usize,
    total_users: usize,
    coverage_percentage: f64,
    avg_tests_per_requirement: f64,
    covered_requirements: usize,
    total_links: usize,
    requirements_by_status: std::collections::HashMap<String, i32>,
    tests_by_status: std::collections::HashMap<String, i32>,
    _requirements_by_category: std::collections::HashMap<String, i32>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let doc = PdfDocument::new("ReqMan Report");
    let mut pages = vec![Vec::new()];
    let mut current_page = 0usize;
    let mut page_number = 1usize;

    add_header(&mut pages[current_page]);

    let mut y_position = Mm(CONTENT_START_Y);

    let overview_items = vec![
        format!("Total Requirements: {}", total_requirements),
        format!("Total Tests: {}", total_tests),
        format!("Total Categories: {}", total_categories),
        format!("Total Users: {}", total_users),
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
            covered_requirements, total_requirements, coverage_percentage
        ),
        format!("Total Test Links: {}", total_links),
        format!(
            "Average Tests per Requirement: {:.1}",
            avg_tests_per_requirement
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
        &requirements_by_status,
        &mut y_position,
        &mut page_number,
    );

    add_status_section(
        &mut pages,
        &mut current_page,
        "Tests by Status",
        &tests_by_status,
        &mut y_position,
        &mut page_number,
    );

    add_footer(&mut pages[0]);
    save_pdf(doc, pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_generate_pdf_content() {
        let mut req_status = HashMap::new();
        req_status.insert("Open".to_string(), 5);
        req_status.insert("Closed".to_string(), 3);

        let mut test_status = HashMap::new();
        test_status.insert("Passed".to_string(), 10);
        test_status.insert("Failed".to_string(), 2);

        let mut req_category = HashMap::new();
        req_category.insert("Functional".to_string(), 4);
        req_category.insert("Performance".to_string(), 1);

        let html = generate_pdf_content(
            8,
            12,
            3,
            2,
            75.0,
            1.5,
            6,
            20,
            req_status.clone(),
            test_status.clone(),
            req_category.clone(),
        );

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
        for i in 0..15 {
            req_status.insert(format!("Status{}", i), i as i32);
        }
        let mut test_status = HashMap::new();
        for i in 0..15 {
            test_status.insert(format!("TestStatus{}", i), i as i32);
        }
        let pdf_bytes = generate_pdf_report_data(
            100,
            50,
            5,
            3,
            60.0,
            2.0,
            60,
            80,
            req_status,
            test_status,
            HashMap::new(),
        )
        .unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_generate_pdf_report_data_adds_new_page() {
        let mut req_status = HashMap::new();
        for i in 0..20 {
            req_status.insert(format!("Status{}", i), i as i32);
        }

        let pdf_bytes = generate_pdf_report_data(
            100,
            50,
            5,
            3,
            60.0,
            2.0,
            60,
            80,
            req_status,
            HashMap::new(),
            HashMap::new(),
        )
        .unwrap();

        let pdf_text = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_text.contains("Page 2"));
    }
}
