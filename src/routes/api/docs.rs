//! API documentation and OpenAPI specification.
//!
//! This module provides API documentation using OpenAPI 3.0 specification.

use rocket::serde::json::Json;
use serde_json::Value;

/// OpenAPI 3.0 specification for the ReqMan API
pub fn openapi_spec() -> Value {
    serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "ReqMan API",
            "description": "Requirements Management System API",
            "version": "1.0.0",
            "contact": {
                "name": "ReqMan Support",
                "email": "support@reqman.example.com"
            }
        },
        "servers": [
            {
                "url": "http://localhost:8000/api/v1",
                "description": "Development server"
            }
        ],
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "description": "Check if the API is running",
                    "responses": {
                        "200": {
                            "description": "API is healthy",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/version": {
                "get": {
                    "summary": "Get API version",
                    "description": "Get API version information",
                    "responses": {
                        "200": {
                            "description": "Version information",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiResponse"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/requirements": {
                "get": {
                    "summary": "Get all requirements",
                    "description": "Retrieve all requirements",
                    "parameters": [
                        {
                            "name": "project_id",
                            "in": "query",
                            "description": "Filter by project ID",
                            "required": false,
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "List of requirements",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiResponse"
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create a new requirement",
                    "description": "Create a new requirement",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewRequirement"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Requirement created successfully",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Bad request - validation error"
                        }
                    }
                }
            },
            "/requirements/{id}": {
                "get": {
                    "summary": "Get requirement by ID",
                    "description": "Retrieve a specific requirement by its ID",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Requirement ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Requirement found",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiResponse"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Requirement not found"
                        }
                    }
                },
                "put": {
                    "summary": "Update requirement",
                    "description": "Update an existing requirement",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Requirement ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewRequirement"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Requirement updated successfully"
                        },
                        "404": {
                            "description": "Requirement not found"
                        }
                    }
                },
                "delete": {
                    "summary": "Delete requirement",
                    "description": "Delete a requirement",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Requirement ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Requirement deleted successfully"
                        },
                        "404": {
                            "description": "Requirement not found"
                        }
                    }
                }
            },
            "/tests": {
                "get": {
                    "summary": "Get all tests",
                    "description": "Retrieve all test cases",
                    "parameters": [
                        {
                            "name": "project_id",
                            "in": "query",
                            "description": "Filter by project ID",
                            "required": false,
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "List of tests",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/ApiResponse"
                                    }
                                }
                            }
                        }
                    }
                },
                "post": {
                    "summary": "Create a new test",
                    "description": "Create a new test case",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewTest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Test created successfully"
                        },
                        "400": {
                            "description": "Bad request - validation error"
                        }
                    }
                }
            },
            "/tests/{id}": {
                "get": {
                    "summary": "Get test by ID",
                    "description": "Retrieve a specific test by its ID",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Test ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Test found"
                        },
                        "404": {
                            "description": "Test not found"
                        }
                    }
                },
                "put": {
                    "summary": "Update test",
                    "description": "Update an existing test",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Test ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewTest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Test updated successfully"
                        },
                        "404": {
                            "description": "Test not found"
                        }
                    }
                },
                "delete": {
                    "summary": "Delete test",
                    "description": "Delete a test",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Test ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Test deleted successfully"
                        },
                        "404": {
                            "description": "Test not found"
                        }
                    }
                }
            },
            "/categories": {
                "get": {
                    "summary": "Get all categories",
                    "description": "Retrieve all requirement categories",
                    "responses": {
                        "200": {
                            "description": "List of categories"
                        }
                    }
                },
                "post": {
                    "summary": "Create a new category",
                    "description": "Create a new requirement category",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewCategory"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Category created successfully"
                        }
                    }
                }
            },
            "/categories/{id}": {
                "get": {
                    "summary": "Get category by ID",
                    "description": "Retrieve a specific category by its ID",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Category ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Category found"
                        },
                        "404": {
                            "description": "Category not found"
                        }
                    }
                },
                "put": {
                    "summary": "Update category",
                    "description": "Update an existing category",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Category ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewCategory"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Category updated successfully"
                        }
                    }
                },
                "delete": {
                    "summary": "Delete category",
                    "description": "Delete a category",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Category ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Category deleted successfully"
                        }
                    }
                }
            },
            "/applicability": {
                "get": {
                    "summary": "Get all applicability options",
                    "description": "Retrieve all applicability options",
                    "responses": {
                        "200": {
                            "description": "List of applicability options"
                        }
                    }
                },
                "post": {
                    "summary": "Create a new applicability",
                    "description": "Create a new applicability option",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewApplicability"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Applicability created successfully"
                        }
                    }
                }
            },
            "/applicability/{id}": {
                "get": {
                    "summary": "Get applicability by ID",
                    "description": "Retrieve a specific applicability by its ID",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Applicability ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Applicability found"
                        },
                        "404": {
                            "description": "Applicability not found"
                        }
                    }
                },
                "put": {
                    "summary": "Update applicability",
                    "description": "Update an existing applicability",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Applicability ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewApplicability"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Applicability updated successfully"
                        }
                    }
                },
                "delete": {
                    "summary": "Delete applicability",
                    "description": "Delete an applicability",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Applicability ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Applicability deleted successfully"
                        }
                    }
                }
            },
            "/users": {
                "get": {
                    "summary": "Get all users",
                    "description": "Retrieve all users",
                    "responses": {
                        "200": {
                            "description": "List of users"
                        }
                    }
                },
                "post": {
                    "summary": "Create a new user",
                    "description": "Create a new user",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewUser"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "User created successfully"
                        }
                    }
                }
            },
            "/users/{id}": {
                "get": {
                    "summary": "Get user by ID",
                    "description": "Retrieve a specific user by their ID",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "User ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "User found"
                        },
                        "404": {
                            "description": "User not found"
                        }
                    }
                },
                "delete": {
                    "summary": "Delete user",
                    "description": "Delete a user",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "User ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "User deleted successfully"
                        }
                    }
                }
            },
            "/projects": {
                "get": {
                    "summary": "Get all projects",
                    "description": "Retrieve all projects",
                    "responses": {
                        "200": {
                            "description": "List of projects"
                        }
                    }
                },
                "post": {
                    "summary": "Create a new project",
                    "description": "Create a new project",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewProject"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Project created successfully"
                        }
                    }
                }
            },
            "/projects/{id}": {
                "get": {
                    "summary": "Get project by ID",
                    "description": "Retrieve a specific project by its ID",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Project ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Project found"
                        },
                        "404": {
                            "description": "Project not found"
                        }
                    }
                },
                "put": {
                    "summary": "Update project",
                    "description": "Update an existing project",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Project ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewProject"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Project updated successfully"
                        }
                    }
                },
                "delete": {
                    "summary": "Delete project",
                    "description": "Delete a project",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "description": "Project ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Project deleted successfully"
                        }
                    }
                }
            },
            "/status": {
                "get": {
                    "summary": "Get all status options",
                    "description": "Retrieve all status options",
                    "responses": {
                        "200": {
                            "description": "List of status options"
                        }
                    }
                },
                "post": {
                    "summary": "Create a new status",
                    "description": "Create a new status option",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/NewStatus"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Status created successfully"
                        }
                    }
                }
            },
            "/matrix": {
                "get": {
                    "summary": "Get traceability matrix",
                    "description": "Retrieve the traceability matrix",
                    "parameters": [
                        {
                            "name": "project_id",
                            "in": "query",
                            "description": "Filter by project ID",
                            "required": false,
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Traceability matrix"
                        }
                    }
                },
                "post": {
                    "summary": "Create matrix link",
                    "description": "Create a new traceability link between requirement and test",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/MatrixLinkRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Matrix link created successfully"
                        }
                    }
                }
            },
            "/matrix/{req_id}/{test_id}": {
                "delete": {
                    "summary": "Delete matrix link",
                    "description": "Delete a traceability link between requirement and test",
                    "parameters": [
                        {
                            "name": "req_id",
                            "in": "path",
                            "required": true,
                            "description": "Requirement ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        },
                        {
                            "name": "test_id",
                            "in": "path",
                            "required": true,
                            "description": "Test ID",
                            "schema": {
                                "type": "integer",
                                "format": "int32"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Matrix link deleted successfully"
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "ApiResponse": {
                    "type": "object",
                    "properties": {
                        "success": {
                            "type": "boolean",
                            "description": "Whether the operation was successful"
                        },
                        "data": {
                            "description": "Response data (present when success is true)"
                        },
                        "error": {
                            "type": "string",
                            "description": "Error message (present when success is false)"
                        },
                        "timestamp": {
                            "type": "string",
                            "format": "date-time",
                            "description": "Response timestamp"
                        }
                    },
                    "required": ["success", "timestamp"]
                },
                "Requirement": {
                    "type": "object",
                    "properties": {
                        "req_id": {"type": "integer", "format": "int32"},
                        "req_title": {"type": "string"},
                        "req_description": {"type": "string"},
                        "req_verification": {"type": "integer", "format": "int32"},
                        "req_current_status": {"type": "integer", "format": "int32"},
                        "req_author": {"type": "integer", "format": "int32"},
                        "req_reviewer": {"type": "integer", "format": "int32"},
                        "req_link": {"type": "string"},
                        "req_reference": {"type": "string"},
                        "req_category": {"type": "integer", "format": "int32"},
                        "req_parent": {"type": "integer", "format": "int32"},
                        "req_creation_date": {"type": "string", "format": "date-time"},
                        "req_update_date": {"type": "string", "format": "date-time"},
                        "req_deadline_date": {"type": "string", "format": "date-time"},
                        "req_applicability": {"type": "integer", "format": "int32"},
                        "req_justification": {"type": "string", "nullable": true},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["req_id", "req_title", "req_description", "req_verification", "req_current_status", "req_author", "req_reviewer", "req_link", "req_reference", "req_category", "req_parent", "req_creation_date", "req_update_date", "req_deadline_date", "req_applicability", "project_id"]
                },
                "NewRequirement": {
                    "type": "object",
                    "properties": {
                        "req_id": {"type": "integer", "format": "int32", "nullable": true},
                        "req_title": {"type": "string"},
                        "req_description": {"type": "string"},
                        "req_verification": {"type": "integer", "format": "int32"},
                        "req_current_status": {"type": "integer", "format": "int32"},
                        "req_author": {"type": "integer", "format": "int32"},
                        "req_reviewer": {"type": "integer", "format": "int32"},
                        "req_link": {"type": "string"},
                        "req_reference": {"type": "string"},
                        "req_category": {"type": "integer", "format": "int32"},
                        "req_parent": {"type": "integer", "format": "int32"},
                        "req_creation_date": {"type": "string", "format": "date-time", "nullable": true},
                        "req_update_date": {"type": "string", "format": "date-time", "nullable": true},
                        "req_deadline_date": {"type": "string", "format": "date-time", "nullable": true},
                        "req_applicability": {"type": "integer", "format": "int32"},
                        "req_justification": {"type": "string", "nullable": true},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["req_title", "req_description", "req_verification", "req_current_status", "req_author", "req_reviewer", "req_link", "req_reference", "req_category", "req_parent", "req_applicability", "project_id"]
                },
                "Test": {
                    "type": "object",
                    "properties": {
                        "test_id": {"type": "integer", "format": "int32"},
                        "test_name": {"type": "string"},
                        "test_description": {"type": "string"},
                        "test_source": {"type": "string"},
                        "test_status": {"type": "integer", "format": "int32"},
                        "test_parent": {"type": "integer", "format": "int32"},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["test_id", "test_name", "test_description", "test_source", "test_status", "test_parent", "project_id"]
                },
                "NewTest": {
                    "type": "object",
                    "properties": {
                        "test_id": {"type": "integer", "format": "int32", "nullable": true},
                        "test_name": {"type": "string"},
                        "test_description": {"type": "string"},
                        "test_source": {"type": "string"},
                        "test_status": {"type": "integer", "format": "int32"},
                        "test_parent": {"type": "integer", "format": "int32"},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["test_name", "test_description", "test_source", "test_status", "test_parent", "project_id"]
                },
                "Category": {
                    "type": "object",
                    "properties": {
                        "cat_id": {"type": "integer", "format": "int32"},
                        "cat_title": {"type": "string"},
                        "cat_description": {"type": "string"},
                        "cat_tag": {"type": "string"},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["cat_id", "cat_title", "cat_description", "cat_tag", "project_id"]
                },
                "NewCategory": {
                    "type": "object",
                    "properties": {
                        "cat_id": {"type": "integer", "format": "int32", "nullable": true},
                        "cat_title": {"type": "string"},
                        "cat_description": {"type": "string"},
                        "cat_tag": {"type": "string"},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["cat_title", "cat_description", "cat_tag", "project_id"]
                },
                "Applicability": {
                    "type": "object",
                    "properties": {
                        "app_id": {"type": "integer", "format": "int32"},
                        "app_title": {"type": "string"},
                        "app_description": {"type": "string"},
                        "app_tag": {"type": "string"},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["app_id", "app_title", "app_description", "app_tag", "project_id"]
                },
                "NewApplicability": {
                    "type": "object",
                    "properties": {
                        "app_id": {"type": "integer", "format": "int32", "nullable": true},
                        "app_title": {"type": "string"},
                        "app_description": {"type": "string"},
                        "app_tag": {"type": "string"},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["app_title", "app_description", "app_tag", "project_id"]
                },
                "User": {
                    "type": "object",
                    "properties": {
                        "user_id": {"type": "integer", "format": "int32"},
                        "user_username": {"type": "string"},
                        "user_name": {"type": "string"},
                        "user_email": {"type": "string", "nullable": true},
                        "is_admin": {"type": "boolean"},
                        "project_id": {"type": "integer", "format": "int32", "nullable": true}
                    },
                    "required": ["user_id", "user_username", "user_name", "is_admin"]
                },
                "NewUser": {
                    "type": "object",
                    "properties": {
                        "user_username": {"type": "string"},
                        "user_name": {"type": "string"},
                        "user_email": {"type": "string", "nullable": true},
                        "user_password": {"type": "string"},
                        "is_admin": {"type": "boolean"},
                        "project_id": {"type": "integer", "format": "int32", "nullable": true}
                    },
                    "required": ["user_username", "user_name", "user_password", "is_admin"]
                },
                "Project": {
                    "type": "object",
                    "properties": {
                        "project_id": {"type": "integer", "format": "int32"},
                        "project_name": {"type": "string"},
                        "project_description": {"type": "string", "nullable": true}
                    },
                    "required": ["project_id", "project_name"]
                },
                "NewProject": {
                    "type": "object",
                    "properties": {
                        "project_name": {"type": "string"},
                        "project_description": {"type": "string", "nullable": true}
                    },
                    "required": ["project_name"]
                },
                "Status": {
                    "type": "object",
                    "properties": {
                        "status_id": {"type": "integer", "format": "int32"},
                        "status_name": {"type": "string"},
                        "status_description": {"type": "string", "nullable": true}
                    },
                    "required": ["status_id", "status_name"]
                },
                "NewStatus": {
                    "type": "object",
                    "properties": {
                        "status_name": {"type": "string"},
                        "status_description": {"type": "string", "nullable": true}
                    },
                    "required": ["status_name"]
                },
                "Matrix": {
                    "type": "object",
                    "properties": {
                        "matrix_req_id": {"type": "integer", "format": "int32"},
                        "matrix_test_id": {"type": "integer", "format": "int32"},
                        "matrix_creation_date": {"type": "string", "format": "date-time"},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["matrix_req_id", "matrix_test_id", "matrix_creation_date", "project_id"]
                },
                "MatrixLinkRequest": {
                    "type": "object",
                    "properties": {
                        "req_id": {"type": "integer", "format": "int32"},
                        "test_id": {"type": "integer", "format": "int32"},
                        "project_id": {"type": "integer", "format": "int32"}
                    },
                    "required": ["req_id", "test_id", "project_id"]
                }
            }
        }
    })
}

/// Get OpenAPI specification
#[get("/openapi.json")]
pub fn get_openapi_spec() -> Json<Value> {
    Json(openapi_spec())
}

/// Get API documentation page
#[get("/docs")]
pub fn get_api_docs() -> rocket::response::content::RawHtml<String> {
    let _spec = openapi_spec();
    let html = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>ReqMan API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui.css" />
    <style>
        html {{
            box-sizing: border-box;
            overflow: -moz-scrollbars-vertical;
            overflow-y: scroll;
        }}
        *, *:before, *:after {{
            box-sizing: inherit;
        }}
        body {{
            margin:0;
            background: #fafafa;
        }}
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {{
            const ui = SwaggerUIBundle({{
                url: '/api/v1/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout"
            }});
        }};
    </script>
</body>
</html>
    "#);
    rocket::response::content::RawHtml(html)
}
