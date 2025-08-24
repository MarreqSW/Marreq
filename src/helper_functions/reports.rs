
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
    
    content.push_str(&chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
    content.push_str("</p>
        </div>
        
        <div class='section'>
            <h2>Executive Summary</h2>
            <div class='metric-grid'>
                <div class='metric-card'>
                    <div class='metric-value'>");
    content.push_str(&total_requirements.to_string());
    content.push_str("</div>
                    <div class='metric-label'>Total Requirements</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>");
    content.push_str(&total_tests.to_string());
    content.push_str("</div>
                    <div class='metric-label'>Total Tests</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>");
    content.push_str(&format!("{:.1}%", coverage_percentage));
    content.push_str("</div>
                    <div class='metric-label'>Coverage</div>
                </div>
                <div class='metric-card'>
                    <div class='metric-value'>");
    content.push_str(&format!("{:.1}", avg_tests_per_requirement));
    content.push_str("</div>
                    <div class='metric-label'>Avg Tests/Req</div>
                </div>
            </div>
        </div>
        
        <div class='section'>
            <h2>Requirements by Status</h2>");
    
    for (status, count) in requirements_by_status {
        content.push_str(&format!("
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>", status, count));
    }
    
    content.push_str("
        </div>
        
        <div class='section'>
            <h2>Tests by Status</h2>");
    
    for (status, count) in tests_by_status {
        content.push_str(&format!("
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>", status, count));
    }
    
    content.push_str("
        </div>
        
        <div class='section'>
            <h2>Requirements by Category</h2>");
    
    for (category, count) in requirements_by_category {
        content.push_str(&format!("
            <div class='status-item'>
                <span class='status-name'>{}</span>
                <span class='status-count'>{}</span>
            </div>", category, count));
    }
    
    content.push_str(&format!("
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

pub fn generate_pdf_from_html(_html_content: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use printpdf::*;
    use std::io::{Cursor, BufWriter};
    
    // Create a new PDF document
    let (doc, page1, layer1) = PdfDocument::new("ReqMan Report", Mm(210.0), Mm(297.0), "Layer 1");
    let page1 = doc.get_page(page1);
    let layer1 = page1.get_layer(layer1);
    
    // Add title
    let title_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Failed to load title font: {}", e))?;
    let title_font_size = 18.0;
    let title_text = "ReqMan Project Report";
    
    layer1.use_text(
        title_text,
        title_font_size,
        Mm(105.0),
        Mm(280.0),
        &title_font,
    );
    
    // Add generation date
    let date_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load date font: {}", e))?;
    let date_font_size = 10.0;
    let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    layer1.use_text(
        &format!("Generated on: {}", current_date),
        date_font_size,
        Mm(20.0),
        Mm(270.0),
        &date_font,
    );
    
    // Add content sections
    let content_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load content font: {}", e))?;
    let content_font_size = 12.0;
    let mut y_position = Mm(250.0);
    
    // Parse HTML content to extract meaningful data
    // For now, we'll create a simple structured report
    let sections = vec![
        ("Project Overview", "This report contains project metrics and statistics"),
        ("Requirements", "Total requirements and their status distribution"),
        ("Tests", "Total tests and their status distribution"),
        ("Coverage", "Requirements coverage analysis"),
        ("Categories", "Requirements categorized by type"),
    ];
    
    for (section_title, section_desc) in sections {
        // Section title
        let section_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
            .map_err(|e| format!("Failed to load section font: {}", e))?;
        layer1.use_text(
            section_title,
            content_font_size + 2.0,
            Mm(20.0),
            y_position,
            &section_font,
        );
        y_position -= Mm(8.0);
        
        // Section description
        layer1.use_text(
            section_desc,
            content_font_size,
            Mm(20.0),
            y_position,
            &content_font,
        );
        y_position -= Mm(15.0);
        
        // Add some spacing between sections
        if y_position < Mm(50.0) {
            // If we're running out of space, add a new page
            let (page2, layer2) = doc.add_page(Mm(210.0), Mm(297.0), "Page 2");
            let page2 = doc.get_page(page2);
            let layer2 = page2.get_layer(layer2);
            
            // Add page number
            layer2.use_text(
                "Page 2",
                content_font_size,
                Mm(20.0),
                Mm(20.0),
                &content_font,
            );
            
            y_position = Mm(280.0);
        }
    }
    
    // Add footer
    let footer_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load footer font: {}", e))?;
    layer1.use_text(
        "Generated by ReqMan - Requirements Management System",
        8.0,
        Mm(20.0),
        Mm(20.0),
        &footer_font,
    );
    
    // Write PDF to memory
    let cursor = Cursor::new(Vec::new());
    let mut buf_writer = BufWriter::new(cursor);
    doc.save(&mut buf_writer)?;
    
    let cursor = buf_writer.into_inner()?;
    Ok(cursor.into_inner())
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
    use printpdf::*;
    use std::io::{Cursor, BufWriter};
    
    // Create a new PDF document
    let (doc, page1, layer1) = PdfDocument::new("ReqMan Report", Mm(210.0), Mm(297.0), "Layer 1");
    let page1 = doc.get_page(page1);
    let layer1 = page1.get_layer(layer1);
    
    // Add title
    let title_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Failed to load title font: {}", e))?;
    let title_font_size = 18.0;
    let title_text = "ReqMan Project Report";
    
    layer1.use_text(
        title_text,
        title_font_size,
        Mm(105.0),
        Mm(105.0),
        &title_font,
    );
    
    // Add generation date
    let date_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load date font: {}", e))?;
    let date_font_size = 10.0;
    let current_date = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    layer1.use_text(
        &format!("Generated on: {}", current_date),
        date_font_size,
        Mm(20.0),
        Mm(270.0),
        &date_font,
    );
    
    // Add metrics overview
    let metrics_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("Failed to load metrics font: {}", e))?;
    let metrics_font_size = 14.0;
    let content_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load content font: {}", e))?;
    let content_font_size = 12.0;
    let mut y_position = Mm(250.0);
    
    // Project Overview Section
    layer1.use_text(
        "Project Overview",
        metrics_font_size,
        Mm(20.0),
        y_position,
        &metrics_font,
    );
    y_position -= Mm(12.0);
    
    let overview_items = vec![
        format!("Total Requirements: {}", total_requirements),
        format!("Total Tests: {}", total_tests),
        format!("Total Categories: {}", total_categories),
        format!("Total Users: {}", total_users),
    ];
    
    for item in overview_items {
        layer1.use_text(
            &item,
            content_font_size,
            Mm(25.0),
            y_position,
            &content_font,
        );
        y_position -= Mm(8.0);
    }
    
    y_position -= Mm(8.0);
    
    // Coverage Analysis Section
    layer1.use_text(
        "Coverage Analysis",
        metrics_font_size,
        Mm(20.0),
        y_position,
        &metrics_font,
    );
    y_position -= Mm(12.0);
    
    let coverage_items = vec![
        format!("Covered Requirements: {} out of {} ({:.1}%)", 
                covered_requirements, total_requirements, coverage_percentage),
        format!("Total Test Links: {}", total_links),
        format!("Average Tests per Requirement: {:.1}", avg_tests_per_requirement),
    ];
    
    for item in coverage_items {
        layer1.use_text(
            &item,
            content_font_size,
            Mm(25.0),
            y_position,
            &content_font,
        );
        y_position -= Mm(8.0);
    }
    
    y_position -= Mm(8.0);
    
    // Requirements by Status Section
    layer1.use_text(
        "Requirements by Status",
        metrics_font_size,
        Mm(20.0),
        y_position,
        &metrics_font,
    );
    y_position -= Mm(12.0);
    
    for (status, count) in &requirements_by_status {
        let status_text = format!("{}: {}", status, count);
        layer1.use_text(
            &status_text,
            content_font_size,
            Mm(25.0),
            y_position,
            &content_font,
        );
        y_position -= Mm(8.0);
        
        if y_position < Mm(50.0) {
            // Add new page if needed
            let (page2, layer2) = doc.add_page(Mm(210.0), Mm(297.0), "Page 2");
            let page2 = doc.get_page(page2);
            let layer2 = page2.get_layer(layer2);
            
            // Continue on new page
            y_position = Mm(280.0);
            layer2.use_text(
                "Requirements by Status (continued)",
                metrics_font_size,
                Mm(20.0),
                y_position,
                &metrics_font,
            );
            y_position -= Mm(12.0);
        }
    }
    
    y_position -= Mm(8.0);
    
    // Tests by Status Section
    if y_position > Mm(60.0) {
        layer1.use_text(
            "Tests by Status",
            metrics_font_size,
            Mm(20.0),
            y_position,
            &metrics_font,
        );
        y_position -= Mm(12.0);
        
        for (status, count) in &tests_by_status {
            let status_text = format!("{}: {}", status, count);
            layer1.use_text(
                &status_text,
                content_font_size,
                Mm(25.0),
                y_position,
                &content_font,
            );
            y_position -= Mm(8.0);
        }
    } else {
        // Add new page for tests section
        let (page3, layer3) = doc.add_page(Mm(210.0), Mm(297.0), "Page 3");
        let page3 = doc.get_page(page3);
        let layer3 = page3.get_layer(layer3);
        
        layer3.use_text(
            "Tests by Status",
            metrics_font_size,
            Mm(20.0),
            Mm(280.0),
            &metrics_font,
        );
        
        let mut test_y = Mm(268.0);
        for (status, count) in &tests_by_status {
            let status_text = format!("{}: {}", status, count);
            layer3.use_text(
                &status_text,
                content_font_size,
                Mm(25.0),
                test_y,
                &content_font,
            );
            test_y -= Mm(8.0);
        }
    }
    
    // Add footer to all pages
    let footer_font = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("Failed to load footer font: {}", e))?;
    layer1.use_text(
        "Generated by ReqMan - Requirements Management System",
        8.0,
        Mm(20.0),
        Mm(20.0),
        &footer_font,
    );
    
    // Write PDF to memory
    let cursor = Cursor::new(Vec::new());
    let mut buf_writer = BufWriter::new(cursor);
    doc.save(&mut buf_writer)?;
    
    let cursor = buf_writer.into_inner()?;
    Ok(cursor.into_inner())
}

// Project management functions
