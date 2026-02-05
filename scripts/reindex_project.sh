#!/bin/bash

# Configuration
DEFAULT_URL="http://localhost:8000"
COOKIE_FILE="reindex_cookies.txt"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}ReqMan Reindexing Tool${NC}"
echo "----------------------"

# 1. Get Base URL
read -p "Enter Base URL [${DEFAULT_URL}]: " BASE_URL
BASE_URL=${BASE_URL:-$DEFAULT_URL}

# 2. Get Credentials
echo ""
echo "Please enter admin credentials:"
read -p "Username: " USERNAME
read -s -p "Password: " PASSWORD
echo ""

# 3. Get Project ID
echo ""
read -p "Enter Project ID to reindex: " PROJECT_ID

if [ -z "$PROJECT_ID" ]; then
    echo -e "${RED}Error: Project ID is required.${NC}"
    exit 1
fi

# 4. Login
echo ""
echo "Logging in as $USERNAME..."

# Clear user input variable
rm -f "$COOKIE_FILE"

LOGIN_RESPONSE=$(curl -s -c "$COOKIE_FILE" -w "%{http_code}" -d "username=$USERNAME&password=$PASSWORD" -X POST "$BASE_URL/login")
HTTP_CODE=${LOGIN_RESPONSE: -3}

# Check for redirect (303) which indicates successful login in Rocket
if [ "$HTTP_CODE" == "303" ] || [ "$HTTP_CODE" == "200" ]; then
    echo -e "${GREEN}Login successful!${NC}"
else
    echo -e "${RED}Login failed. HTTP Code: $HTTP_CODE${NC}"
    echo "Response: ${LOGIN_RESPONSE::-3}"
    rm -f "$COOKIE_FILE"
    exit 1
fi

# 5. Reindex
echo ""
echo "Triggering reindex for Project $PROJECT_ID..."

REINDEX_URL="$BASE_URL/api/projects/$PROJECT_ID/requirements/reindex"
REINDEX_RESPONSE=$(curl -s -b "$COOKIE_FILE" -X POST "$REINDEX_URL")

echo "Response:"
echo "$REINDEX_RESPONSE"

# 6. Check status
echo ""
echo "Checking index status..."
sleep 2
STATUS_URL="$BASE_URL/api/projects/$PROJECT_ID/requirements/index_status"
curl -s -b "$COOKIE_FILE" "$STATUS_URL"

# Cleanup
rm -f "$COOKIE_FILE"
echo ""
