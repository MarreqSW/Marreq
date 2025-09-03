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
    BuiltinFont, IndirectFontRef, Mm, PdfDocument, PdfDocumentReference, PdfLayerReference,
};
use std::io::{BufWriter, Cursor};

fn add_text(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    size: f32,
    x: Mm,
    y: Mm,
    text: &str,
) {
    layer.use_text(text, size, x, y, font);
}

fn add_header(
    layer: &PdfLayerReference,
    title_font: &IndirectFontRef,
    date_font: &IndirectFontRef,
) {
    add_text(
        layer,
        title_font,
        18.0,
        Mm(105.0),
        Mm(280.0),
        "ReqMan Project Report",
    );
    let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    add_text(
        layer,
        date_font,
        10.0,
        Mm(20.0),
        Mm(270.0),
        &format!("Generated on: {}", current_date),
    );
}

fn add_footer(layer: &PdfLayerReference, font: &IndirectFontRef) {
    add_text(
        layer,
        font,
        8.0,
        Mm(20.0),
        Mm(20.0),
        "Generated by ReqMan - Requirements Management System",
    );
}

fn save_pdf(doc: PdfDocumentReference) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cursor = Cursor::new(Vec::new());
    let mut buf_writer = BufWriter::new(cursor);
    doc.save(&mut buf_writer)?;
    Ok(buf_writer.into_inner()?.into_inner())
}

fn add_page(
    doc: &PdfDocumentReference,
    font: &IndirectFontRef,
    page_number: usize,
) -> (PdfLayerReference, Mm) {
    let (page_idx, layer_idx) =
        doc.add_page(Mm(210.0), Mm(297.0), &format!("Page {}", page_number));
    let page = doc.get_page(page_idx);
    let layer = page.get_layer(layer_idx);
    add_text(
        &layer,
        font,
        12.0,
        Mm(20.0),
        Mm(20.0),
        &format!("Page {}", page_number),
    );
    (layer, Mm(280.0))
}

fn add_list_section(
    doc: &PdfDocumentReference,
    layer: &mut PdfLayerReference,
    title: &str,
    items: Vec<String>,
    title_font: &IndirectFontRef,
    content_font: &IndirectFontRef,
    y: &mut Mm,
    page_num: &mut usize,
) {
    add_text(layer, title_font, 14.0, Mm(20.0), *y, title);
    *y -= Mm(12.0);
    for item in items {
        add_text(layer, content_font, 12.0, Mm(25.0), *y, &item);
        *y -= Mm(8.0);
        if *y < Mm(50.0) {
            *page_num += 1;
            let (new_layer, new_y) = add_page(doc, content_font, *page_num);
            *layer = new_layer;
            *y = new_y;
            add_text(
                layer,
                title_font,
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
    doc: &PdfDocumentReference,
    layer: &mut PdfLayerReference,
    title: &str,
    data: &std::collections::HashMap<String, i32>,
    title_font: &IndirectFontRef,
    content_font: &IndirectFontRef,
    y: &mut Mm,
    page_num: &mut usize,
) {
    let items: Vec<String> = data.iter().map(|(s, c)| format!("{}: {}", s, c)).collect();
    add_list_section(
        doc,
        layer,
        title,
        items,
        title_font,
        content_font,
        y,
        page_num,
    );
}

pub fn generate_pdf_from_html(_html_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let (doc, page_idx, layer_idx) =
        PdfDocument::new("ReqMan Report", Mm(210.0), Mm(297.0), "Layer 1");
    let page = doc.get_page(page_idx);
    let first_layer = page.get_layer(layer_idx);
    let mut current_layer = first_layer.clone();

    let title_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let regular_font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let bold_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let footer_font = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    add_header(&first_layer, &title_font, &regular_font);

    let mut y_position = Mm(250.0);
    let mut page_number = 1usize;
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
        add_text(
            &current_layer,
            &bold_font,
            14.0,
            Mm(20.0),
            y_position,
            title,
        );
        y_position -= Mm(8.0);
        add_text(
            &current_layer,
            &regular_font,
            12.0,
            Mm(20.0),
            y_position,
            desc,
        );
        y_position -= Mm(15.0);

        if y_position < Mm(50.0) {
            page_number += 1;
            let (new_layer, new_y) = add_page(&doc, &regular_font, page_number);
            current_layer = new_layer;
            y_position = new_y;
        }
    }

    add_footer(&first_layer, &footer_font);
    save_pdf(doc)
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
    let (doc, page_idx, layer_idx) =
        PdfDocument::new("ReqMan Report", Mm(210.0), Mm(297.0), "Layer 1");
    let page = doc.get_page(page_idx);
    let first_layer = page.get_layer(layer_idx);
    let mut layer = first_layer.clone();

    let title_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let regular_font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let bold_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let footer_font = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    add_header(&first_layer, &title_font, &regular_font);

    let mut y_position = Mm(250.0);
    let mut page_number = 1usize;

    let overview_items = vec![
        format!("Total Requirements: {}", total_requirements),
        format!("Total Tests: {}", total_tests),
        format!("Total Categories: {}", total_categories),
        format!("Total Users: {}", total_users),
    ];
    add_list_section(
        &doc,
        &mut layer,
        "Project Overview",
        overview_items,
        &bold_font,
        &regular_font,
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
        &doc,
        &mut layer,
        "Coverage Analysis",
        coverage_items,
        &bold_font,
        &regular_font,
        &mut y_position,
        &mut page_number,
    );

    add_status_section(
        &doc,
        &mut layer,
        "Requirements by Status",
        &requirements_by_status,
        &bold_font,
        &regular_font,
        &mut y_position,
        &mut page_number,
    );

    add_status_section(
        &doc,
        &mut layer,
        "Tests by Status",
        &tests_by_status,
        &bold_font,
        &regular_font,
        &mut y_position,
        &mut page_number,
    );

    add_footer(&first_layer, &footer_font);
    save_pdf(doc)
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
        ).unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF"));
    }
}
