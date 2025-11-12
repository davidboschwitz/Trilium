# API Service Dependencies for Migration

## Overview
This document maps each API endpoint to its service dependencies, helping identify critical services that need to be ported/implemented first.

---

## Critical Service: Becca (Backend Cache)

**Location:** `apps/server/src/becca/becca.ts`

### Why Critical
- **40+ API endpoints** depend directly on Becca
- In-memory cache of all entities (notes, branches, attributes, attachments, revisions)
- Must be loaded before server accepts requests
- High-performance lookups essential for UI responsiveness

### Core Methods Used by API Routes

```typescript
// Note lookups
becca.notes[noteId]              // Direct access
becca.getNoteOrThrow(noteId)     // With error handling
becca.getNote(noteId)            // Optional access

// Branch lookups
becca.branches[branchId]         // Direct access
becca.getBranch(branchId)        // Optional
becca.getBranchOrThrow(branchId) // With error

// Attribute lookups
becca.attributes[attributeId]
becca.getAttribute(attributeId)
becca.getAttributeOrThrow(attributeId)

// Attachment lookups
becca.attachments[attachmentId]
becca.getAttachment(attachmentId)
becca.getAttachmentOrThrow(attachmentId)

// Revision lookups
becca.getRevision(revisionId)
becca.getRevisionOrThrow(revisionId)
becca.getRevisionsFromQuery()

// Collections
becca.getNotes(noteIds, true)    // Bulk fetch
```

### Endpoints Depending on Becca

#### Direct Cache Access (23 endpoints)
- GET /api/tree
- POST /api/tree/load
- GET /api/notes/:noteId
- GET /api/notes/:noteId/metadata
- GET /api/notes/:noteId/attributes
- GET /api/notes/:noteId/revisions
- GET /api/notes/:noteId/attachments
- DELETE /api/notes/:noteId/attributes/:attributeId
- GET /api/attachments/:attachmentId
- GET /api/attachments/:attachmentId/all
- DELETE /api/attachments/:attachmentId
- PUT /api/attachments/:attachmentId/rename
- GET /api/revisions/:revisionId
- DELETE /api/branches/:branchId
- PUT /api/branches/:branchId/expanded/:expanded
- PUT /api/branches/:branchId/expanded-subtree/:expanded
- And others...

### Cache Initialization
```typescript
// Must happen before API requests accepted
await becca.initialize(); // Loads all entities from database

// Cache structure
{
  notes: Map<string, BNote>,
  branches: Map<string, BBranch>,
  attributes: Map<string, BAttribute>,
  attachments: Map<string, BAttachment>,
  revisions: Map<string, BRevision>,
  options: Map<string, string>
}
```

### Cache Invalidation Patterns
- On note update: `note.save()`
- On branch move: Direct cache update + database sync
- On attribute change: Automatic via BAttribute.save()
- On deletion: Entity marked as deleted in both cache and DB

---

## Critical Service: blobService

**Location:** `apps/server/src/services/blob.js`

### Why Critical
- **7+ API endpoints** directly use blobService for content retrieval
- Handles both text and binary content
- Manages blob storage layer (separate from entities)
- Content deduplication support

### Core Methods

```typescript
blobService.getBlobPojo(
  entityType: "notes" | "attachments" | "revisions",
  entityId: string,
  options?: { preview?: boolean }
): { content: string | Buffer, contentLength: number }

blobService.setContent(entityId, content)
blobService.deleteBlob(blobId)
```

### Endpoints Using blobService

1. **Note Content Retrieval (3 endpoints)**
   - GET /api/notes/:noteId/blob - Main note content
   - GET /api/attachments/:attachmentId/blob - Attachment content
   - GET /api/revisions/:revisionId/blob - Revision snapshots

2. **File Content Streaming (4 endpoints)**
   - GET /api/notes/:noteId/open
   - GET /api/notes/:noteId/download
   - GET /api/attachments/:attachmentId/open
   - GET /api/attachments/:attachmentId/download

3. **Image Content (2 endpoints)**
   - GET /api/images/:noteId/:filename
   - GET /api/attachments/:attachmentId/image/:filename

### Implementation Dependencies
- Database schema with blobs table
- File system or cloud storage backend
- Optional compression/decompression
- Optional encryption for protected content

---

## Critical Service: noteService

**Location:** `apps/server/src/services/notes.ts`

### Why Critical
- **12+ API endpoints** depend on noteService for write operations
- Coordinates between note updates, blob storage, revisions
- Manages cascading updates (title changes, type changes)
- Handles protected content

### Core Methods

```typescript
noteService.createNewNoteWithTarget(
  target: "into" | "after" | "before",
  targetBranchId: string,
  params: NoteCreateParams
): { note: BNote, branch: BBranch }

noteService.updateNoteData(noteId, content, attachments)

noteService.saveRevisionIfNeeded(note)

noteService.protectNoteRecursively(note, protect, includingSubtree)

noteService.triggerNoteTitleChanged(note)

noteService.undeleteNote(noteId, taskContext)

noteService.duplicateSubtree(noteId, parentNoteId)

noteService.asyncPostProcessContent(note, content)
  // Image optimization, indexing, etc.
```

### Endpoints Using noteService

1. **Note CRUD (7 endpoints)**
   - GET /api/notes/:noteId
   - POST /api/notes/:parentNoteId/children
   - PUT /api/notes/:noteId/data
   - PUT /api/notes/:noteId/title
   - DELETE /api/notes/:noteId
   - PUT /api/notes/:noteId/undelete
   - POST /api/notes/:noteId/duplicate/:parentNoteId

2. **File Operations (2 endpoints)**
   - PUT /api/notes/:noteId/file
   - PUT /api/images/:noteId

3. **Special Operations (3 endpoints)**
   - PUT /api/notes/:noteId/protect/:isProtected
   - POST /api/revisions/:revisionId/restore
   - POST /api/notes/:noteId/revision (forceSaveRevision)

### Implementation Notes
- Must wrap operations in transactions
- Triggers entity change events for sync
- Revision creation is automatic for content changes
- Protected session handling required for protected notes

---

## Critical Service: branchService

**Location:** `apps/server/src/services/branches.ts`

### Why Critical
- **4+ API endpoints** for tree navigation depend on this
- Manages note positioning and hierarchy
- Handles branch-to-branch relationships (for note clones)
- Updates notePosition field for ordering

### Core Methods

```typescript
branchService.moveBranchToBranch(branchToMove, targetParent, branchId)

branchService.deleteBranch(branch)

branchService.moveBranchAfterNote(branchToMove, afterBranch)

branchService.moveBranchBeforeNote(branchToMove, beforeBranch)
```

### Endpoints Using branchService

1. **Branch Navigation (3 endpoints)**
   - PUT /api/branches/:branchId/move-to/:parentBranchId
   - PUT /api/branches/:branchId/move-before/:beforeBranchId
   - PUT /api/branches/:branchId/move-after/:afterBranchId

2. **Branch Deletion (1 endpoint)**
   - DELETE /api/branches/:branchId

### Key Concepts
- Branches allow notes to have multiple parents (clones)
- notePosition determines order in parent's children
- Tree consistency must be validated before moves
- SQL updates for positioning are batch operations

---

## Critical Service: searchService

**Location:** `apps/server/src/services/search/`

### Why Critical
- **5+ API endpoints** for search depend on this
- Full-text search engine for note finding
- Supports fuzzy matching and snippet extraction
- Complex search expression parsing

### Core Methods

```typescript
searchService.findResultsWithQuery(
  searchString: string,
  searchContext: SearchContext
): SearchResult[]

searchService.searchFromNote(note: BNote): SearchNoteResult

searchService.extractContentSnippet(noteId, tokens)
searchService.extractAttributeSnippet(noteId, tokens)

searchService.highlightSearchResults(results, tokens, ignoreInternal)

searchService.searchNotes(query, options)
```

### Endpoints Using searchService

1. **Quick Search (1 endpoint)**
   - GET /api/quick-search/:searchString
     - Limited to 200 results
     - Includes snippets and highlighting

2. **Full Search (3 endpoints)**
   - GET /api/search/:searchString
   - GET /api/search-note/:noteId
   - POST /api/search-and-execute-note/:noteId

3. **Related Notes (1 endpoint)**
   - POST /api/search-related

4. **Templates (1 endpoint)**
   - GET /api/search-templates

### Implementation Notes
- Must support full-text index (FTS)
- Search context controls behavior (fuzzy, archived, hoisted, etc.)
- Highlighting requires token tracking
- Can be complex to port - consider phased approach

---

## Supporting Service: sql / entityChangesService

**Location:** 
- `apps/server/src/services/sql.ts`
- `apps/server/src/services/entity_changes.ts`

### Why Important
- **All write endpoints** use sql for transactions
- Entity change tracking for sync support
- Transaction rollback on errors
- Atomic multi-operation updates

### Core Methods

```typescript
sql.transactional(callback)
sql.execute(sql, params)
sql.getRows(sql, params)
sql.getValue(sql, params)

entityChangesService.putEntityChange(entityName, entityId, operation)
entityChangesService.putNoteReorderingEntityChange(noteId)
entityChangesService.getMaxEntityChangeId()
```

### Required For
- All POST/PUT/DELETE endpoints
- Consistency checks
- Sync coordination

---

## Supporting Service: optionService

**Location:** `apps/server/src/services/options.ts`

### Methods Used

```typescript
optionService.getOptionMap(): Record<OptionNames, string>
optionService.setOption(name, value)
optionService.getOption(name): string
```

### Endpoints Using optionService

- GET /api/options
- PUT /api/options/:name/:value
- PUT /api/options

### Implementation Notes
- Simple key-value storage
- Whitelist validation required
- Language change triggers i18n update

---

## Supporting Service: imageService

**Location:** `apps/server/src/services/image.ts`

### Methods Used

```typescript
imageService.updateImage(noteId, buffer, originalname)
imageService.saveImageToAttachment(noteId, buffer, originalname)
```

### Endpoints Using imageService

- PUT /api/images/:noteId
- POST /api/notes/:noteId/attachments/upload (for image files)

### Implementation Notes
- Image optimization/compression
- MIME type validation
- Attachment creation for images

---

## Supporting Service: treeService

**Location:** `apps/server/src/services/tree.ts`

### Methods Used

```typescript
treeService.validateParentChild(parentId, childId, branchId)
treeService.sortNotes(noteId, sortBy, reverse, foldersFirst)
treeService.sortNotesIfNeeded(noteId)
```

### Endpoints Using treeService

- PUT /api/notes/:noteId/sort-children
- PUT /api/branches/:branchId/move-*

### Implementation Notes
- Prevents circular references
- Handles note sorting logic
- Validates parent-child relationships

---

## Service Implementation Priority for Migration

### MUST PORT FIRST (Phase 1)
1. **becca** - Core cache system
   - Dependency: Database schema + entity definitions
   - Complexity: High (in-memory structure)
   - Impact: ~40% of endpoints

2. **blobService** - Content storage
   - Dependency: Blob storage backend
   - Complexity: Medium
   - Impact: ~20% of endpoints

3. **sql & entityChangesService** - Database layer
   - Dependency: Database connection
   - Complexity: Medium
   - Impact: ~80% of write endpoints

4. **noteService** - Note operations
   - Dependency: becca, blobService, sql
   - Complexity: High (cascading updates)
   - Impact: ~25% of endpoints

### PORT IN PHASE 2
5. **branchService** - Tree navigation
6. **optionService** - Settings storage
7. **imageService** - Image handling

### PORT IN PHASE 3
8. **searchService** - Full-text search (most complex)
9. **treeService** - Tree validation
10. Supporting services (attributes, revisions, etc.)

---

## Database Dependencies

### Required Tables
```sql
-- Core entities
notes
branches
attributes
attachments
blobs
revisions

-- Support tables
options
entity_changes
deleted_items (soft delete tracking)
```

### Key Indexes
- notes(noteId)
- branches(noteId, parentNoteId)
- attributes(noteId, type, name)
- attachments(attachmentId, noteId)
- blobs(blobId)
- entity_changes(id, isSynced)

---

## Testing Strategy

### Phase 1 Testing
- Unit tests for becca initialization
- Unit tests for blobService read operations
- Integration tests for note CRUD
- Tree navigation tests

### Phase 2 Testing
- File upload/download tests
- Attachment management tests
- Revision restoration tests

### Phase 3 Testing
- Full-text search tests
- Complex query tests
- Performance benchmarks

---

## Migration Checklist

```
[ ] Implement becca cache system
    [ ] Entity classes (BNote, BBranch, etc.)
    [ ] Cache initialization from database
    [ ] Cache invalidation patterns
[ ] Implement blobService
    [ ] Storage backend selection
    [ ] Content retrieval
    [ ] Content storage
[ ] Implement SQL/transaction layer
    [ ] Database connection
    [ ] Transaction management
    [ ] Entity change tracking
[ ] Implement noteService
    [ ] Note creation with positioning
    [ ] Note update with cascading
    [ ] Revision creation
[ ] Implement basic routes
    [ ] GET /api/tree
    [ ] GET/PUT /api/notes/:noteId
    [ ] Basic CRUD operations
[ ] Implement branchService
    [ ] Branch movement
    [ ] Tree reordering
[ ] Implement searchService
    [ ] Full-text search
    [ ] Query parsing
    [ ] Snippet extraction
```

