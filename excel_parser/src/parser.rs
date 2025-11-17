use calamine::{open_workbook, DataType, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Result, anyhow};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequirementData {
    pub req_id: Option<i32>,
    pub req_title: String,
    pub req_description: String,
    pub req_reference: String,
    pub req_category: String,
    pub req_applicability: String,
    pub req_current_status: String,
    pub req_verification_method: String,
    pub req_author: String,
    pub req_reviewer: String,
    pub req_parent: Option<i32>,
    pub req_parent_title: String,
    pub req_link: String,
    pub req_creation_date: String,
    pub req_update_date: String,
    pub req_deadline_date: String,
    pub req_justification: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestData {
    pub test_id: Option<i32>,
    pub test_name: String,
    pub test_description: String,
    pub test_status: String,
    pub test_source: String,
    pub test_parent: Option<i32>,
    pub test_parent_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImportData {
    Requirement(RequirementData),
    Test(TestData),
}

pub fn parse_excel_file<P: AsRef<Path>>(path: P) -> Result<Vec<ImportData>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    
    // Try to determine the type of Excel file based on sheet names or content
    let sheet_names: Vec<String> = workbook.sheet_names().to_owned();
    
    if sheet_names.iter().any(|name| name.to_lowercase().contains("requirement")) {
        parse_requirements_sheet(&mut workbook)
    } else if sheet_names.iter().any(|name| name.to_lowercase().contains("test")) {
        parse_tests_sheet(&mut workbook)
    } else {
        // Default to requirements if we can't determine
        parse_requirements_sheet(&mut workbook)
    }
}

fn parse_requirements_sheet(workbook: &mut Xlsx<std::io::BufReader<std::fs::File>>) -> Result<Vec<ImportData>> {
    let mut data = Vec::new();
    
    // Get the first sheet
    let sheet_name = workbook.sheet_names()[0].clone();
    let range = workbook.worksheet_range(&sheet_name)
        .ok_or_else(|| anyhow!("Sheet '{}' not found", sheet_name))??;
    
    let mut headers = Vec::new();
    let mut is_first_row = true;
    
    for row in range.rows() {
        if is_first_row {
            // Parse headers
            headers = row.iter()
                .map(|cell| cell.to_string().to_lowercase())
                .collect();
            is_first_row = false;
            continue;
        }
        
        if row.iter().all(|cell| cell.is_empty()) {
            continue; // Skip empty rows
        }
        
        let requirement = parse_requirement_row(row, &headers)?;
        data.push(ImportData::Requirement(requirement));
    }
    
    Ok(data)
}

fn parse_tests_sheet(workbook: &mut Xlsx<std::io::BufReader<std::fs::File>>) -> Result<Vec<ImportData>> {
    let mut data = Vec::new();
    
    // Get the first sheet
    let sheet_name = workbook.sheet_names()[0].clone();
    let range = workbook.worksheet_range(&sheet_name)
        .ok_or_else(|| anyhow!("Sheet '{}' not found", sheet_name))??;
    
    let mut headers = Vec::new();
    let mut is_first_row = true;
    
    for row in range.rows() {
        if is_first_row {
            // Parse headers
            headers = row.iter()
                .map(|cell| cell.to_string().to_lowercase())
                .collect();
            is_first_row = false;
            continue;
        }
        
        if row.iter().all(|cell| cell.is_empty()) {
            continue; // Skip empty rows
        }
        
        let test = parse_test_row(row, &headers)?;
        data.push(ImportData::Test(test));
    }
    
    Ok(data)
}

fn parse_requirement_row(row: &[DataType], headers: &[String]) -> Result<RequirementData> {
    let mut req = RequirementData {
        req_id: None,
        req_title: String::new(),
        req_description: String::new(),
        req_reference: String::new(),
        req_category: String::new(),
        req_applicability: String::new(),
        req_current_status: String::new(),
        req_verification_method: String::new(),
        req_author: String::new(),
        req_reviewer: String::new(),
        req_parent: None,
        req_parent_title: String::new(),
        req_link: String::new(),
        req_creation_date: String::new(),
        req_update_date: String::new(),
        req_deadline_date: String::new(),
        req_justification: None,
    };
    
    for (i, cell) in row.iter().enumerate() {
        if i >= headers.len() {
            break;
        }
        
        let header = &headers[i];
        let value = cell.to_string();
        
        match header.as_str() {
            "req id" => {
                if let Ok(id) = value.parse::<i32>() {
                    req.req_id = Some(id);
                }
            },
            "title" => req.req_title = value,
            "description" => req.req_description = value,
            "reference" => req.req_reference = value,
            "category" => req.req_category = value,
            "applicability" => req.req_applicability = value,
            "status" => req.req_current_status = value,
            "verification" => req.req_verification_method = value,
            "author" => req.req_author = value,
            "reviewer" => req.req_reviewer = value,
            "parent" => {
                if value != "None" && !value.is_empty() {
                    if let Ok(id) = value.parse::<i32>() {
                        req.req_parent = Some(id);
                    }
                }
            },
            "parent title" => req.req_parent_title = value,
            "link" => req.req_link = value,
            "creation date" => req.req_creation_date = value,
            "update date" => req.req_update_date = value,
            "deadline date" => req.req_deadline_date = value,
            "justification" => {
                if value != "None" && !value.is_empty() {
                    req.req_justification = Some(value);
                }
            },
            _ => {} // Ignore unknown headers
        }
    }
    
    Ok(req)
}

fn parse_test_row(row: &[DataType], headers: &[String]) -> Result<TestData> {
    let mut test = TestData {
        test_id: None,
        test_name: String::new(),
        test_description: String::new(),
        test_status: String::new(),
        test_source: String::new(),
        test_parent: None,
        test_parent_name: String::new(),
    };
    
    for (i, cell) in row.iter().enumerate() {
        if i >= headers.len() {
            break;
        }
        
        let header = &headers[i];
        let value = cell.to_string();
        
        match header.as_str() {
            "test id" => {
                if let Ok(id) = value.parse::<i32>() {
                    test.test_id = Some(id);
                }
            },
            "name" => test.test_name = value,
            "description" => test.test_description = value,
            "status" => test.test_status = value,
            "source" => test.test_source = value,
            "parent id" => {
                if value != "None" && !value.is_empty() {
                    if let Ok(id) = value.parse::<i32>() {
                        test.test_parent = Some(id);
                    }
                }
            },
            "parent name" => test.test_parent_name = value,
            _ => {} // Ignore unknown headers
        }
    }
    
    Ok(test)
} 