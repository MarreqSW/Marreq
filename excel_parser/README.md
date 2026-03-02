# Excel Parser for Marreq

A command-line tool to parse Excel files exported from Marreq and import the data back into the Marreq API.

## Features

- 🔍 **Excel File Parsing**: Parse requirements and tests from Excel files
- 🔄 **API Integration**: Direct import into Marreq API
- 📊 **Data Validation**: Automatic resolution of relationships and references
- 💾 **JSON Export**: Generate JSON files for manual review
- 🧪 **Dry Run Mode**: Preview data without making API calls
- 🎯 **Smart Resolution**: Automatically create missing categories, applicability, and users

## Installation

```bash
cd excel_parser
cargo build --release
```

## Usage

### Basic Usage

```bash
# Parse and import requirements
./target/release/excel_parser -f requirements.xls

# Parse and import tests
./target/release/excel_parser -f tests.xls

# Use custom API URL
./target/release/excel_parser -f requirements.xls --api-url http://localhost:8000
```

### Advanced Options

```bash
# Dry run - preview data without importing
./target/release/excel_parser -f requirements.xls --dry-run

# Generate JSON file only
./target/release/excel_parser -f requirements.xls --json-only -o output.json

# Generate JSON and import to API
./target/release/excel_parser -f requirements.xls -o output.json
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --file` | Path to Excel file | Required |
| `-u, --api-url` | Marreq API base URL | `http://127.0.0.1:8000` |
| `-o, --output` | Output JSON file path | None |
| `--dry-run` | Preview data without API calls | false |
| `--json-only` | Generate JSON only, skip API | false |

## Supported Excel Formats

### Requirements Excel Format

The parser expects Excel files with the following headers:

| Column | Description | Required |
|--------|-------------|----------|
| Req ID | Requirement ID | No |
| Title | Requirement title | Yes |
| Description | Requirement description | Yes |
| Reference | Requirement reference | Yes |
| Category | Category name | Yes |
| Applicability | Applicability name | Yes |
| Status | Status name | Yes |
| Verification | Verification type | Yes |
| Author | Author name | Yes |
| Reviewer | Reviewer name | Yes |
| Parent | Parent requirement ID | No |
| Parent Title | Parent requirement title | No |
| Link | External link | No |
| Creation Date | Creation date | No |
| Update Date | Update date | No |
| Deadline Date | Deadline date | No |
| Justification | Justification text | No |

### Tests Excel Format

The parser expects Excel files with the following headers:

| Column | Description | Required |
|--------|-------------|----------|
| Test ID | Test ID | No |
| Name | Test name | Yes |
| Description | Test description | Yes |
| Status | Test status | Yes |
| Source | Test source | Yes |
| Parent ID | Parent test ID | No |
| Parent Name | Parent test name | No |

## Data Resolution

The parser automatically resolves the following relationships:

### Categories
- If a category doesn't exist, it will be created automatically
- Category tag is generated from the category name

### Applicability
- If an applicability doesn't exist, it will be created automatically
- Applicability tag is generated from the applicability name

### Users
- Author and reviewer names are resolved to user IDs
- If a user doesn't exist, a default user ID is used

### Status
- Status names are resolved to status IDs
- If a status doesn't exist, a default status ID is used

### Parent Relationships
- Parent requirements/tests are resolved by title/name
- If parent doesn't exist, the relationship is skipped

## Examples

### Import Requirements

```bash
# Import requirements from Excel file
./target/release/excel_parser -f requirements.xls

# Preview data first
./target/release/excel_parser -f requirements.xls --dry-run

# Save to JSON and import
./target/release/excel_parser -f requirements.xls -o requirements.json
```

### Import Tests

```bash
# Import tests from Excel file
./target/release/excel_parser -f tests.xls

# Use custom API URL
./target/release/excel_parser -f tests.xls --api-url http://my-marreq-server:8000
```

## Error Handling

The parser provides detailed error messages for:

- **File not found**: Excel file doesn't exist
- **Invalid format**: Excel file doesn't have expected headers
- **API errors**: Network issues or server errors
- **Data validation**: Missing required fields or invalid data

## Output

### Console Output

```
🔍 Excel Parser for Marreq
📁 File: requirements.xls
🌐 API URL: http://127.0.0.1:8000
✅ Parsed 20 records
📤 API Import Results:
✅ Success: Requirement 'User Authentication' imported successfully
✅ Success: Requirement 'Data Validation' imported successfully
🎉 Processing complete!
```

### JSON Output

When using `-o` option, the parser generates a JSON file with the parsed data:

```json
[
  {
    "id": 1,
    "title": "User Authentication",
    "description": "System shall provide user authentication",
    "reference_code": "REQ-001",
    "category_id": "Security",
    "applicability_id": "All Systems",
    "status_id": "Draft",
    "verification_method_id": "Test",
    "author_id": "Alice",
    "reviewer_id": "Bob",
    "parent_id": null,
    "req_parent_title": "None",
    "req_link": "",
    "creation_date": "2025-08-03",
    "update_date": "2025-08-03",
    "deadline_date": "2025-12-31",
    "justification": "Security requirement"
  }
]
```

## Dependencies

- **calamine**: Excel file parsing
- **reqwest**: HTTP client for API calls
- **serde**: JSON serialization/deserialization
- **clap**: Command-line argument parsing
- **tokio**: Async runtime
- **anyhow**: Error handling

## Development

### Building

```bash
cargo build
cargo build --release
```

### Testing

```bash
cargo test
```

### Running

```bash
cargo run -- -f requirements.xls --dry-run
```

## License

This project is part of the Marreq ecosystem and follows the same license terms. 