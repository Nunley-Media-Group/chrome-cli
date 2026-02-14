# Requirements: File Upload to Page Elements

**Issue**: #23
**Date**: 2026-02-14
**Status**: Approved
**Author**: Claude (nmg-sdlc)

---

## User Story

**As a** developer or automation engineer
**I want** to upload files to file input elements via the CLI
**So that** my automation scripts can programmatically test file upload forms without manual browser interaction

---

## Background

The MCP server's `upload_file` tool allows uploading files through file input elements on a page. This CLI feature exposes the same capability via `chrome-cli form upload`, using CDP's `DOM.setFileInputFiles` to programmatically set files on `<input type="file">` elements. This is necessary for testing file upload forms, avatar uploaders, document submission workflows, and similar features.

The command targets elements by UID (from accessibility snapshot) or CSS selector, validates that the target is a file input element and that the specified files exist and are readable, then sets the files via CDP and dispatches a `change` event for framework compatibility.

---

## Acceptance Criteria

### AC1: Upload a single file by UID

**Given** Chrome is running with a page containing a file input element
**And** an accessibility snapshot has been taken with UIDs assigned
**When** I run `chrome-cli form upload <UID> <FILE_PATH>`
**Then** the file is uploaded to the file input element
**And** a `change` event is dispatched on the element
**And** JSON output is returned: `{"uploaded": "<UID>", "files": ["<FILE_PATH>"], "size": <BYTES>}`
**And** the exit code is 0

**Example**:
- Given: A page with `<input type="file" id="avatar">` and UID "s5"
- When: `chrome-cli form upload s5 /tmp/photo.jpg`
- Then: `{"uploaded": "s5", "files": ["/tmp/photo.jpg"], "size": 24576}`

### AC2: Upload multiple files

**Given** Chrome is running with a page containing a file input element that accepts multiple files
**And** an accessibility snapshot has been taken
**When** I run `chrome-cli form upload <UID> file1.jpg file2.jpg`
**Then** all specified files are set on the file input element
**And** JSON output includes all file paths and total size
**And** the exit code is 0

**Example**:
- Given: A page with `<input type="file" multiple>` and UID "s3"
- When: `chrome-cli form upload s3 /tmp/doc1.pdf /tmp/doc2.pdf`
- Then: `{"uploaded": "s3", "files": ["/tmp/doc1.pdf", "/tmp/doc2.pdf"], "size": 102400}`

### AC3: Upload with --tab flag targets specific tab

**Given** Chrome is running with multiple tabs open
**And** a file input element exists in a specific tab
**When** I run `chrome-cli form upload <UID> <FILE_PATH> --tab <TAB_ID>`
**Then** the file is uploaded in the specified tab

### AC4: Upload with --include-snapshot flag

**Given** Chrome is running with a page containing a file input element
**When** I run `chrome-cli form upload <UID> <FILE_PATH> --include-snapshot`
**Then** the JSON output includes a `snapshot` field with the updated accessibility tree
**And** the snapshot state file is updated with new UID mappings

### AC5: Upload by CSS selector

**Given** Chrome is running with a page containing a file input with id "file-upload"
**When** I run `chrome-cli form upload css:#file-upload /tmp/document.pdf`
**Then** the file is uploaded to the matching element
**And** JSON output is returned: `{"uploaded": "css:#file-upload", "files": ["/tmp/document.pdf"], "size": <BYTES>}`

### AC6: Error when file not found

**Given** Chrome is running with a page containing a file input element
**When** I run `chrome-cli form upload s5 /nonexistent/file.txt`
**Then** the exit code is nonzero
**And** stderr contains an error: file not found

### AC7: Error when element is not a file input

**Given** Chrome is running with a page containing a text input with UID "s2"
**When** I run `chrome-cli form upload s2 /tmp/file.txt`
**Then** the exit code is nonzero
**And** stderr contains an error that the element is not a file input

### AC8: Error when UID not found

**Given** Chrome is running with a snapshot taken
**When** I run `chrome-cli form upload s999 /tmp/file.txt`
**Then** the exit code is nonzero
**And** stderr contains an error about the UID not being found

### AC9: Error when file is not readable

**Given** Chrome is running with a page containing a file input element
**And** a file exists at /tmp/secret.txt but is not readable
**When** I run `chrome-cli form upload s5 /tmp/secret.txt`
**Then** the exit code is nonzero
**And** stderr contains an error about the file not being readable

### AC10: Upload without required arguments

**Given** chrome-cli is built
**When** I run `chrome-cli form upload`
**Then** the exit code is nonzero
**And** stderr contains usage information about required arguments

### AC11: Large file warning

**Given** Chrome is running with a page containing a file input element
**And** a file exists at /tmp/huge.bin that is very large (e.g. > 100MB)
**When** I run `chrome-cli form upload s5 /tmp/huge.bin`
**Then** the file is uploaded (CDP does not enforce size limits)
**And** stderr contains a warning about the file being large

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | `form upload <TARGET> <FILE_PATH>` uploads a single file | Must | Core single-file upload |
| FR2 | `form upload <TARGET> file1 file2 ...` uploads multiple files | Must | Multiple files support |
| FR3 | Target resolution via UID (s\d+) or CSS selector (css:) | Must | Consistent with form fill targeting |
| FR4 | Validate that target element is a file input | Must | Prevent misuse on non-file elements |
| FR5 | Validate that all file paths exist and are readable | Must | Early error before CDP call |
| FR6 | Dispatch `change` event after setting files | Must | Framework compatibility |
| FR7 | Return JSON with uploaded target, file paths, and total size | Must | Structured output |
| FR8 | `--include-snapshot` returns updated accessibility snapshot | Must | Snapshot integration |
| FR9 | `--tab <ID>` targets specific tab | Must | Tab targeting (via global flag) |
| FR10 | Warn on large files (> 100MB) | Should | User awareness |
| FR11 | `DOM.setFileInputFiles` CDP call with resolved paths | Must | Implementation mechanism |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Upload operation should complete in < 500ms (excluding file transfer time) |
| **Reliability** | Change event must be dispatched for React/Vue/Angular framework compatibility |
| **Platforms** | macOS, Linux, Windows (all platforms Chrome supports) |
| **Error handling** | Clear error messages for missing files, wrong element type, invalid UIDs |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| target (uid or selector) | String | Must be valid UID format (s\d+) or css: prefix | Yes |
| files | Vec<PathBuf> | Each path must exist and be readable; at least one required | Yes |

### Output Data

| Field | Type | Description |
|-------|------|-------------|
| uploaded | String | Target identifier that received the files |
| files | Vec<String> | File paths that were uploaded |
| size | u64 | Total size in bytes of all uploaded files |
| snapshot | Object (optional) | Accessibility tree if --include-snapshot |

---

## Dependencies

### Internal Dependencies
- [x] #4 -- CDP client (WebSocket communication)
- [x] #6 -- Session management (connection resolution)
- [x] #10 -- UID system (accessibility snapshot UIDs)

### External Dependencies
- Chrome/Chromium with CDP enabled

---

## Out of Scope

- Drag-and-drop file upload simulation
- File upload progress tracking
- File upload via URL (remote files)
- File upload size limits enforcement (Chrome handles this)
- Multi-step upload workflows (e.g., upload then submit form)
- ContentEditable / rich text editor file insertion

---

## Open Questions

None -- all requirements are clear from the issue specification.

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states specified
- [x] Dependencies identified
- [x] Out of scope defined
