# API Routes Migration Guide

## Quick Navigation

This repository contains three detailed analysis documents to guide the migration of Trilium Notes API endpoints:

### 1. **API_ROUTES_ANALYSIS.md** (608 lines)
Comprehensive analysis of all critical API endpoints organized by priority level.

**Contains:**
- Detailed breakdown of HIGH PRIORITY endpoints (17 endpoints)
- MEDIUM PRIORITY endpoints (19 endpoints)  
- LOWER PRIORITY endpoints (additional features)
- Complete service dependency information
- Implementation notes and considerations

**When to use:** Start here for detailed understanding of specific endpoints

### 2. **API_ROUTES_QUICK_REFERENCE.md** (142 lines)
Quick lookup tables for rapid endpoint reference and planning.

**Contains:**
- Tabular view of all HIGH PRIORITY endpoints
- Tabular view of all MEDIUM PRIORITY endpoints
- List of LOWER PRIORITY endpoints
- Service dependencies at a glance
- Implementation notes summary

**When to use:** Use during daily development for quick lookups

### 3. **API_SERVICE_DEPENDENCIES.md** (380 lines)
Deep dive into service layer dependencies and implementation order.

**Contains:**
- Critical service analysis (becca, blobService, noteService, branchService, searchService)
- Service method signatures and usage patterns
- Endpoints dependent on each service
- Implementation priority ranking
- Database schema requirements
- Testing strategy
- Migration checklist

**When to use:** Planning implementation phases and understanding service interactions

---

## Executive Summary

### Total Endpoints Analyzed: 53+
- **HIGH PRIORITY (17):** Essential for basic note viewing/editing
- **MEDIUM PRIORITY (19):** Important for complete functionality
- **LOWER PRIORITY (17+):** Advanced features

### Migration Phases

#### Phase 1: Foundation (Estimated 2-3 weeks)
Implement minimal API for basic note reading/writing:
- GET /api/tree - Load tree structure
- GET /api/notes/:noteId - Get note metadata
- GET /api/notes/:noteId/blob - Get note content
- PUT /api/notes/:noteId/data - Update content
- Basic branch navigation and attributes

**Services Required:**
- becca (entity cache)
- blobService (content storage)
- noteService (note operations)
- sql & transactions

**Endpoint Coverage:** 40% of essential functionality

#### Phase 2: Full Editing (Estimated 2-3 weeks)
Add complete CRUD, files, attachments, revisions:
- Note creation: POST /api/notes/:parentNoteId/children
- File operations: GET/PUT /api/notes/:noteId/file
- Attachments: Full CRUD support
- Revisions: View and restore history
- Images: Display and upload

**Additional Services:**
- branchService
- optionService
- imageService

**Endpoint Coverage:** 85% of core functionality

#### Phase 3: Advanced Features (Ongoing)
Add search, sync, special notes, scripting:
- Full-text search with snippets
- Entity synchronization
- Date-based special notes
- Bulk actions
- LLM integration

**Additional Services:**
- searchService
- entityChangesService (enhanced)
- Script execution services

**Endpoint Coverage:** 100% of functionality

---

## Critical Service Dependencies (Ranked by Priority)

```
Phase 1:
1. becca (40+ endpoints) - MUST PORT FIRST
2. blobService (7+ endpoints)
3. sql/transactions (all write operations)
4. noteService (12+ endpoints)

Phase 2:
5. branchService (4+ endpoints)
6. optionService (3+ endpoints)
7. imageService (2+ endpoints)

Phase 3:
8. searchService (5+ endpoints) - MOST COMPLEX
9. treeService (2+ endpoints)
10. Advanced services (sync, scripting, etc.)
```

---

## Key Implementation Patterns

### Endpoint Pattern 1: Tree Navigation
```
GET /api/tree
GET /api/tree/load
PUT /api/branches/:branchId/expanded/:expanded
PUT /api/branches/:branchId/move-to/:parentBranchId
DELETE /api/branches/:branchId
```
**Pattern:** Cache-based lookups, simple SQL updates
**Dependencies:** becca, sql
**Difficulty:** Medium

### Endpoint Pattern 2: Note CRUD
```
GET /api/notes/:noteId
GET /api/notes/:noteId/blob
PUT /api/notes/:noteId/data
PUT /api/notes/:noteId/title
DELETE /api/notes/:noteId
```
**Pattern:** Full entity lifecycle management
**Dependencies:** becca, blobService, noteService, transactions
**Difficulty:** High

### Endpoint Pattern 3: Metadata Management
```
GET /api/notes/:noteId/attributes
PUT /api/notes/:noteId/attributes
PUT /api/notes/:noteId/attribute
DELETE /api/notes/:noteId/attributes/:attributeId
```
**Pattern:** Simple CRUD on related entities
**Dependencies:** becca, BAttribute
**Difficulty:** Low-Medium

### Endpoint Pattern 4: Content Retrieval
```
GET /api/notes/:noteId/blob
GET /api/attachments/:attachmentId/blob
GET /api/notes/:noteId/open
GET /api/images/:noteId/:filename
```
**Pattern:** Stream content from blob storage
**Dependencies:** blobService, security checks
**Difficulty:** Medium

### Endpoint Pattern 5: Search
```
GET /api/quick-search/:searchString
GET /api/search/:searchString
POST /api/search-related
```
**Pattern:** Full-text search with result formatting
**Dependencies:** searchService, full-text index
**Difficulty:** High

---

## Database Schema Overview

### Core Tables Required
```
notes
  - noteId (PK)
  - title
  - type (text, code, image, file, canvas, mermaid, relation-map, webview, doc)
  - mime
  - content (deprecated, use blobs)
  - dateCreated
  - dateModified
  - utcDateCreated
  - utcDateModified
  - isDeleted
  - isProtected
  - blobId (FK -> blobs)

branches
  - branchId (PK)
  - noteId (FK)
  - parentNoteId (FK)
  - notePosition (ordering)
  - prefix (optional branch label)
  - isExpanded
  - isDeleted

attributes
  - attributeId (PK)
  - noteId (FK)
  - type (label | relation)
  - name
  - value
  - position
  - isInheritable
  - isDeleted

attachments
  - attachmentId (PK)
  - noteId (FK)
  - role (file | image)
  - mime
  - title
  - blobId (FK -> blobs)
  - isDeleted

blobs
  - blobId (PK)
  - content (binary)
  - contentLength

revisions
  - revisionId (PK)
  - noteId (FK)
  - type
  - mime
  - title
  - dateCreated
  - utcDateCreated
  - blobId (FK -> blobs)
  - isDeleted

options
  - name (PK)
  - value

entity_changes
  - id (PK)
  - entityName
  - entityId
  - operation (CREATE | UPDATE | DELETE)
  - isSynced
  - utcDateCreated
```

---

## Common Gotchas & Solutions

### 1. Soft Deletes vs Hard Deletes
- **Pattern:** Records marked as deleted, not removed
- **Issue:** Must always filter `WHERE isDeleted = 0`
- **Solution:** Use view layers to hide deleted records

### 2. Note Cloning (Multiple Parents)
- **Pattern:** Branches connect notes, allowing multiple parents
- **Issue:** A note can have many branches (clones)
- **Solution:** Operations on note vs branch differ
  - Delete note: All branches deleted
  - Delete branch: Only that relationship deleted

### 3. Protected Sessions
- **Pattern:** Protected note content requires session validation
- **Issue:** Can't access protected note content without session
- **Solution:** Check `protectedSessionService.isProtectedSessionAvailable()`

### 4. Content Deduplication
- **Pattern:** Multiple notes can reference same blob
- **Issue:** Deleting note doesn't always delete blob
- **Solution:** Track blob references, only delete when unreferenced

### 5. Cascading Updates
- **Pattern:** Title changes trigger revisions and events
- **Issue:** Multiple side effects per operation
- **Solution:** Use transaction-based approach, track all changes

### 6. Entity Position Ordering
- **Pattern:** notePosition field used for ordering (10, 20, 30...)
- **Issue:** Reordering requires bulk updates
- **Solution:** Update positions in batches during moves

---

## Testing Checklist

### Phase 1 Tests
- [ ] becca initialization and caching
- [ ] Tree loading (empty, single note, complex tree)
- [ ] Note retrieval (metadata and blob)
- [ ] Branch expansion toggle
- [ ] Note title and type updates
- [ ] Basic attributes CRUD
- [ ] Search indexing

### Phase 2 Tests
- [ ] Note creation with positioning
- [ ] File upload/download
- [ ] Attachment CRUD
- [ ] Image handling (PNG, JPEG, GIF, WebP, SVG)
- [ ] Revision creation and restoration
- [ ] Option get/set with whitelist

### Phase 3 Tests
- [ ] Full-text search accuracy
- [ ] Search result ranking
- [ ] Query parsing edge cases
- [ ] Sync change detection
- [ ] Special note generation
- [ ] Bulk action execution

---

## File Structure Reference

### Routes
```
apps/server/src/routes/
├── api/
│   ├── tree.ts                 # Tree navigation
│   ├── notes.ts                # Note CRUD
│   ├── branches.ts             # Branch operations
│   ├── attributes.ts           # Attribute CRUD
│   ├── attachments.ts          # Attachment management
│   ├── search.ts               # Search operations
│   ├── revisions.ts            # Version history
│   ├── files.ts                # File operations
│   ├── image.ts                # Image handling
│   ├── options.ts              # Settings
│   └── sync.ts                 # Synchronization
└── route_api.ts                # Route registration

```

### Services
```
apps/server/src/services/
├── becca/                      # Entity cache
├── blob.js                     # Content storage
├── notes.ts                    # Note operations
├── branches.ts                 # Branch operations
├── attributes.ts              # Attribute operations
├── search/                     # Full-text search
├── sql.ts                      # Database access
├── options.ts                  # Settings storage
├── image.ts                    # Image processing
└── tree.ts                     # Tree validation
```

---

## Success Metrics

### Phase 1 Complete
- User can view tree structure
- User can read note metadata and content
- User can create/update basic notes
- User can update note titles and types
- User can manage attributes (tags/labels)

### Phase 2 Complete
- All note CRUD operations working
- File upload/download functional
- Attachments fully managed
- Revision history available
- Image handling complete

### Phase 3 Complete
- Full-text search working
- All advanced features functional
- Sync between instances working
- All original features implemented

---

## Additional Resources

### Original TypeScript Source
- Look in `apps/server/src/` for original implementations
- Review service files to understand business logic
- Check tests for expected behavior

### Database Schema
- See `apps/server/src/assets/db/schema.sql` for full schema
- Review migration files for schema evolution
- Check comments for special handling

### Type Definitions
- Review `packages/commons/src/` for shared types
- Check API response interfaces in commons
- Use TypeScript for type safety

---

## Next Steps

1. **Start with this guide** - Understand the overall structure
2. **Read API_ROUTES_ANALYSIS.md** - Deep dive into specific endpoints
3. **Reference API_ROUTES_QUICK_REFERENCE.md** - Quick lookup during coding
4. **Study API_SERVICE_DEPENDENCIES.md** - Plan implementation sequence
5. **Review original TypeScript source** - Understand actual implementation details
6. **Run integration tests** - Ensure correctness

---

## Questions to Consider

Before starting migration:

1. **Target Technology?**
   - Rust? Go? Python? Node.js?
   - Decision affects API compatibility

2. **Database?**
   - SQLite (single-file)? PostgreSQL (multi-client)? Other?
   - Choice affects blob storage strategy

3. **Scope?**
   - Full feature parity or MVP?
   - Affects which endpoints to prioritize

4. **Timeline?**
   - Phased rollout or big bang?
   - Determines testing strategy

5. **Sync Strategy?**
   - Multi-client support needed?
   - Affects entity change tracking requirements

---

Generated: 2025-01-16
Analysis of: Trilium Notes API Routes (apps/server/src/routes/api/)
Total Endpoints: 53+ across 20+ route files
