use calamine::{open_workbook, Data, DataType, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Result, anyhow};

#[derive(Debug, Serialize, Deserialize)]
pub struct RequirementData {
    pub id: Option<i32>,
    pub title: String,
    pub description: String,
    pub reference_code: String,
    pub category_id: String,
    pub applicability_id: String,
    pub status_id: String,
    pub verification_method_id: String,
    pub author_id: String,
    pub reviewer_id: String,
    pub parent_id: Option<i32>,
    pub req_parent_title: String,
    pub req_link: String,
    pub creation_date: String,
    pub update_date: String,
    pub deadline_date: String,
    pub justification: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestData {
    pub id: Option<i32>,
    pub name: String,
    pub description: String,
    pub status_id: String,
    pub source: String,
    pub parent_id: Option<i32>,
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
        .map_err(|e| anyhow!("Sheet '{}' not found: {}", sheet_name, e))?;
    
    let mut headers = Vec::new();
    let mut is_first_row = true;
    
    for row in range.rows() {
        if is_first_row {
            // Parse headers
            headers = row.iter()
                .map(|cell: &Data| cell.to_string().to_lowercase())
                .collect();
            is_first_row = false;
            continue;
        }
        
        if row.iter().all(|cell: &Data| cell.is_empty()) {
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
        .map_err(|e| anyhow!("Sheet '{}' not found: {}", sheet_name, e))?;
    
    let mut headers = Vec::new();
    let mut is_first_row = true;
    
    for row in range.rows() {
        if is_first_row {
            // Parse headers
            headers = row.iter()
                .map(|cell: &Data| cell.to_string().to_lowercase())
                .collect();
            is_first_row = false;
            continue;
        }
        
        if row.iter().all(|cell: &Data| cell.is_empty()) {
            continue; // Skip empty rows
        }
        
        let test = parse_test_row(row, &headers)?;
        data.push(ImportData::Test(test));
    }
    
    Ok(data)
}

fn parse_requirement_row(row: &[Data], headers: &[String]) -> Result<RequirementData> {
    let mut req = RequirementData {
        id: None,
        title: String::new(),
        description: String::new(),
        reference_code: String::new(),
        category_id: String::new(),
        applicability_id: String::new(),
        status_id: String::new(),
        verification_method_id: String::new(),
        author_id: String::new(),
        reviewer_id: String::new(),
        parent_id: None,
        req_parent_title: String::new(),
        req_link: String::new(),
        creation_date: String::new(),
        update_date: String::new(),
        deadline_date: String::new(),
        justification: None,
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
                    req.id = Some(id);
                }
            },
            "title" => req.title = value,
            "description" => req.description = value,
            "reference" => req.reference_code = value,
            "category" => req.category_id = value,
            "applicability" => req.applicability_id = value,
            "status" => req.status_id = value,
            "verification" => req.verification_method_id = value,
            "author" => req.author_id = value,
            "reviewer" => req.reviewer_id = value,
            "parent" => {
                if value != "None" && !value.is_empty() {
                    if let Ok(id) = value.parse::<i32>() {
                        req.parent_id = Some(id);
                    }
                }
            },
            "parent title" => req.req_parent_title = value,
            "link" => req.req_link = value,
            "creation date" => req.creation_date = value,
            "update date" => req.update_date = value,
            "deadline date" => req.deadline_date = value,
            "justification" => {
                if value != "None" && !value.is_empty() {
                    req.justification = Some(value);
                }
            },
            _ => {} // Ignore unknown headers
        }
    }
    
    Ok(req)
}

fn parse_test_row(row: &[Data], headers: &[String]) -> Result<TestData> {
    let mut test = TestData {
        id: None,
        name: String::new(),
        description: String::new(),
        status_id: String::new(),
        source: String::new(),
        parent_id: None,
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
                    test.id = Some(id);
                }
            },
            "name" => test.name = value,
            "description" => test.description = value,
            "status" => test.status_id = value,
            "source" => test.source = value,
            "parent id" => {
                if value != "None" && !value.is_empty() {
                    if let Ok(id) = value.parse::<i32>() {
                        test.parent_id = Some(id);
                    }
                }
            },
            "parent name" => test.test_parent_name = value,
            _ => {} // Ignore unknown headers
        }
    }
    
    Ok(test)
} 