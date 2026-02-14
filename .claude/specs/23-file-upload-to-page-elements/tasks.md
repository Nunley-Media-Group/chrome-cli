# Tasks: File Upload to Page Elements

**Issue**: #23
**Date**: 2026-02-14
**Status**: Planning
**Author**: Claude (nmg-sdlc)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| Backend | 2 | [ ] |
| Integration | 1 | [ ] |
| Testing | 2 | [ ] |
| **Total** | **6** | |

---

## Phase 1: Setup

### T001: Add FormUploadArgs CLI type and Upload variant to FormCommand

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `FormUploadArgs` struct with: `target: String`, `files: Vec<PathBuf>` (required, at least one), `include_snapshot: bool`
- [ ] `FormCommand::Upload(FormUploadArgs)` variant added to enum
- [ ] `files` argument uses `#[arg(required = true)]` to enforce at least one file
- [ ] Help text describes the command purpose and arguments
- [ ] `cargo check` passes

**Notes**: Follow the same pattern as `FormFillArgs`. The `--tab` flag is already global so no per-command addition needed.

---

## Phase 2: Backend Implementation

### T002: Implement execute_upload function in form.rs

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `UploadResult` output struct: `uploaded: String`, `files: Vec<String>`, `size: u64`, `snapshot: Option<Value>`
- [ ] Plain text print helper for upload result
- [ ] `execute_upload` function that:
  - Validates all file paths exist, are files, and are readable
  - Computes total file size from metadata
  - Warns to stderr if any file > 100MB
  - Sets up CDP session (reuse `setup_session`)
  - Enables DOM and Runtime domains
  - Resolves target to backend node ID (reuse `resolve_target_to_backend_node_id`)
  - Resolves to object ID (reuse `resolve_to_object_id`)
  - Validates element is `<input type="file">` via `Runtime.callFunctionOn`
  - Calls `DOM.setFileInputFiles` with file paths and backend node ID
  - Dispatches `change` event via `Runtime.callFunctionOn`
  - Optionally takes snapshot if `--include-snapshot`
  - Returns `UploadResult` as JSON output
- [ ] Handles error cases: file not found, file not readable, element not a file input, UID not found
- [ ] `cargo check` passes

**Notes**: Reuse existing helpers from form.rs: `setup_session`, `resolve_target_to_backend_node_id`, `resolve_to_object_id`, `take_snapshot`, `get_current_url`, `print_output`. Add new error helpers to `error.rs` as needed (`file_not_found`, `file_not_readable`, `not_file_input`).

### T003: Add error helpers to error.rs

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `file_not_found(path: &str)` error helper: `"File not found: {path}"`
- [ ] `file_not_readable(path: &str)` error helper: `"File not readable: {path}"`
- [ ] `not_file_input(target: &str)` error helper: `"Element is not a file input: {target}"`
- [ ] Unit tests for each new error helper
- [ ] `cargo test` passes for error module

---

## Phase 3: Integration

### T004: Wire Upload variant into form dispatch in form.rs

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `FormCommand::Upload(args) => execute_upload(global, args).await` added to `execute_form` match
- [ ] `cargo check` passes
- [ ] `cargo clippy` passes with no new warnings

**Notes**: The `form` module is already wired into `main.rs` -- only the internal dispatch in `execute_form` needs the new arm.

---

## Phase 4: Testing

### T005: Create BDD feature file for form upload

**File(s)**: `tests/features/form_upload.feature`
**Type**: Create
**Depends**: T002, T004
**Acceptance**:
- [ ] All 11 acceptance criteria from requirements.md have corresponding scenarios
- [ ] Uses Given/When/Then format
- [ ] Includes error handling scenarios (file not found, wrong element type, invalid UID)
- [ ] Includes multi-file upload scenario
- [ ] Valid Gherkin syntax

### T006: Add unit tests for upload to form.rs

**File(s)**: `src/form.rs`
**Type**: Modify
**Depends**: T002
**Acceptance**:
- [ ] `UploadResult` serialization tests (with and without snapshot)
- [ ] File validation logic tests (nonexistent path, non-file path)
- [ ] `cargo test` passes

---

## Dependency Graph

```
T001 ──┬──> T002 ──┬──> T004
       |           |
T003 ──┘           └──> T005, T006
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
