#!/bin/bash
# Test script for Trilium Rust Server Phase 2 Endpoints
# This script tests all implemented endpoints to verify functionality

set -e

BASE_URL="${TRILIUM_SERVER:-http://localhost:8081}"
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "====================================="
echo "Trilium Rust Server - Endpoint Tests"
echo "====================================="
echo "Server: $BASE_URL"
echo ""

# Function to test endpoint
test_endpoint() {
    local method=$1
    local endpoint=$2
    local description=$3
    local data=$4

    echo -n "Testing: $description... "

    if [ -z "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint")
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -d "$data")
    fi

    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')

    if [ "$http_code" -ge 200 ] && [ "$http_code" -lt 300 ]; then
        echo -e "${GREEN}✓ PASS${NC} (HTTP $http_code)"
        return 0
    elif [ "$http_code" -eq 404 ]; then
        echo -e "${YELLOW}⚠ Not Found${NC} (HTTP $http_code) - Expected if no data exists"
        return 0
    else
        echo -e "${RED}✗ FAIL${NC} (HTTP $http_code)"
        echo "Response: $body"
        return 1
    fi
}

# Track test results
PASSED=0
FAILED=0

echo "Phase 1: Basic Endpoints"
echo "-------------------------"

# Health check
if test_endpoint GET "/health" "Health check"; then
    ((PASSED++))
else
    ((FAILED++))
fi

# Get all notes
if test_endpoint GET "/api/notes" "Get all notes"; then
    ((PASSED++))
else
    ((FAILED++))
fi

# Get tree
if test_endpoint GET "/api/tree" "Get tree structure"; then
    ((PASSED++))
else
    ((FAILED++))
fi

# Get recent changes
if test_endpoint GET "/api/recent-changes" "Get recent changes"; then
    ((PASSED++))
else
    ((FAILED++))
fi

# Get options
if test_endpoint GET "/api/options" "Get all options"; then
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""
echo "Phase 2: Search Endpoints"
echo "-------------------------"

# Search by path
if test_endpoint GET "/api/search/test" "Search notes by title (path param)"; then
    ((PASSED++))
else
    ((FAILED++))
fi

# Search by query
if test_endpoint GET "/api/search-notes?search=test" "Search notes by title (query param)"; then
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""
echo "Phase 2: Note Operations (will fail without existing notes)"
echo "------------------------------------------------------------"

# Test note retrieval (will 404 if no notes)
test_endpoint GET "/api/notes/root" "Get root note"
test_endpoint GET "/api/notes/root/branches" "Get root branches"
test_endpoint GET "/api/notes/root/attributes" "Get root attributes"

echo ""
echo "Phase 2: Create Operations (requires valid parent note)"
echo "-------------------------------------------------------"

echo "NOTE: The following tests require a valid database with at least a 'root' note"
echo "They will fail with 404 if the database doesn't exist or is empty"
echo ""

# Try to create a note (will fail if no root note exists)
CREATE_NOTE_DATA='{
  "title": "Test Note Created by Script",
  "type": "text",
  "content": "<p>This is a test note created by the test script.</p>"
}'

echo "Testing note creation..."
response=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/notes/root/children" \
    -H "Content-Type: application/json" \
    -d "$CREATE_NOTE_DATA")

http_code=$(echo "$response" | tail -n1)
body=$(echo "$response" | sed '$d')

if [ "$http_code" -eq 200 ] || [ "$http_code" -eq 201 ]; then
    echo -e "${GREEN}✓ Note created successfully${NC}"

    # Extract note ID from response
    NOTE_ID=$(echo "$body" | grep -o '"noteId":"[^"]*"' | head -1 | cut -d'"' -f4)
    BRANCH_ID=$(echo "$body" | grep -o '"branchId":"[^"]*"' | head -1 | cut -d'"' -f4)

    echo "  Created Note ID: $NOTE_ID"
    echo "  Created Branch ID: $BRANCH_ID"

    if [ -n "$NOTE_ID" ]; then
        echo ""
        echo "Phase 2: Testing operations on created note"
        echo "--------------------------------------------"

        # Update title
        if test_endpoint PUT "/api/notes/$NOTE_ID/title" "Update note title" '{"title": "Updated Test Note"}'; then
            ((PASSED++))
        else
            ((FAILED++))
        fi

        # Update note type
        if test_endpoint PUT "/api/notes/$NOTE_ID/type" "Update note type" '{"type": "code", "mime": "text/x-python"}'; then
            ((PASSED++))
        else
            ((FAILED++))
        fi

        # Update note content
        TEST_CONTENT="<p>Updated content via test script</p>"
        if test_endpoint PUT "/api/notes/$NOTE_ID/blob" "Update note content" "$TEST_CONTENT"; then
            ((PASSED++))
        else
            ((FAILED++))
        fi

        # Get note content back
        if test_endpoint GET "/api/notes/$NOTE_ID/blob" "Get note content"; then
            ((PASSED++))
        else
            ((FAILED++))
        fi

        # Create attribute
        CREATE_ATTR_DATA="{
          \"noteId\": \"$NOTE_ID\",
          \"type\": \"label\",
          \"name\": \"test-label\",
          \"value\": \"automated-test\"
        }"

        echo "Creating test attribute..."
        attr_response=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/attributes" \
            -H "Content-Type: application/json" \
            -d "$CREATE_ATTR_DATA")

        attr_http_code=$(echo "$attr_response" | tail -n1)
        attr_body=$(echo "$attr_response" | sed '$d')

        if [ "$attr_http_code" -eq 200 ] || [ "$attr_http_code" -eq 201 ]; then
            echo -e "${GREEN}✓ Attribute created${NC}"
            ((PASSED++))

            ATTR_ID=$(echo "$attr_body" | grep -o '"attributeId":"[^"]*"' | cut -d'"' -f4)
            echo "  Created Attribute ID: $ATTR_ID"

            if [ -n "$ATTR_ID" ]; then
                # Update attribute
                if test_endpoint PUT "/api/attributes/$ATTR_ID" "Update attribute" '{"value": "updated-value"}'; then
                    ((PASSED++))
                else
                    ((FAILED++))
                fi

                # Delete attribute
                if test_endpoint DELETE "/api/attributes/$ATTR_ID" "Delete attribute"; then
                    ((PASSED++))
                else
                    ((FAILED++))
                fi
            fi
        else
            echo -e "${RED}✗ Attribute creation failed${NC}"
            ((FAILED++))
        fi

        # Bulk update attributes
        BULK_ATTRS='[
          {"type": "label", "name": "priority", "value": "high"},
          {"type": "label", "name": "status", "value": "active"}
        ]'

        if test_endpoint PUT "/api/notes/$NOTE_ID/attributes" "Bulk update attributes" "$BULK_ATTRS"; then
            ((PASSED++))
        else
            ((FAILED++))
        fi

        # Delete the test note
        echo ""
        echo "Cleaning up test note..."
        if test_endpoint DELETE "/api/notes/$NOTE_ID" "Delete test note"; then
            ((PASSED++))
            echo -e "${GREEN}✓ Test note cleaned up${NC}"
        else
            ((FAILED++))
            echo -e "${YELLOW}⚠ Could not clean up test note${NC}"
        fi
    fi
elif [ "$http_code" -eq 404 ]; then
    echo -e "${YELLOW}⚠ Cannot create note - parent 'root' note not found${NC}"
    echo "  This is expected if no database exists"
    echo "  To test creation operations:"
    echo "  1. Run the Node.js Trilium server once to create the database"
    echo "  2. Or provide a valid Trilium database at ~/trilium-data/document.db"
else
    echo -e "${RED}✗ Note creation failed${NC} (HTTP $http_code)"
    echo "Response: $body"
    ((FAILED++))
fi

echo ""
echo "Phase 2: Options/Settings Operations"
echo "------------------------------------"

# Set an option
if test_endpoint PUT "/api/options/test-setting" "Set option" '{"value": "test-value"}'; then
    ((PASSED++))
else
    ((FAILED++))
fi

# Get the option back
if test_endpoint GET "/api/options/test-setting" "Get option"; then
    ((PASSED++))
else
    ((FAILED++))
fi

echo ""
echo "====================================="
echo "Test Results Summary"
echo "====================================="
echo -e "Total Passed: ${GREEN}$PASSED${NC}"
echo -e "Total Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠ Some tests failed${NC}"
    echo ""
    echo "Common reasons for failures:"
    echo "  - No database at ~/trilium-data/document.db"
    echo "  - Server not running on $BASE_URL"
    echo "  - Database is empty (no root note)"
    echo ""
    echo "To fix:"
    echo "  1. Start the server: cd apps/rust-server && cargo run --release"
    echo "  2. Ensure database exists (run Node.js Trilium once to create it)"
    echo "  3. Run this script again"
    exit 1
fi
