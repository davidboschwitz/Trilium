# Critical Node.js API Routes Analysis - Trilium Notes

## Executive Summary

This document identifies and prioritizes the critical API endpoints that need to be migrated for Trilium Notes. The analysis focuses on endpoints essential for basic note viewing/editing, navigation, and content retrieval.

---

## HIGH PRIORITY ENDPOINTS

### 1. TREE & NAVIGATION ENDPOINTS
**File:** `/home/user/Trilium/apps/server/src/routes/api/tree.ts`

These endpoints are essential for loading the note tree structure, which is fundamental to the application.

#### GET /api/tree
- **Function:** `getTree(req: Request)`
- **Returns:** `{ branches: BranchRow[], notes: NoteRow[], attributes: AttributeRow[] }`
- **Query Params:**
  - `subTreeNoteId` (optional) - Limits tree data to this note and descendants
- **Description:** Retrieves the hierarchical tree structure of notes with their branches and attributes. Loads the tree starting from the root or a specified subtree, including only expanded branches.
- **Key Services:**
  - `becca` - Backend cache for fetching notes/branches
  - Recursive collection of note hierarchy
- **Data Structure:** 
  - Returns expanded note tree with parent-child relationships
  - Critical for UI tree view initialization
- **Performance Considerations:** Recursively collects expanded branches, could be large for complex hierarchies

#### POST /api/tree/load
- **Function:** `load(req: Request)`
- **Returns:** `{ branches: BranchRow[], notes: NoteRow[], attributes: AttributeRow[] }`
- **Request Body:** `{ noteIds: string[] }`
- **Description:** Loads specific notes and their relationships (parents, children, attributes). Used for bulk loading of note data.
- **Key Services:**
  - `becca` - Cache lookups
  - Collects template/inherit relations
- **Data Structure:**
  - Loads target notes and their immediate relationships
  - Follows template and inherit attribute links
  - Essential for loading note context in editor

---

### 2. NOTE CRUD OPERATIONS
**File:** `/home/user/Trilium/apps/server/src/routes/api/notes.ts`

Core operations for reading, creating, and updating notes.

#### GET /api/notes/:noteId
- **Function:** `getNote(req: Request)`
- **Returns:** Note entity with metadata (title, type, mime, isProtected, dateCreated, dateModified, etc.)
- **Description:** Retrieves complete note metadata without content
- **Key Services:**
  - `becca.getNoteOrThrow()` - Throws if note not found
- **Critical For:** Loading note properties in editor
- **Dependencies:** Note must exist in Becca cache

#### GET /api/notes/:noteId/blob
- **Function:** `getNoteBlob(req: Request)`
- **Returns:** `{ content: string | Buffer, contentLength: number }`
- **Description:** Retrieves note content (blob). Handles text and binary content.
- **Key Services:**
  - `blobService.getBlobPojo()` - Fetches blob content from blob storage
- **Critical For:** Loading note content into editor
- **Special Handling:** May require protected session for protected notes

#### GET /api/notes/:noteId/metadata
- **Function:** `getNoteMetadata(req: Request)`
- **Returns:** `{ dateCreated, utcDateCreated, dateModified, utcDateModified }`
- **Description:** Lightweight metadata endpoint returning only timestamps
- **Key Services:** `becca.getNoteOrThrow()`
- **Use Case:** Checking if note was recently modified

#### PUT /api/notes/:noteId/data
- **Function:** `updateNoteData(req: Request)`
- **Request Body:** `{ content: string | Buffer, attachments?: Attachment[] }`
- **Returns:** Updated note data
- **Description:** Updates note content and attachments in a single operation
- **Key Services:**
  - `noteService.updateNoteData()` - Coordinates update across content and attachments
  - `blobService` - Stores content
  - Triggers revision snapshots if needed
- **Critical For:** Saving note edits
- **Side Effects:** Creates revisions, updates modification timestamps

#### POST /api/notes/:parentNoteId/children
- **Function:** `createNote(req: Request)`
- **Query Params:**
  - `target` (required) - "into", "after", or "before"
  - `targetBranchId` (optional) - Branch ID for positioning
- **Request Body:** Note creation parameters (title, type, mime, etc.)
- **Returns:** `{ note: Note, branch: Branch }`
- **Description:** Creates new note as child of parent with optional positioning
- **Key Services:**
  - `noteService.createNewNoteWithTarget()` - Handles note creation and tree insertion
- **Critical For:** Creating new notes in the tree
- **Dependencies:** Parent note must exist

#### PUT /api/notes/:noteId/title
- **Function:** `changeTitle(req: Request)`
- **Request Body:** `{ title: string }`
- **Returns:** Updated note
- **Description:** Updates note title with revision tracking
- **Key Services:**
  - `noteService.saveRevisionIfNeeded()` - Creates revision before title change
  - Triggers note title change event
- **Side Effects:** May create revision snapshot

#### PUT /api/notes/:noteId/type
- **Function:** `setNoteTypeMime(req: Request)`
- **Request Body:** `{ type: string, mime: string }`
- **Returns:** (none)
- **Description:** Updates note type and MIME type
- **Key Services:** Direct Becca note modification
- **Critical For:** Changing between text/code/image/etc note types

#### DELETE /api/notes/:noteId
- **Function:** `deleteNote(req: Request)`
- **Query Params:**
  - `taskId` (required) - Task identifier for grouping operations
  - `eraseNotes` (optional) - Whether to immediately erase instead of soft delete
  - `last` (required) - Whether this is the last request in the task
- **Returns:** (none)
- **Description:** Soft deletes note (marks as deleted) with optional immediate erasure
- **Key Services:**
  - `note.deleteNote()` - Marks note and children as deleted
  - `eraseService.eraseNotesWithDeleteId()` - Optional immediate erasure
  - `TaskContext` - Coordinates multi-request delete operations
- **Critical For:** Removing notes from the tree
- **Async Behavior:** Task-based system allows grouping multiple deletes

#### PUT /api/notes/:noteId/undelete
- **Function:** `undeleteNote(req: Request)`
- **Returns:** (none)
- **Description:** Restores a soft-deleted note
- **Key Services:** `noteService.undeleteNote()`, `TaskContext`

---

### 3. BRANCH & TREE NAVIGATION
**File:** `/home/user/Trilium/apps/server/src/routes/api/branches.ts`

Manages note positioning and hierarchy relationships. Branches represent the connection between parent and child notes (allowing multiple parents/clones).

#### PUT /api/branches/:branchId/expanded/:expanded
- **Function:** `setExpanded(req: Request)`
- **Params:** 
  - `branchId` - ID of branch to toggle
  - `expanded` - 0 or 1
- **Returns:** (none)
- **Description:** Toggles whether a branch node is expanded in the tree
- **Key Services:**
  - SQL UPDATE statement for persistence
  - `becca` cache update
- **Critical For:** Tree UI state management
- **Special Note:** Root branch "none_root" is always expanded

#### PUT /api/branches/:branchId/move-to/:parentBranchId
- **Function:** `moveBranchToParent(req: Request)`
- **Returns:** Updated branch data
- **Description:** Moves a branch (and its note) to a new parent
- **Key Services:**
  - `branchService.moveBranchToBranch()` - Handles parent change
  - Updates notePosition field for ordering
- **Critical For:** Drag-and-drop tree operations

#### DELETE /api/branches/:branchId
- **Function:** `deleteBranch(req: Request)`
- **Query Params:**
  - `taskId` (required)
  - `eraseNotes` (optional)
  - `last` (required)
- **Returns:** `{ noteDeleted: boolean }`
- **Description:** Deletes a branch (may delete note if last branch)
- **Key Services:** `branchService.deleteBranch()` or `eraseService`
- **Critical For:** Removing branches (clones) from tree
- **Behavior:** If note has multiple branches, only branch is deleted. Last branch deletion also deletes the note.

---

### 4. NOTE ATTRIBUTES & METADATA
**File:** `/home/user/Trilium/apps/server/src/routes/api/attributes.ts`

Attributes are key-value metadata attached to notes (labels and relations).

#### GET /api/notes/:noteId/attributes
- **Function:** `getEffectiveNoteAttributes(req: Request)`
- **Returns:** Array of `{ attributeId, noteId, type, name, value, position, isInheritable }`
- **Description:** Retrieves all attributes for a note, including inherited ones
- **Key Services:** `note.getAttributes()` - Includes inherited attributes
- **Critical For:** Loading note metadata/properties in UI

#### PUT /api/notes/:noteId/attributes
- **Function:** `updateNoteAttributes(req: Request)`
- **Request Body:** Array of attributes with type, name, value, isInheritable
- **Returns:** (none)
- **Description:** Bulk update all attributes for a note (replace operation)
- **Key Services:**
  - `BAttribute` - Entity model for attributes
  - SQL for persistence
- **Behavior:** Compares incoming with existing, creates/updates/deletes as needed
- **Critical For:** Updating note metadata/tags

#### PUT /api/notes/:noteId/attribute
- **Function:** `updateNoteAttribute(req: Request)`
- **Request Body:** Single attribute update
- **Returns:** `{ attributeId: string }`
- **Description:** Update or create a single attribute
- **Behavior:** Creates clone if type/name/value changes

#### DELETE /api/notes/:noteId/attributes/:attributeId
- **Function:** `deleteNoteAttribute(req: Request)`
- **Returns:** (none)
- **Description:** Deletes a specific attribute

---

### 5. BASIC SEARCH
**File:** `/home/user/Trilium/apps/server/src/routes/api/search.ts`

Essential for finding notes.

#### GET /api/quick-search/:searchString
- **Function:** `quickSearch(req: Request)`
- **Returns:** `{ searchResultNoteIds: string[], searchResults: SearchResultWithHighlights[], error?: string }`
- **Description:** Fast search with fuzzy matching and snippets (limited to 200 results)
- **Key Services:**
  - `SearchContext` - Search configuration
  - `searchService.findResultsWithQuery()` - Main search engine
  - `searchService.extractContentSnippet()` - Preview extraction
  - Highlights matching text in results
- **Critical For:** Quick note lookup in UI
- **Features:** 
  - Includes content and attribute snippets
  - Syntax highlighting in results
  - Shows note path and icon

#### GET /api/search/:searchString
- **Function:** `search(req: Request)`
- **Returns:** Array of matching note IDs
- **Description:** Full search without snippets/highlights
- **Key Services:** `searchService.findResultsWithQuery()`
- **Use Case:** When full result set is needed

#### GET /api/search-note/:noteId
- **Function:** `searchFromNote(req: Request)`
- **Returns:** `{ searchResultNoteIds: string[], searchResults?: SearchResult[] }`
- **Description:** Executes search stored in a search note
- **Requires:** Note must be of type "search"
- **Key Services:** `searchService.searchFromNote()`

---

## MEDIUM PRIORITY ENDPOINTS

### 6. CONTENT RETRIEVAL (Files & Downloads)
**File:** `/home/user/Trilium/apps/server/src/routes/api/files.ts`

Handles file/binary content retrieval and upload.

#### GET /api/notes/:noteId/open
- **Function:** `openFile(req: Request, res: Response)`
- **Returns:** File content in response body
- **Description:** Streams file note content without Content-Disposition header
- **Key Services:** `filesRoute.openFile` (uses internal helper)
- **Use Case:** Opening files in browser/editor
- **Content-Type:** Set according to note's MIME type

#### GET /api/notes/:noteId/download
- **Function:** `downloadFile(req: Request, res: Response)`
- **Returns:** File content with Content-Disposition header
- **Description:** Downloads file note content with attachment header
- **Key Services:** Internal file streaming handler
- **Use Case:** Downloading files to disk

#### PUT /api/notes/:noteId/file
- **Function:** `updateFile(req: Request)`
- **Request Body:** Multipart form with file
- **Returns:** `{ uploaded: boolean, message?: string }`
- **Description:** Uploads/replaces file note content
- **Key Services:**
  - `note.saveRevision()` - Backup old version
  - `note.setContent()` - Store new content
  - `noteService.asyncPostProcessContent()` - Image optimization, etc.
- **Side Effects:** Creates revision, sets original filename label

---

### 7. ATTACHMENT MANAGEMENT
**File:** `/home/user/Trilium/apps/server/src/routes/api/attachments.ts`

Manages file attachments within notes.

#### GET /api/notes/:noteId/attachments
- **Function:** `getAttachments(req: Request)`
- **Returns:** Array of attachment objects with metadata
- **Description:** Lists all attachments for a note
- **Key Services:** `note.getAttachments()`
- **Includes:** Content length for progress indication

#### GET /api/attachments/:attachmentId/blob
- **Function:** `getAttachmentBlob(req: Request)`
- **Query Params:** `preview` (boolean) - Return preview instead of full content
- **Returns:** Blob object with content
- **Description:** Fetches attachment content
- **Key Services:** `blobService.getBlobPojo()`

#### POST /api/notes/:noteId/attachments
- **Function:** `saveAttachment(req: Request)`
- **Request Body:** `{ attachmentId?, role, mime, title, content }`
- **Query Params:** `matchBy` - "attachmentId" or "title"
- **Returns:** (none)
- **Description:** Create or update attachment
- **Key Services:** `note.saveAttachment()`
- **Matching:** Can create/update based on title or ID

#### POST /api/notes/:noteId/attachments/upload
- **Function:** `uploadAttachment(req: Request)`
- **Request Body:** Multipart form with file
- **Returns:** `{ uploaded: boolean, url: string }`
- **Description:** Upload file as attachment
- **Key Services:**
  - `imageService.saveImageToAttachment()` - For image files
  - `note.saveAttachment()` - For other files
- **Behavior:** Images get special handling, returns display URL

---

### 8. REVISIONS & VERSION HISTORY
**File:** `/home/user/Trilium/apps/server/src/routes/api/revisions.ts`

Manages note version history.

#### GET /api/notes/:noteId/revisions
- **Function:** `getRevisions(req: Request)`
- **Returns:** `RevisionItem[]` with metadata and content length
- **Description:** Lists all revisions for a note, ordered by creation date
- **Key Services:** `becca.getRevisionsFromQuery()`

#### GET /api/revisions/:revisionId/blob
- **Function:** `getRevisionBlob(req: Request)`
- **Query Params:** `preview` (boolean)
- **Returns:** Revision blob content
- **Description:** Fetches revision content
- **Key Services:** `blobService.getBlobPojo()`

#### GET /api/revisions/:revisionId
- **Function:** `getRevision(req: Request)`
- **Returns:** Complete revision data with content
- **Description:** Loads revision with full content
- **Processing:** 
  - For file revisions: Truncates to 10KB for previews
  - For images: Converts to base64
- **Key Services:** Direct Becca entity

#### POST /api/revisions/:revisionId/restore
- **Function:** `restoreRevision(req: Request)`
- **Returns:** (none)
- **Description:** Restores note to a previous revision state
- **Key Services:**
  - `note.saveRevision()` - Save current state first
  - Updates note title, type, mime, content
  - Copies over attachment state
  - Rewrites attachment references in content
- **Side Effects:** Creates new revision of current state before restoring

---

### 9. OPTIONS & SETTINGS
**File:** `/home/user/Trilium/apps/server/src/routes/api/options.ts`

User configuration and preferences.

#### GET /api/options
- **Function:** `getOptions(req: Request)`
- **Returns:** `{ [optionName: string]: string }`
- **Description:** Retrieves all user settings/options
- **Special Fields:**
  - `isPasswordSet` - Derived from passwordVerificationHash
  - `databaseReadonly` - From config
- **Filtering:** Only whitelisted options returned
- **Key Services:** `optionService.getOptionMap()`

#### PUT /api/options/:name/:value
- **Function:** `updateOption(req: Request)`
- **Params:**
  - `name` - Option name (must be whitelisted)
  - `value` - Option value as string
- **Returns:** (none)
- **Description:** Updates single option
- **Whitelist:** ~100 options allowed (theme, zoom, fonts, keyboard shortcuts, etc.)
- **Special Behavior:** "locale" changes trigger i18n language change

#### PUT /api/options
- **Function:** `updateOptions(req: Request)`
- **Request Body:** `{ [optionName: string]: string }`
- **Returns:** (none)
- **Description:** Batch update multiple options
- **Validation:** Each option must be whitelisted

---

### 10. IMAGE RETRIEVAL
**File:** `/home/user/Trilium/apps/server/src/routes/api/image.ts`

Image display and management.

#### GET /api/images/:noteId/:filename
- **Function:** `returnImageFromNote(req: Request, res: Response)`
- **Returns:** Image content (PNG, JPEG, GIF, WebP, SVG)
- **Special Handling:**
  - Canvas notes: Returns canvas-export.svg attachment
  - Mermaid notes: Returns mermaid-export.svg attachment
  - MindMap notes: Returns mindmap-export.svg attachment
  - Missing images: Returns image-deleted.png placeholder
- **Cache-Control:** No caching (must-revalidate)

#### GET /api/attachments/:attachmentId/image/:filename
- **Function:** `returnAttachedImage(req: Request, res: Response)`
- **Returns:** Image attachment content
- **Validation:** Must have role="image"

#### PUT /api/images/:noteId
- **Function:** `updateImage(req: Request)`
- **Request Body:** Multipart form with image file
- **Returns:** `{ uploaded: boolean, message?: string }`
- **Validation:**
  - Allowed types: PNG, JPEG, GIF, WebP, SVG+XML
  - File must be binary (not string)
- **Key Services:** `imageService.updateImage()`

---

## LOWER PRIORITY ENDPOINTS

### 11. SYNC (Multi-Device Synchronization)
**File:** `/home/user/Trilium/apps/server/src/routes/api/sync.ts`

Entity change tracking and synchronization (primarily for multi-client scenarios).

#### GET /api/sync/check
#### GET /api/sync/changed
#### PUT /api/sync/update
#### POST /api/sync/finished
- **Description:** Core sync protocol endpoints for pulling and pushing changes
- **Complexity:** High - requires entity change tracking, version hashes, conflict resolution
- **Priority:** Medium for multi-client, Low for single-client scenario

#### POST /api/sync/test
#### POST /api/sync/now
- **Description:** Manually trigger sync operations
- **Async:** Returns before sync completes

---

### 12. ADVANCED SEARCH
**File:** `/home/user/Trilium/apps/server/src/routes/api/search.ts`

#### POST /api/search-and-execute-note/:noteId
- **Function:** `searchAndExecute(req: Request)`
- **Description:** Executes search and applies bulk actions from search note
- **Behavior:** Complex - executes predefined actions on search results

#### POST /api/search-related
- **Function:** `getRelatedNotes(req: Request)`
- **Request Body:** Attribute object
- **Returns:** Related notes matching attribute name/value
- **Use Case:** "Related notes" feature in sidebar

#### GET /api/search-templates
- **Function:** `searchTemplates()`
- **Returns:** Array of template note IDs
- **Description:** Finds all notes marked with #template label

---

### 13. SPECIAL FEATURES

#### GET /api/notes/:noteId/attributes (Formatted)
- Attribute search and formatting
- Used for relation dropdowns and attribute suggestions

#### GET /api/attribute-names
#### GET /api/attribute-values/:attributeName
- Autocomplete support for attribute editing

#### Additional Lower-Priority Endpoints:
- Cloning operations (note duplication with references)
- Bulk actions (batch operations on search results)
- Special notes (calendar days, months, search notes)
- Database operations (backup, vacuum, consistency checks)
- Scripting endpoints (execute backend scripts)
- LLM integration (Claude, OpenAI, Ollama)
- Metrics and statistics
- Login and authentication
- Web clipper integration

---

## ENDPOINT DEPENDENCIES SUMMARY

### Critical Service Dependencies:

```
becca (Backend Cache)
├── becca.getNoteOrThrow()
├── becca.getNoteBlob()
├── becca.getBranch()
├── becca.getAttribute()
├── becca.getAttachment()
├── becca.getRevision()
└── Cache invalidation on updates

noteService (Note Operations)
├── noteService.createNewNoteWithTarget()
├── noteService.updateNoteData()
├── noteService.saveRevisionIfNeeded()
├── noteService.undeleteNote()
└── noteService.protectNoteRecursively()

branchService (Tree Navigation)
├── branchService.moveBranchToBranch()
├── branchService.deleteBranch()
└── Branch position management

blobService (Content Storage)
├── blobService.getBlobPojo()
├── Content persistence
└── Blob deduplication

searchService (Full-Text Search)
├── searchService.findResultsWithQuery()
├── searchService.extractContentSnippet()
├── searchService.searchFromNote()
└── Highlighting

treeService (Tree Consistency)
├── treeService.validateParentChild()
├── treeService.sortNotes()
└── Tree integrity checks

sql & entityChangesService
├── Database persistence
├── Transaction management
├── Entity change tracking
└── Sync coordination
```

---

## MIGRATION STRATEGY RECOMMENDATION

### Phase 1: Foundation (High Priority)
1. **Tree loading** - GET /api/tree, POST /api/tree/load
2. **Note CRUD** - GET/PUT/DELETE /api/notes/:noteId
3. **Note content** - GET/PUT /api/notes/:noteId/{blob,data}
4. **Title & type** - PUT /api/notes/:noteId/{title,type}
5. **Branches** - PUT/DELETE /api/branches/:branchId/*
6. **Attributes** - GET/PUT /api/notes/:noteId/attributes*
7. **Quick search** - GET /api/quick-search

### Phase 2: Full Editing (Medium Priority)
1. Note creation - POST /api/notes/:parentNoteId/children
2. Image handling - GET/PUT /api/images
3. File operations - GET/PUT /api/notes/:noteId/file
4. Attachments - GET/POST/PUT/DELETE /api/attachments
5. Revisions - GET /api/notes/:noteId/revisions, POST /api/revisions/:revisionId/restore
6. Options - GET/PUT /api/options

### Phase 3: Advanced Features (Lower Priority)
1. Advanced search features
2. Sync operations
3. Special notes (calendar, etc.)
4. Bulk actions
5. Scripting
6. LLM integration

---

## Implementation Notes

### Becca Cache Dependency
Most endpoints depend on Becca (backend entity cache) for fast lookups. This cache must be:
- Loaded on server startup
- Kept in sync with database changes
- Invalidated appropriately on updates
- Serializable for multi-process scenarios

### Transaction Management
Most write operations must be wrapped in transactions to ensure:
- Atomic updates to notes and branches
- Cascade updates (e.g., deleting note also deletes branches)
- Entity change tracking for sync
- Revision creation

### Protected Sessions
Some operations (viewing/editing protected notes) require protected session validation:
- Protected note viewing
- Revision restoration
- Attribute management on protected notes

### Entity Change Tracking
For sync and multi-client scenarios, every change must be tracked:
- Entity type and ID
- Change timestamp (utcDateModified)
- Content hash for conflict detection
- Sync status
