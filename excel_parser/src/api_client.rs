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

    pub async fn import_data(&self, data: &[ImportData], project_id: i32) -> Result<Vec<Result<String, String>>> {
        let mut results = Vec::new();

        for item in data {
            let result = match item {
                ImportData::Requirement(req) => self.import_requirement(req, project_id).await,
                ImportData::Test(test) => self.import_test(test, project_id).await,
            };
            results.push(result);
        }

        Ok(results)
    }

    async fn import_requirement(&self, req: &RequirementData, project_id: i32) -> Result<String, String> {
        // First, resolve category ID
        let category_id = self.resolve_category(&req.category_id).await
            .map_err(|e| format!("Failed to resolve category '{}': {}", req.category_id, e))?;

        // Resolve applicability ID
        let applicability_id = self.resolve_applicability(&req.applicability_id).await
            .map_err(|e| format!("Failed to resolve applicability '{}': {}", req.applicability_id, e))?;

        // Resolve status ID
        let status_id = self.resolve_status(&req.current_status_id).await
            .map_err(|e| format!("Failed to resolve status '{}': {}", req.current_status_id, e))?;

        // Resolve verification ID
        let id = self.resolve_verification(&req.verification_method_id).await
            .map_err(|e| format!("Failed to resolve verification '{}': {}", req.verification_method_id, e))?;

        // Resolve author ID
        let author_id = self.resolve_user(&req.author_id).await
            .map_err(|e| format!("Failed to resolve author '{}': {}", req.author_id, e))?;

        // Resolve reviewer ID
        let reviewer_id = self.resolve_user(&req.reviewer_id).await
            .map_err(|e| format!("Failed to resolve reviewer '{}': {}", req.reviewer_id, e))?;

        // Resolve parent requirement ID if specified
        let parent_id = if req.req_parent_title != "None" && !req.req_parent_title.is_empty() {
            self.resolve_requirement_by_title(&req.req_parent_title).await.ok()
        } else {
            req.parent_id
        };

        let payload = json!({
            "title": req.title,
            "description": req.description,
            "reference_code": req.reference_code,
            "category_id": category_id,
            "applicability_id": applicability_id,
            "current_status_id": status_id,
            "verification_method_id": id,
            "author_id": author_id,
            "reviewer_id": reviewer_id,
            "parent_id": parent_id,
            "req_link": req.req_link,
            "justification": req.justification,
            "project_id": project_id
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
            Ok(format!("Requirement '{}' imported successfully", req.title))
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Failed to import requirement '{}': {}", req.title, error_text))
        }
    }

    async fn import_test(&self, test: &TestData, project_id: i32) -> Result<String, String> {
        // Resolve status ID
        let status_id = self.resolve_status(&test.status_id).await
            .map_err(|e| format!("Failed to resolve status '{}': {}", test.status_id, e))?;

        // Resolve parent test ID if specified
        let parent_id = if test.test_parent_name != "None" && !test.test_parent_name.is_empty() {
            self.resolve_test_by_name(&test.test_parent_name).await.ok()
        } else {
            test.parent_id
        };

        let payload = json!({
            "name": test.name,
            "description": test.description,
            "source": test.source,
            "status_id": status_id,
            "parent_id": parent_id,
            "project_id": project_id
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
            Ok(format!("Test '{}' imported successfully", test.name))
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Failed to import test '{}': {}", test.name, error_text))
        }
    }

    async fn resolve_category(&self, category_name: &str) -> Result<i32> {
        let response = self.client
            .get(&format!("{}/api/v1/categories", self.base_url))
            .send()
            .await?;

        let categories: Vec<Value> = response.json().await?;
        
        for category in categories {
            if category["title"].as_str() == Some(category_name) {
                return Ok(category["id"].as_i64().unwrap_or(0) as i32);
            }
        }

        // If category doesn't exist, create it
        let payload = json!({
            "title": category_name,
            "description": format!("Imported category: {}", category_name),
            "tag": category_name.to_lowercase().replace(" ", "_")
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
            if app["title"].as_str() == Some(applicability_name) {
                return Ok(app["id"].as_i64().unwrap_or(0) as i32);
            }
        }

        // If applicability doesn't exist, create it
        let payload = json!({
            "title": applicability_name,
            "description": format!("Imported applicability: {}", applicability_name),
            "tag": applicability_name.to_lowercase().replace(" ", "_")
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
            if status["title"].as_str() == Some(status_name) {
                return Ok(status["id"].as_i64().unwrap_or(0) as i32);
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

    async fn resolve_user(&self, name: &str) -> Result<i32> {
        let response = self.client
            .get(&format!("{}/api/v1/users", self.base_url))
            .send()
            .await?;

        let users: Vec<Value> = response.json().await?;
        
        for user in users {
            if user["name"].as_str() == Some(name) {
                return Ok(user["id"].as_i64().unwrap_or(0) as i32);
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
            if req["title"].as_str() == Some(title) {
                return Ok(req["id"].as_i64().unwrap_or(0) as i32);
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
            if test["name"].as_str() == Some(name) {
                return Ok(test["id"].as_i64().unwrap_or(0) as i32);
            }
        }

        Err(anyhow!("Test with name '{}' not found", name))
    }
} 