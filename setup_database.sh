#!/bin/bash

# =============================================================================
# ReqMan Database Setup Script
# =============================================================================
# This script automates the complete setup of the ReqMan database
# =============================================================================

set -e  # Exit on any error

echo "=========================================="
echo "ReqMan Database Setup Script"
echo "=========================================="
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Error: Docker is not running. Please start Docker first."
    exit 1
fi

# Check if the database container is running
if ! docker ps | grep -q reqman_db_1; then
    echo "❌ Error: Database container 'reqman_db_1' is not running."
    echo "Please start the database with: docker-compose up -d"
    exit 1
fi

echo "✅ Database container is running"
echo ""

# Create database if it doesn't exist
echo "📊 Creating database 'reqman' if it doesn't exist..."
docker exec reqman_db_1 psql -U rust -d postgres -c "SELECT 1 FROM pg_database WHERE datname='reqman'" | grep -q 1 || \
docker exec reqman_db_1 psql -U rust -d postgres -c "CREATE DATABASE reqman;"

echo "✅ Database 'reqman' is ready"
echo ""

# Drop all tables if they exist (clean slate)
echo "🧹 Cleaning existing tables..."
docker exec reqman_db_1 psql -U rust -d reqman -c "
DROP TABLE IF EXISTS matrix CASCADE;
DROP TABLE IF EXISTS logs CASCADE;
DROP TABLE IF EXISTS requirements CASCADE;
DROP TABLE IF EXISTS tests CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS applicability CASCADE;
DROP TABLE IF EXISTS verification CASCADE;
DROP TABLE IF EXISTS status CASCADE;
DROP TABLE IF EXISTS projects CASCADE;
" > /dev/null 2>&1

echo "✅ Database cleaned"
echo ""

# Run the complete initialization script
echo "🚀 Initializing database with complete schema and data..."
docker exec -i reqman_db_1 psql -U rust -d reqman < init_complete.sql

echo ""
echo "✅ Database initialization completed successfully!"
echo ""

# Verify the setup
echo "🔍 Verifying database setup..."
echo ""

# Check tables
echo "📋 Tables created:"
docker exec reqman_db_1 psql -U rust -d reqman -c "\dt" | grep -E "(projects|users|requirements|tests|matrix|logs|categories|applicability|verification|status)"

echo ""
echo "👥 Users created:"
docker exec reqman_db_1 psql -U rust -d reqman -c "SELECT user_username, user_name, is_admin FROM users ORDER BY user_id;"

echo ""
echo "📁 Projects created:"
docker exec reqman_db_1 psql -U rust -d reqman -c "SELECT project_id, project_name, project_status FROM projects ORDER BY project_id;"

echo ""
echo "📊 Sample data counts:"
docker exec reqman_db_1 psql -U rust -d reqman -c "
SELECT 
    'Requirements' as entity, COUNT(*) as count FROM requirements
UNION ALL
SELECT 'Tests', COUNT(*) FROM tests
UNION ALL
SELECT 'Matrix Links', COUNT(*) FROM matrix
UNION ALL
SELECT 'Categories', COUNT(*) FROM categories
UNION ALL
SELECT 'Applicability', COUNT(*) FROM applicability
UNION ALL
SELECT 'Logs', COUNT(*) FROM logs;
"

echo ""
echo "=========================================="
echo "🎉 ReqMan Database Setup Complete!"
echo "=========================================="
echo ""
echo "📝 Login Credentials (all users have password: 'password'):"
echo "   • alice (Admin) - Alice Johnson"
echo "   • dr_smith (Admin) - Dr. Sarah Smith"
echo "   • eng_jones - Engineer Mike Jones"
echo "   • tech_lee - Technician Lisa Lee"
echo "   • qa_wilson - QA Specialist Tom Wilson"
echo "   • admin (Admin) - System Administrator"
echo ""
echo "🌐 Application URL: http://localhost:8000"
echo ""
echo "🚀 To start the application:"
echo "   cargo run --bin req_man"
echo ""
echo "=========================================="
