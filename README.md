# Requirement Manager (ReqMan)

A comprehensive web-based requirements and test management system built with Rust, Rocket, and PostgreSQL. This software provides a complete solution for managing hierarchical requirements, tests, traceability matrices, and generating reports.

## ✨ Features

### 📋 Core Management
- **Multi-Project Support**: Manage multiple projects with isolated data
- **Requirements Management**: Create, edit, and organize hierarchical requirements
- **Test Management**: Manage tests with status tracking and source documentation
- **Traceability Matrix**: Visual mapping between requirements and tests
- **User Management**: Assign authors and reviewers to requirements with authentication

### 🏷️ Advanced Features
- **Categories**: User-defined categories for organizing requirements (project-specific)
- **Applicability**: Define product lines, system types, or project scopes (project-specific)
- **Status Tracking**: Track requirement status (Draft, Accepted, Rejected, etc.)
- **Verification Methods**: Specify verification types (Test, Analysis, Review, etc.)
- **Authentication**: Secure login system with password management
- **Project Isolation**: Data separation between different projects

### 📊 Reporting & Export
- **Excel Export**: Export requirements with all fields to Excel format
- **Matrix Export**: Export traceability matrix to Excel
- **Comprehensive Data**: All metadata included in exports (categories, applicability, dates, etc.)

### 🎨 Modern UI
- **Responsive Design**: Works on desktop and mobile devices
- **Modern Interface**: Clean, card-based layout with consistent styling
- **Intuitive Navigation**: Easy-to-use interface with clear visual hierarchy
- **Professional Styling**: Consistent color scheme and typography

### 🔌 API Access
- **RESTful API**: Complete programmatic access to all data
- **JSON Format**: Standard JSON responses for integration
- **CRUD Operations**: Full Create, Read, Update, Delete support
- **Project-Scoped**: All API operations respect project boundaries

## ToDo List
+ [X] Hierarchy for
  + [X] Requirements
  + [X] Tests
+ [X] Better webpage 
  + [X] Use templates (based on hbs)
  + [X] Modern CSS design system
  + [X] Responsive layout
+ [X] Reports generator
  + [X] Excel export for requirements
  + [X] Excel export for traceability matrix
  + [ ] Latex template
  + [ ] PDF document
+ [X] Categories management
  + [X] CRUD operations
  + [X] API endpoints
+ [X] Applicability management
  + [X] CRUD operations
  + [X] API endpoints
+ [X] REST API (comprehensive)
  + [X] Requirements endpoints
  + [X] Tests endpoints
  + [X] Categories endpoints
  + [X] Applicability endpoints
  + [X] Matrix endpoints
+ [x] Operations logging
+ [X] Parsers for requirements
  + [ ] Latex files (Write a command)
  + [ ] Word files (Write a macro)
  + [X] Excel files
+ [ ] Parsers for tests
  + [ ] Doxygen documentation
  + [ ] ...
+ [X] Multiple projects
+ [X] Optimize DB access
  + [X] Reduce SQL queries
  + [X] DB pool
+ [X] Security
  + [ ] Use https
  + [X] users/admin
+ [ ] Snapshots
  + [ ] Configuration management
+ [ ] Better error management (remove all unwrap())

## 🚀 Quick Start

### Prerequisites

+ **PostgreSQL**: Database backend (provided via Docker)
+ **Docker & Docker Compose**: For database containerization
+ **Rust**: Programming language
+ **clang** /  **libclang-dev**: Required by `xlsxwriter`

### Installation

#### Option 1: Automated Setup (Recommended)

1. **Start the database**:
```bash
docker-compose up -d
```

2. **Run the automated database setup**:
```bash
./setup_database.sh
```

3. **Start the application**:
```bash
cargo run --bin req_man
```

4. **Access the application**:
   - URL: http://localhost:8000
   - Login with any user using password: `password`

#### Option 2: Manual Setup

1. **Install PostgreSQL dependencies** (if not using Docker):
```bash
sudo apt install libpq-dev libpq5 postgresql-client postgresql-client-common
```

2. **Install Diesel CLI**:
```bash
cargo install diesel_cli --no-default-features --features postgres
```

3. **Start the database**:
```bash
docker-compose up -d
```

4. **Create and initialize the database**:
```bash
# Create database
docker exec reqman_db_1 psql -U rust -d postgres -c "CREATE DATABASE reqman;"

# Run complete initialization
docker exec -i reqman_db_1 psql -U rust -d reqman < init_complete.sql
```

5. **Start the application**:
```bash
cargo run --bin req_man
```

The application will be available at **http://localhost:8000**

## 📖 Usage

### Web Interface

1. **Requirements**: Navigate to `/requirements` to view and manage requirements
2. **Tests**: Go to `/tests` to manage test cases
3. **Matrix**: Visit `/matrix` to view the traceability matrix
4. **Categories**: Access `/categories` to manage requirement categories
5. **Applicability**: Visit `/applicability` to manage applicability options

### Export Features

- **Requirements Export**: Click "Export Excel" on the requirements page or homepage
- **Matrix Export**: Click "Export Excel" on the matrix page
- **File Format**: Downloads as `.xls` files with all metadata included

### Import Features

- **Excel Parser**: Standalone application to parse exported Excel files and import data via API
- **Data Import**: Import requirements, tests, and traceability data from Excel files
- **API Integration**: Seamless integration with the main application's REST API

## 🔌 API Reference

### Base URL
```
http://localhost:8000/api/v1
```

### Endpoints

#### Requirements
- `GET /requirements` - List all requirements
- `GET /requirements/{id}` - Get specific requirement
- `POST /requirements` - Create new requirement
- `PATCH /requirements/{id}` - Partially update supported requirement fields
- `DELETE /requirements/{id}` - Delete requirement

#### Tests
- `GET /tests` - List all tests
- `GET /tests/{id}` - Get specific test
- `POST /tests` - Create new test
- `DELETE /tests/{id}` - Delete test

#### Categories
- `GET /categories` - List all categories
- `GET /categories/{id}` - Get specific category
- `POST /categories` - Create new category
- `PUT /categories/{id}` - Update category
- `DELETE /categories/{id}` - Delete category

#### Applicability
- `GET /applicability` - List all applicability options
- `GET /applicability/{id}` - Get specific applicability
- `POST /applicability` - Create new applicability
- `PUT /applicability/{id}` - Update applicability
- `DELETE /applicability/{id}` - Delete applicability

#### Matrix
- `GET /matrix` - Get traceability matrix data

#### Users
- `GET /users` - List all users
- `GET /users/{id}` - Get specific user

#### Status
- `GET /status` - List all status options
- `POST /status` - Create new status

### Example API Usage

```bash
# Get all requirements
curl http://localhost:8000/api/v1/requirements

# Create a new category
curl -X POST http://localhost:8000/api/v1/categories \
  -H "Content-Type: application/json" \
  -d '{"cat_title": "API", "cat_description": "API requirements", "cat_tag": "API"}'

# Export requirements to Excel
curl -O http://localhost:8000/requirements.xls
```

## 🗄️ Database

### Schema
The application uses PostgreSQL with the following main entities:
- **Projects**: Multi-project support with project metadata
- **Requirements**: Core requirement data with metadata and project association
- **Tests**: Test cases with status and source information, project association
- **Matrix**: Traceability links between requirements and tests, project-scoped
- **Categories**: User-defined requirement categories, project-specific
- **Applicability**: User-defined applicability options, project-specific
- **Users**: System users (authors, reviewers) with authentication
- **Status**: Requirement and test status definitions
- **Verification**: Verification method definitions
- **Logs**: Audit trail for all system activities

See the entity diagram: ![ER Diagram](doc/ER%20diagram.png)

### Database Initialization System

The project includes a comprehensive database initialization system that provides:

#### 📁 Initialization Files

- **`init_complete.sql`**: Complete database setup with all tables, constraints, indexes, and sample data
- **`init_simple.sql`**: Simplified version with basic schema and data
- **`setup_database.sh`**: Automated setup script that handles the entire initialization process
- **`DATABASE_SETUP_README.md`**: Detailed documentation for database setup

#### 🔧 Features

- **Complete Schema**: All tables, foreign key constraints, and performance indexes
- **Working Authentication**: Pre-configured users with working bcrypt password hashes
- **Sample Data**: Comprehensive Space Project with requirements, tests, and traceability
- **Multi-Project Support**: Multiple projects with isolated data
- **Audit Logging**: Sample audit trail entries
- **Performance Optimization**: Strategic indexes for common queries

#### 👥 Pre-configured Users

All users have password: `password`

| Username | Name | Role | Project |
|----------|------|------|---------|
| `alice` | Alice Johnson | Admin | ReqMan Project |
| `dr_smith` | Dr. Sarah Smith | Admin | Space Project |
| `eng_jones` | Engineer Mike Jones | User | Space Project |
| `tech_lee` | Technician Lisa Lee | User | Space Project |
| `qa_wilson` | QA Specialist Tom Wilson | User | Space Project |
| `admin` | System Administrator | Admin | ReqMan Project |

#### 🚀 Sample Data

**Space Project** includes:
- **8 Categories**: Power, Communication, Attitude Control, Thermal, etc.
- **6 Applicability**: All Missions, Earth Observation, Communication, etc.
- **4 Verification Methods**: Inspection, Analysis, Demonstration, Test
- **5 Requirements**: Power, communication, and thermal requirements
- **5 Tests**: Corresponding test cases for each requirement
- **5 Matrix Links**: Complete traceability mapping

#### 🔄 Setup Options

**Automated Setup (Recommended)**:
```bash
# Start database and run complete setup
docker-compose up -d
./setup_database.sh
```

**Manual Setup**:
```bash
# Create database and run initialization script
docker exec reqman_db_1 psql -U rust -d postgres -c "CREATE DATABASE reqman;"
docker exec -i reqman_db_1 psql -U rust -d reqman < init_complete.sql
```

**Reset Database**:
```bash
# Complete reset
docker exec reqman_db_1 psql -U rust -d postgres -c "DROP DATABASE IF EXISTS reqman;"
docker exec reqman_db_1 psql -U rust -d postgres -c "CREATE DATABASE reqman;"
./setup_database.sh
```

### Migrations
Database schema changes are managed through Diesel migrations:
```bash
# Create new migration
diesel migration generate migration_name

# Run migrations
diesel migration run

# Revert migrations
diesel migration redo
```

**Note**: The initialization scripts provide a complete, working database setup that bypasses the need for individual migrations during initial setup.

## 🛠️ Development

### Project Structure
```
ReqMan/
├── src/
│   ├── main.rs              # Application entry point
│   ├── models.rs            # Data models
│   ├── schema.rs            # Database schema (auto-generated)
│   ├── helper_functions.rs  # Database operations
│   ├── routes/              # Route handlers
│   ├── generators/          # Report generators
│   └── html/               # Static assets
├── templates/              # Handlebars templates
├── migrations/             # Database migrations
├── excel_parser/           # Standalone Excel parser application
│   ├── src/
│   │   ├── main.rs         # CLI entry point
│   │   ├── parser.rs       # Excel parsing logic
│   │   └── api_client.rs   # API integration
│   └── README.md           # Parser documentation
├── doc/                   # Documentation
├── init_complete.sql      # Complete database initialization script
├── init_simple.sql        # Simplified database initialization
├── setup_database.sh      # Automated database setup script
├── DATABASE_SETUP_README.md # Database setup documentation
└── docker-compose.yml     # Docker database configuration
```

### Key Technologies
- **Backend**: Rust with Rocket web framework
- **Database**: PostgreSQL with Diesel ORM
- **Frontend**: Handlebars templates with custom CSS
- **Reports**: Excel generation with xlsxwriter
- **Containerization**: Docker for database

### Building
```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test
```

## 📝 License

This project is open source. See LICENSE file for details.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 🔧 Troubleshooting

### Common Issues

#### Database Connection Issues
```bash
# Check if database container is running
docker ps | grep reqman_db_1

# Check database connectivity
docker exec reqman_db_1 psql -U rust -d reqman -c "SELECT 1;"

# Restart database container
docker-compose restart
```

#### Application Startup Issues
```bash
# Check if port 8000 is in use
lsof -i :8000

# Kill existing process
kill <PID>

# Start application with specific binary
cargo run --bin req_man
```

#### Login Issues
- **Default credentials**: All users have password `password`
- **Available users**: alice, dr_smith, eng_jones, tech_lee, qa_wilson, admin
- **Reset passwords**: Update database directly or re-run setup script

#### Database Reset
```bash
# Complete database reset
docker exec reqman_db_1 psql -U rust -d postgres -c "DROP DATABASE IF EXISTS reqman;"
./setup_database.sh
```

### Verification Commands

```bash
# Verify database setup
docker exec reqman_db_1 psql -U rust -d reqman -c "\dt"

# Check user creation
docker exec reqman_db_1 psql -U rust -d reqman -c "SELECT user_username, user_name, is_admin FROM users;"

# Verify sample data
docker exec reqman_db_1 psql -U rust -d reqman -c "SELECT COUNT(*) as requirements FROM requirements;"
```

### Performance Issues

- **Database indexes**: The initialization script includes optimized indexes
- **Connection pooling**: Application uses connection pooling for better performance
- **Query optimization**: Consider adding indexes for custom queries

## 📞 Support

For issues and questions, please open an issue on the project repository.

### Getting Help

1. **Check troubleshooting section** above
2. **Review application logs** for error messages
3. **Verify database setup** using verification commands
4. **Check Docker container status**
5. **Open an issue** with detailed error information

