use crate::parser::{ImportData, RequirementData, TestData};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{json, Value};


pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn import_data(&self, data: &[ImportData]) -> Result<Vec<Result<String, String>>> {
        let mut results = Vec::new();

        for item in data {
            let result = match item {
                ImportData::Requirement(req) => self.import_requirement(req).await,
                ImportData::Test(test) => self.import_test(test).await,
            };
            results.push(result);
        }

        Ok(results)
    }

    async fn import_requirement(&self, req: &RequirementData) -> Result<String, String> {
        // First, resolve category ID
        let category_id = self.resolve_category(&req.req_category).await
            .map_err(|e| format!("Failed to resolve category '{}': {}", req.req_category, e))?;

        // Resolve applicability ID
        let applicability_id = self.resolve_applicability(&req.req_applicability).await
            .map_err(|e| format!("Failed to resolve applicability '{}': {}", req.req_applicability, e))?;

        // Resolve status ID
        let status_id = self.resolve_status(&req.req_current_status).await
            .map_err(|e| format!("Failed to resolve status '{}': {}", req.req_current_status, e))?;

        // Resolve verification ID
        let verification_id = self.resolve_verification(&req.req_verification).await
            .map_err(|e| format!("Failed to resolve verification '{}': {}", req.req_verification, e))?;

        // Resolve author ID
        let author_id = self.resolve_user(&req.req_author).await
            .map_err(|e| format!("Failed to resolve author '{}': {}", req.req_author, e))?;

        // Resolve reviewer ID
        let reviewer_id = self.resolve_user(&req.req_reviewer).await
            .map_err(|e| format!("Failed to resolve reviewer '{}': {}", req.req_reviewer, e))?;

        // Resolve parent requirement ID if specified
        let parent_id = if req.req_parent_title != "None" && !req.req_parent_title.is_empty() {
            self.resolve_requirement_by_title(&req.req_parent_title).await.ok()
        } else {
            req.req_parent
        };

        let payload = json!({
            "req_title": req.req_title,
            "req_description": req.req_description,
            "req_reference": req.req_reference,
            "req_category": category_id,
            "req_applicability": applicability_id,
            "req_current_status": status_id,
            "req_verification": verification_id,
            "req_author": author_id,
            "req_reviewer": reviewer_id,
            "req_parent": parent_id.unwrap_or(0),
            "req_link": req.req_link,
            "req_justification": req.req_justification
        });

        let response = self.client
            .post(&format!("{}/api/v1/requirements", self.base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if response.status().is_success() {
            let _response_text = response.text().await
                .map_err(|e| format!("Failed to read response: {}", e))?;
            Ok(format!("Requirement '{}' imported successfully", req.req_title))
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Failed to import requirement '{}': {}", req.req_title, error_text))
        }
    }

    async fn import_test(&self, test: &TestData) -> Result<String, String> {
        // Resolve status ID
        let status_id = self.resolve_status(&test.test_status).await
            .map_err(|e| format!("Failed to resolve status '{}': {}", test.test_status, e))?;

        // Resolve parent test ID if specified
        let parent_id = if test.test_parent_name != "None" && !test.test_parent_name.is_empty() {
            self.resolve_test_by_name(&test.test_parent_name).await.ok()
        } else {
            test.test_parent
        };

        let payload = json!({
            "test_name": test.test_name,
            "test_description": test.test_description,
            "test_source": test.test_source,
            "test_status": status_id,
            "test_parent": parent_id.unwrap_or(0)
        });

        let response = self.client
            .post(&format!("{}/api/v1/tests", self.base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if response.status().is_success() {
            let _response_text = response.text().await
                .map_err(|e| format!("Failed to read response: {}", e))?;
            Ok(format!("Test '{}' imported successfully", test.test_name))
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Failed to import test '{}': {}", test.test_name, error_text))
        }
    }

    async fn resolve_category(&self, category_name: &str) -> Result<i32> {
        let response = self.client
            .get(&format!("{}/api/v1/categories", self.base_url))
            .send()
            .await?;

        let categories: Vec<Value> = response.json().await?;
        
        for category in categories {
            if category["cat_title"].as_str() == Some(category_name) {
                return Ok(category["cat_id"].as_i64().unwrap_or(0) as i32);
            }
        }

        // If category doesn't exist, create it
        let payload = json!({
            "cat_title": category_name,
            "cat_description": format!("Imported category: {}", category_name),
            "cat_tag": category_name.to_lowercase().replace(" ", "_")
        });

        let response = self.client
            .post(&format!("{}/api/v1/categories", self.base_url))
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            // For now, return a default ID since the API might not return the created ID
            Ok(1) // You might need to adjust this based on your API response
        } else {
            Err(anyhow!("Failed to create category: {}", category_name))
        }
    }

    async fn resolve_applicability(&self, applicability_name: &str) -> Result<i32> {
        let response = self.client
            .get(&format!("{}/api/v1/applicability", self.base_url))
            .send()
            .await?;

        let applicability_list: Vec<Value> = response.json().await?;
        
        for app in applicability_list {
            if app["app_title"].as_str() == Some(applicability_name) {
                return Ok(app["app_id"].as_i64().unwrap_or(0) as i32);
            }
        }

        // If applicability doesn't exist, create it
        let payload = json!({
            "app_title": applicability_name,
            "app_description": format!("Imported applicability: {}", applicability_name),
            "app_tag": applicability_name.to_lowercase().replace(" ", "_")
        });

        let response = self.client
            .post(&format!("{}/api/v1/applicability", self.base_url))
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(1) // Default ID
        } else {
            Err(anyhow!("Failed to create applicability: {}", applicability_name))
        }
    }

    async fn resolve_status(&self, status_name: &str) -> Result<i32> {
        let response = self.client
            .get(&format!("{}/api/v1/status", self.base_url))
            .send()
            .await?;

        let statuses: Vec<Value> = response.json().await?;
        
        for status in statuses {
            if status["st_title"].as_str() == Some(status_name) {
                return Ok(status["st_id"].as_i64().unwrap_or(0) as i32);
            }
        }

        // Default status ID if not found
        Ok(1)
    }

    async fn resolve_verification(&self, _verification_name: &str) -> Result<i32> {
        // For now, return a default verification ID
        // You might need to implement verification resolution based on your API
        Ok(1)
    }

    async fn resolve_user(&self, user_name: &str) -> Result<i32> {
        let response = self.client
            .get(&format!("{}/api/v1/users", self.base_url))
            .send()
            .await?;

        let users: Vec<Value> = response.json().await?;
        
        for user in users {
            if user["user_name"].as_str() == Some(user_name) {
                return Ok(user["user_id"].as_i64().unwrap_or(0) as i32);
            }
        }

        // Default user ID if not found
        Ok(1)
    }

    async fn resolve_requirement_by_title(&self, title: &str) -> Result<i32> {
        let response = self.client
            .get(&format!("{}/api/v1/requirements", self.base_url))
            .send()
            .await?;

        let requirements: Vec<Value> = response.json().await?;
        
        for req in requirements {
            if req["req_title"].as_str() == Some(title) {
                return Ok(req["req_id"].as_i64().unwrap_or(0) as i32);
            }
        }

        Err(anyhow!("Requirement with title '{}' not found", title))
    }

    async fn resolve_test_by_name(&self, name: &str) -> Result<i32> {
        let response = self.client
            .get(&format!("{}/api/v1/tests", self.base_url))
            .send()
            .await?;

        let tests: Vec<Value> = response.json().await?;
        
        for test in tests {
            if test["test_name"].as_str() == Some(name) {
                return Ok(test["test_id"].as_i64().unwrap_or(0) as i32);
            }
        }

        Err(anyhow!("Test with name '{}' not found", name))
    }
} 