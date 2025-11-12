# API Routes Quick Reference Table

## HIGH PRIORITY - Essential for Basic Functionality

| Endpoint | Method | Function | File | Key Service | Returns |
|----------|--------|----------|------|-------------|---------|
| `/api/tree` | GET | `getTree()` | tree.ts | becca | Tree structure (notes, branches, attributes) |
| `/api/tree/load` | POST | `load()` | tree.ts | becca | Notes and relationships for specific IDs |
| `/api/notes/:noteId` | GET | `getNote()` | notes.ts | becca | Note metadata |
| `/api/notes/:noteId/blob` | GET | `getNoteBlob()` | notes.ts | blobService | Note content |
| `/api/notes/:noteId/data` | PUT | `updateNoteData()` | notes.ts | noteService | Updated note data |
| `/api/notes/:noteId/title` | PUT | `changeTitle()` | notes.ts | noteService | Updated note |
| `/api/notes/:noteId/type` | PUT | `setNoteTypeMime()` | notes.ts | becca | (none) |
| `/api/notes/:noteId` | DELETE | `deleteNote()` | notes.ts | noteService | (none) |
| `/api/notes/:parentNoteId/children` | POST | `createNote()` | notes.ts | noteService | {note, branch} |
| `/api/branches/:branchId/expanded/:expanded` | PUT | `setExpanded()` | branches.ts | sql | (none) |
| `/api/branches/:branchId/move-to/:parentBranchId` | PUT | `moveBranchToParent()` | branches.ts | branchService | Updated branch |
| `/api/branches/:branchId` | DELETE | `deleteBranch()` | branches.ts | branchService | {noteDeleted} |
| `/api/notes/:noteId/attributes` | GET | `getEffectiveNoteAttributes()` | attributes.ts | becca | Attributes array |
| `/api/notes/:noteId/attributes` | PUT | `updateNoteAttributes()` | attributes.ts | BAttribute | (none) |
| `/api/notes/:noteId/attribute` | PUT | `updateNoteAttribute()` | attributes.ts | BAttribute | {attributeId} |
| `/api/notes/:noteId/attributes/:attributeId` | DELETE | `deleteNoteAttribute()` | attributes.ts | becca | (none) |
| `/api/quick-search/:searchString` | GET | `quickSearch()` | search.ts | searchService | {noteIds, results, error} |

## MEDIUM PRIORITY - Important for Complete Editing

| Endpoint | Method | Function | File | Key Service | Returns |
|----------|--------|----------|------|-------------|---------|
| `/api/notes/:noteId/file` | PUT | `updateFile()` | files.ts | noteService | {uploaded} |
| `/api/notes/:noteId/open` | GET | `openFile()` | files.ts | (stream) | File content |
| `/api/notes/:noteId/download` | GET | `downloadFile()` | files.ts | (stream) | File with headers |
| `/api/notes/:noteId/attachments` | GET | `getAttachments()` | attachments.ts | becca | Attachments array |
| `/api/notes/:noteId/attachments` | POST | `saveAttachment()` | attachments.ts | becca | (none) |
| `/api/notes/:noteId/attachments/upload` | POST | `uploadAttachment()` | attachments.ts | imageService | {uploaded, url} |
| `/api/attachments/:attachmentId` | GET | `getAttachment()` | attachments.ts | becca | Attachment object |
| `/api/attachments/:attachmentId/blob` | GET | `getAttachmentBlob()` | attachments.ts | blobService | Blob content |
| `/api/attachments/:attachmentId` | DELETE | `deleteAttachment()` | attachments.ts | becca | (none) |
| `/api/attachments/:attachmentId/rename` | PUT | `renameAttachment()` | attachments.ts | becca | (none) |
| `/api/notes/:noteId/revisions` | GET | `getRevisions()` | revisions.ts | becca | RevisionItem[] |
| `/api/revisions/:revisionId/blob` | GET | `getRevisionBlob()` | revisions.ts | blobService | Blob content |
| `/api/revisions/:revisionId` | GET | `getRevision()` | revisions.ts | becca | Revision with content |
| `/api/revisions/:revisionId/restore` | POST | `restoreRevision()` | revisions.ts | noteService | (none) |
| `/api/images/:noteId/:filename` | GET | `returnImageFromNote()` | image.ts | (stream) | Image content |
| `/api/images/:noteId` | PUT | `updateImage()` | image.ts | imageService | {uploaded} |
| `/api/options` | GET | `getOptions()` | options.ts | optionService | Options object |
| `/api/options/:name/:value` | PUT | `updateOption()` | options.ts | optionService | (none) |
| `/api/options` | PUT | `updateOptions()` | options.ts | optionService | (none) |

## LOWER PRIORITY - Advanced Features

| Endpoint | Method | Function | File | Category |
|----------|--------|----------|------|----------|
| `/api/search/:searchString` | GET | `search()` | search.ts | Advanced Search |
| `/api/search-note/:noteId` | GET | `searchFromNote()` | search.ts | Advanced Search |
| `/api/search-and-execute-note/:noteId` | POST | `searchAndExecute()` | search.ts | Advanced Search |
| `/api/search-related` | POST | `getRelatedNotes()` | search.ts | Advanced Search |
| `/api/search-templates` | GET | `searchTemplates()` | search.ts | Advanced Search |
| `/api/sync/check` | GET | `checkSync()` | sync.ts | Sync |
| `/api/sync/changed` | GET | `getChanged()` | sync.ts | Sync |
| `/api/sync/update` | PUT | `update()` | sync.ts | Sync |
| `/api/sync/test` | POST | `testSync()` | sync.ts | Sync |
| `/api/sync/now` | POST | `syncNow()` | sync.ts | Sync |
| `/api/notes/:noteId/duplicate/:parentNoteId` | POST | `duplicateSubtree()` | notes.ts | Cloning |
| `/api/notes/:noteId/protect/:isProtected` | PUT | `protectNote()` | notes.ts | Security |
| `/api/notes/:noteId/undelete` | PUT | `undeleteNote()` | notes.ts | Management |
| `/api/notes/erase-deleted-notes-now` | POST | `eraseDeletedNotesNow()` | notes.ts | Cleanup |
| `/api/notes/erase-unused-attachments-now` | POST | `eraseUnusedAttachmentsNow()` | notes.ts | Cleanup |
| `/api/revisions/:revisionId` | DELETE | `eraseRevision()` | revisions.ts | Cleanup |
| `/api/revisions/erase-all-excess-revisions` | POST | `eraseAllExcessRevisions()` | revisions.ts | Cleanup |
| `/api/special-notes/*` | GET/POST | Various | special_notes.ts | Date Notes |
| `/api/bulk-action/*` | POST | Various | bulk_action.ts | Batch Operations |
| `/api/clipper/*` | GET/POST | Various | clipper.ts | Web Clipper |

## Service Dependencies at a Glance

### Core Dependencies for Phase 1
- **becca** - Entity cache (notes, branches, attributes)
- **blobService** - Content storage layer
- **noteService** - Note creation/update logic
- **branchService** - Branch/tree operations
- **searchService** - Full-text search

### Transaction & Persistence
- **sql** - Database transactions
- **entityChangesService** - Change tracking (for sync)

### Supporting Services
- **optionService** - Settings storage
- **imageService** - Image optimization
- **treeService** - Tree validation

---

## Notes on Implementation

### Error Handling
- Most endpoints throw `NotFoundError` if entities don't exist
- Validation errors throw `ValidationError` with descriptive messages
- Protected sessions required for accessing protected notes

### Response Format
- Successful responses return either data (200) or empty (204)
- Errors return [statusCode, errorMessage] for custom codes
- Entities are automatically converted to POJOs via `convertEntitiesToPojo()`

### Authorization
- All API routes require `auth.checkApiAuth` middleware
- Some routes (files, images) allow electron clients without auth: `auth.checkApiAuthOrElectron`
- CSRF protection applied to all POST/PUT/DELETE via middleware

### Transaction Behavior
- Write operations wrapped in database transactions
- Changes trigger entity change events for sync
- Revisions automatically created for content changes

### Performance Considerations
- Tree endpoint can be expensive for large hierarchies
- Search results limited to 200 for quick-search
- Becca cache must be pre-loaded for fast lookups
- Blob deduplication reduces storage overhead
