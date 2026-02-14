# Design: File Upload to Page Elements

**Issue**: #23
**Date**: 2026-02-14
**Status**: Approved
**Author**: Claude (nmg-sdlc)

---

## Overview

This feature adds an `upload` subcommand to the existing `form` command group in chrome-cli. The implementation follows the same architecture as the existing `fill`, `fill-many`, and `clear` operations in `src/form.rs` -- a new `FormCommand::Upload` variant with corresponding `FormUploadArgs` struct and an `execute_upload` function.

File upload uses CDP's `DOM.setFileInputFiles` to programmatically set files on `<input type="file">` elements. Before calling CDP, the implementation validates that all file paths exist and are readable, and verifies that the target element is a file input by inspecting its type via `Runtime.callFunctionOn`. Target elements are resolved from UIDs (via the snapshot UID map) or CSS selectors, reusing the existing target resolution pattern from `form.rs`.

---

## Architecture

### Component Diagram

```
CLI Input (chrome-cli form upload s5 /tmp/photo.jpg)
    |
+------------------+
|   CLI Layer      |  <- Parse args: FormArgs -> FormCommand::Upload(FormUploadArgs)
|   cli/mod.rs     |
+--------+---------+
         |
+------------------+
|  Command Layer   |  <- form.rs: validate files, resolve target, validate element type,
|   form.rs        |     call DOM.setFileInputFiles, dispatch change event
+--------+---------+
         |
+------------------+
|   CDP Layer      |  <- DOM.resolveNode, Runtime.callFunctionOn, DOM.setFileInputFiles
|   ManagedSession |
+--------+---------+
         |
   Chrome Browser
```

### Data Flow

```
1. User runs: chrome-cli form upload s5 /tmp/photo.jpg
2. CLI layer parses args into FormUploadArgs { target: "s5", files: ["/tmp/photo.jpg"], ... }
3. form.rs dispatcher calls execute_upload()
4. Validate all file paths exist and are readable; compute total size
5. Warn to stderr if any file > 100MB
6. Setup CDP session (resolve_connection -> resolve_target -> CdpClient::connect)
7. Enable DOM and Runtime domains
8. Resolve target "s5" to backend node ID via snapshot state
9. Validate element is <input type="file"> via Runtime.callFunctionOn
10. Call DOM.setFileInputFiles with file paths and backend node ID
11. Dispatch change event via Runtime.callFunctionOn
12. Optionally take snapshot if --include-snapshot
13. Return JSON result: {"uploaded": "s5", "files": ["/tmp/photo.jpg"], "size": 24576}
```

---

## API / Interface Changes

### New CLI Command

| Command | Args | Purpose |
|---------|------|---------|
| `form upload <TARGET> <FILES>...` | `--include-snapshot` | Upload files to a file input element |

### CLI Arg Struct (in cli/mod.rs)

```rust
// Add to FormCommand enum:
Upload(FormUploadArgs),

// New args struct:
#[derive(Args)]
pub struct FormUploadArgs {
    /// Target file input element (UID like s5 or CSS selector like css:#file-input)
    pub target: String,

    /// File paths to upload
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Include updated accessibility snapshot in output
    #[arg(long)]
    pub include_snapshot: bool,
}
```

### Output Schema

#### `form upload` output

```json
{
  "uploaded": "s5",
  "files": ["/tmp/photo.jpg"],
  "size": 24576
}
```

With `--include-snapshot`:
```json
{
  "uploaded": "s5",
  "files": ["/tmp/photo.jpg"],
  "size": 24576,
  "snapshot": { ... }
}
```

### Errors

| Condition | Error Message |
|-----------|---------------|
| File not found | `"File not found: /nonexistent/file.txt"` |
| File not readable | `"File not readable: /tmp/secret.txt"` |
| Element is not a file input | `"Element is not a file input: s2"` |
| UID not found in snapshot | `"UID 's999' not found. Run 'chrome-cli page snapshot' first."` |
| No snapshot state | `"No snapshot state found. Run 'chrome-cli page snapshot' first to assign UIDs to interactive elements."` |
| CSS selector matches no element | `"Element not found for selector: #nonexistent"` |
| CDP setFileInputFiles failed | `"Interaction failed (setFileInputFiles): <CDP error>"` |

---

## State Management

No new persistent state. The feature reuses:
- **Snapshot state** (`~/.chrome-cli/snapshot.json`) -- read UID-to-backendNodeId mappings
- **Session state** (`~/.chrome-cli/session.json`) -- resolve CDP connection

When `--include-snapshot` is used, snapshot state is updated (same pattern as `form fill`).

---

## Implementation Details

### File Validation (before CDP calls)

Before connecting to Chrome, validate all file paths:

1. Check each path exists (`Path::exists()`)
2. Check each path is a file (`Path::is_file()`)
3. Check each path is readable (attempt `std::fs::metadata()`)
4. Compute file sizes, sum total
5. If any file > 100MB, print warning to stderr

### Element Type Validation via Runtime.callFunctionOn

After resolving the target to an object ID, verify it's a file input:

```javascript
function() {
  return this.tagName === 'INPUT' && this.type === 'file';
}
```

If the function returns `false`, return an error indicating the element is not a file input.

### CDP Calls Sequence

```
1. DOM.enable
2. Runtime.enable
3. Resolve UID to backendNodeId (via snapshot state) or CSS selector to backendNodeId
4. DOM.resolveNode({ backendNodeId }) -> { object: { objectId } }
5. Runtime.callFunctionOn({ objectId, functionDeclaration: "validate is file input" })
6. DOM.setFileInputFiles({ files: ["/path/to/file"], backendNodeId })
7. Runtime.callFunctionOn({ objectId, functionDeclaration: "dispatch change event" })
8. (Optional) Accessibility.getFullAXTree for snapshot
```

### Change Event Dispatch

After setting files, dispatch a `change` event for framework compatibility:

```javascript
function() {
  this.dispatchEvent(new Event('change', { bubbles: true }));
}
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: DOM.setFileInputFiles only** | Just call the CDP method | Simplest, one CDP call | No event dispatch; frameworks won't detect the change | Rejected -- incomplete |
| **B: DOM.setFileInputFiles + change event** | Set files via CDP, then dispatch change event via JS | Reliable, frameworks detect change, matches MCP server approach | Two CDP calls | **Selected** |
| **C: Simulate click + native file dialog** | Trigger the file dialog programmatically | Most realistic user interaction | Cannot be automated headlessly; requires OS-level file dialog interaction | Rejected -- not automatable |

---

## Security Considerations

- [x] **File path validation**: All paths validated to exist and be readable before sending to CDP
- [x] **No arbitrary code execution**: File paths are passed directly to CDP, not interpolated into JS
- [x] **Local files only**: CDP `DOM.setFileInputFiles` only accepts local file paths
- [x] **No sensitive data storage**: File paths are transient, not persisted

---

## Performance Considerations

- [x] **File validation before CDP**: Fail fast on missing files without needing a browser connection
- [x] **Single CDP call**: `DOM.setFileInputFiles` handles multiple files in one call
- [x] **Snapshot is optional**: Only taken when `--include-snapshot` is passed
- [x] **Size computation is local**: File sizes computed via `std::fs::metadata`, no CDP overhead

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI args | Unit | Arg parsing, required files validation |
| File validation | Unit | File exists, is readable, size computation, large file warning |
| Element type validation | Integration (BDD) | Verify file input check rejects non-file elements |
| Upload single file | Integration (BDD) | Single file upload, JSON output, change event |
| Upload multiple files | Integration (BDD) | Multiple files, JSON output |
| Error handling | Integration (BDD) | Missing file, wrong element type, invalid UID |
| Output serialization | Unit | JSON output struct serialization |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Change event not detected by framework | Low | Medium | Dispatch `change` event with `bubbles: true`; same approach as `form fill` |
| File path encoding on Windows | Low | Low | Use `PathBuf` and `to_string_lossy()` for path conversion |
| Large file causes Chrome to hang | Low | Medium | Warn on files > 100MB; CDP handles the actual transfer |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed (reuses existing snapshot state)
- [x] State management approach is clear (no new state)
- [x] N/A -- CLI tool, no UI components
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives considered and documented
- [x] Risks identified with mitigations
