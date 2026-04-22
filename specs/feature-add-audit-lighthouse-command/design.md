# Technical Design: Add `audit lighthouse` Command

**Issues**: #169, #231
**Date**: 2026-04-22
**Status**: Amended
**Author**: Claude

---

## Architecture Overview

The `audit lighthouse` command follows the established command-group pattern used by `perf`, `tabs`, `cookie`, etc. It adds a new `audit` command group with a `lighthouse` subcommand that shells out to the external `lighthouse` CLI binary rather than using CDP directly.

```
CLI (clap) → main.rs dispatch → audit.rs → std::process::Command("lighthouse") → parse JSON → stdout
```

This is architecturally simpler than most commands (no CDP session needed for the audit itself), but still requires session/connection resolution to determine the Chrome port and optionally the active page URL.

---

## Component Design

### 1. CLI Layer (`src/cli/mod.rs`)

Add to the `Command` enum:

```rust
/// Run audits against the current page (Lighthouse)
Audit(AuditArgs),
```

New types:

```rust
pub struct AuditArgs {
    pub command: AuditCommand,
}

pub enum AuditCommand {
    /// Run a Google Lighthouse audit
    Lighthouse(AuditLighthouseArgs),
}

pub struct AuditLighthouseArgs {
    /// URL to audit (defaults to active page URL)
    pub url: Option<String>,
    /// Comma-separated list of categories to audit
    pub only: Option<String>,
    /// Path to save the full Lighthouse JSON report
    pub output_file: Option<PathBuf>,
}
```

### 2. Command Module (`src/audit.rs`)

New file following the pattern of `perf.rs`, `cookie.rs`, etc.

**Public entry point:**

```rust
pub async fn execute_audit(global: &GlobalOpts, args: &AuditArgs) -> Result<(), AppError>
```

**Internal flow:**

1. **Resolve connection** — `resolve_connection(host, port, ws_url)` to get the Chrome port
2. **Resolve URL** — If no positional URL arg, query `resolve_target()` + `Target.getTargets` to get the active page's URL
3. **Find lighthouse binary** — Use `which::which("lighthouse")` or `std::process::Command::new("which").arg("lighthouse")` to locate the binary. Since the project avoids adding dependencies unnecessarily, use a simple `PATH`-based lookup via `std::process::Command`.
4. **Build lighthouse command** — Construct `lighthouse <URL> --port <PORT> --output json --chrome-flags="--headless" --only-categories=<list>`
5. **Execute and capture output** — Run via `std::process::Command`, capture stdout/stderr
6. **Parse Lighthouse JSON** — Extract `lhr.categories[name].score` fields
7. **Format output** — Emit flat JSON scores summary to stdout
8. **Optionally save full report** — Write raw Lighthouse JSON to `--output-file`

### 3. Dispatch (`src/main.rs`)

Add match arm:

```rust
Command::Audit(args) => audit::execute_audit(&global, args).await,
```

Add module declaration:

```rust
mod audit;
```

### 4. Library Target (`src/lib.rs`)

No changes needed — `audit.rs` is a binary-crate command module like all others. The `lib.rs` only exposes shared infrastructure (`cdp`, `chrome`, `connection`, `session`, `error`).

---

## Data Flow

```
User invokes: agentchrome audit lighthouse [URL] [--only ...] [--output-file ...]
    │
    ▼
resolve_connection(host, port, ws_url) → ResolvedConnection { port }
    │
    ▼
[If no URL arg] resolve_target(host, port, tab, page_id) → TargetInfo { url }
    │
    ▼
Locate "lighthouse" binary in PATH
    │ Not found → AppError { "lighthouse binary not found...", code: 1 }
    ▼
Build command: lighthouse <URL> --port <PORT> --output json --chrome-flags="--headless"
    │ [If --only] append: --only-categories=performance,accessibility,...
    ▼
Execute std::process::Command, capture stdout + stderr + exit code
    │ Non-zero exit → AppError { stderr message, code: 1 }
    ▼
Parse stdout as JSON: lhr.categories.<name>.score
    │
    ▼
Build scores summary: {"url":"...","performance":0.91,"accessibility":0.87,...}
    │
    ├─[If --output-file] write raw lighthouse JSON to file
    ▼
Print scores summary JSON to stdout
```

---

## Lighthouse Binary Interaction

### Binary Discovery

Use a simple `PATH`-based lookup without adding external crates:

```rust
fn find_lighthouse_binary() -> Result<PathBuf, AppError> {
    // Try "lighthouse" in PATH using Command
    let output = std::process::Command::new("which")
        .arg("lighthouse")
        .output();
    // On Windows, use "where" instead
    // Parse output to get path, or return error with install hint
}
```

Cross-platform approach: attempt to run `lighthouse --version` and check if it succeeds. This is simpler and works across macOS, Linux, and Windows without `which`/`where` branching.

### Lighthouse CLI Arguments

```
lighthouse <URL> \
  --port <PORT> \
  --output json \
  --chrome-flags="--headless" \
  [--only-categories=performance,accessibility,best-practices,seo,pwa]
```

Key flags:
- `--port`: Connects to the existing Chrome instance managed by agentchrome
- `--output json`: Machine-readable output (the full Lighthouse Result object)
- `--chrome-flags="--headless"`: Required even if Chrome is already headless; Lighthouse uses this to configure its internal behavior
- `--only-categories`: Comma-separated category IDs to audit

### Output Parsing

Lighthouse JSON output structure (relevant subset):

```json
{
  "requestedUrl": "https://example.com",
  "finalUrl": "https://example.com",
  "categories": {
    "performance": { "score": 0.91 },
    "accessibility": { "score": 0.87 },
    "best-practices": { "score": 0.93 },
    "seo": { "score": 0.90 },
    "pwa": { "score": 0.30 }
  }
}
```

Extract `categories.<name>.score` for each category. Scores are `f64` in range `[0.0, 1.0]` or `null` (unmeasurable).

---

## Category Validation

Valid category names: `performance`, `accessibility`, `best-practices`, `seo`, `pwa`.

When `--only` is provided, validate each comma-separated value against this list before invoking Lighthouse. Return a structured error for invalid category names.

---

## Error Handling

| Condition | Error Message | Exit Code |
|-----------|--------------|-----------|
| `lighthouse` not in PATH | `"lighthouse binary not found. Install it with: npm install -g lighthouse"` | 1 (GeneralError) |
| No active session | Standard `AppError::no_session()` | 2 (ConnectionError) |
| Lighthouse exits non-zero | `"Lighthouse audit failed: <stderr>"` | 1 (GeneralError) |
| Invalid `--only` category | `"Invalid category: '<name>'. Valid categories: performance, accessibility, best-practices, seo, pwa"` | 1 (GeneralError) |
| Failed to parse Lighthouse output | `"Failed to parse Lighthouse output: <reason>"` | 1 (GeneralError) |
| Failed to write output file | Standard `AppError::file_write_failed()` | 1 (GeneralError) |

---

## Output Types

```rust
#[derive(Serialize)]
struct AuditLighthouseResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    performance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accessibility: Option<f64>,
    #[serde(rename = "best-practices", skip_serializing_if = "Option::is_none")]
    best_practices: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seo: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pwa: Option<f64>,
}
```

When `--only` is used, only the requested categories are populated; unrequested categories are `None` (omitted from JSON via `skip_serializing_if`). When a requested category has a `null` score from Lighthouse, the field is present as `null` — this requires a wrapper to distinguish `Some(None)` (requested but null) from `None` (not requested).

Revised approach using an explicit wrapper:

```rust
#[derive(Serialize)]
struct AuditLighthouseResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    performance: Option<Option<f64>>,
    // ... same pattern for others
}
```

Or simpler: build the JSON object manually with `serde_json::Map` to control exactly which keys appear and whether values are `null` vs absent.

---

## Alternatives Considered

### 1. Use Lighthouse as a library (Node.js)

Rejected: Would require bundling Node.js or running a subprocess to a JS script. Shelling out to the `lighthouse` binary is simpler, more maintainable, and consistent with the tool's philosophy of composing existing CLI tools.

### 2. Implement audits via CDP directly

Rejected: Lighthouse performs hundreds of individual audits with complex scoring logic. Reimplementing this would be enormous and fragile. The `lighthouse` binary is the canonical implementation.

### 3. Add `lighthouse` as a Cargo dependency

Not possible: Lighthouse is a Node.js tool, not a Rust crate.

### 4. Use `which` crate for binary discovery

Rejected: Adds an external dependency for a simple PATH lookup. A `Command::new("lighthouse").arg("--version")` probe is sufficient and dependency-free.

---

## Testing Strategy

### BDD Tests (cucumber-rs)

- **CLI-testable** (no Chrome needed): argument validation, help text, `--only` with invalid categories, subcommand requirement
- **Chrome-dependent** (skipped in CI): full audit run, URL override, output file generation — verified via manual smoke test

### Unit Tests

- Category validation logic
- Lighthouse output JSON parsing
- Score extraction with null handling
- Output serialization (requested vs unrequested categories)

---

## Amendment: Prerequisite Handling (Issue #231)

### Motivation

On a fresh machine, `audit lighthouse` fails with `"lighthouse binary not found"`. The error is actionable, but because `audit` appears in `--help` as a first-class command group, the UX reads as "advertised feature that doesn't ship with the tool". This amendment adds three coordinated surfaces so the prerequisite is discoverable *before* failure, one-command-installable, and cross-referenced at failure time.

### Component Additions

#### CLI Layer (`src/cli/mod.rs`)

Extend `AuditLighthouseArgs` with an opt-in install flag:

```rust
pub struct AuditLighthouseArgs {
    pub url: Option<String>,
    #[arg(long)] pub only: Option<String>,
    #[arg(long)] pub output_file: Option<PathBuf>,

    /// Install the `lighthouse` npm package (requires `npm` on PATH).
    /// Running this flag constitutes explicit consent to run `npm install -g lighthouse`.
    #[arg(long)] pub install_prereqs: bool,
}
```

Update the `#[command(about = "...", long_about = "...")]` strings:

- **Top-level `audit` group `about`** (shown by `agentchrome --help` and `agentchrome audit --help`): append `"(requires lighthouse CLI — see 'audit lighthouse --help')"`.
- **`audit lighthouse` subcommand `long_about`**: lead with a `PREREQUISITES:` section above the `EXAMPLES:` block that states `"Requires the lighthouse npm package. Install with: npm install -g lighthouse (or run: agentchrome audit lighthouse --install-prereqs)"`.

Flag-name collision check per retrospective learning: `--install-prereqs` does not conflict with any existing global flag (`--port`, `--host`, `--ws-url`, `--page-id`, `--tab`, `--json`, `--verbose`) or any `audit lighthouse` subcommand flag (`--only`, `--output-file`).

#### Command Module (`src/audit.rs`)

Branch `execute_lighthouse` at the top on the install flag:

```rust
if args.install_prereqs {
    return install_lighthouse_prereqs();
}
// ... existing flow
```

New function:

```rust
fn install_lighthouse_prereqs() -> Result<(), AppError> {
    // 1. Probe npm: `npm --version`
    // 2. If npm missing → structured AppError with install-Node.js hint
    // 3. Run `npm install -g lighthouse`, stream or capture output
    // 4. On success, probe `lighthouse --version` and emit {"installed":"lighthouse","version":"<v>"}
    // 5. On npm failure (non-zero exit), wrap stderr into AppError
}
```

Extend the existing `find_lighthouse_binary` not-found error message:

```
"lighthouse binary not found. Install it with: npm install -g lighthouse
Or run: agentchrome audit lighthouse --install-prereqs"
```

The error remains a single `AppError` → single JSON object on stderr (one invocation = one error object). The newline-separated hint inside the `error` string is acceptable because it is one field on one object.

### Data Flow Additions

```
agentchrome audit lighthouse --install-prereqs
    │
    ▼
Probe `npm --version`
    │ Not found → AppError { "npm not found on PATH — install Node.js first", code: 1 }
    ▼
Execute `npm install -g lighthouse`, capture stdout+stderr+exit
    │ Non-zero exit → AppError { "Failed to install lighthouse: <stderr>", code: 1 }
    ▼
Probe `lighthouse --version` (post-install verification)
    │ Not found → AppError { "lighthouse installed but not on PATH — shell rehash required", code: 1 }
    ▼
Emit {"installed":"lighthouse","version":"<v>"} on stdout, exit 0
```

### npm Execution

Use `std::process::Command::new("npm").args(["install", "-g", "lighthouse"])`. Inherit the current stdio so the user sees npm's own progress (npm writes to stderr/stdout in mixed form) — or capture both and relay structured progress via a log line. Preferred: inherit stdio for visibility during the install, then emit the structured success JSON on stdout once npm exits 0. This matches the approach of commands that wrap long-running subprocesses (see `perf.rs`).

Windows: `npm` is typically `npm.cmd` on Windows. Use `Command::new("npm")` and rely on the OS PATHEXT resolution; if that proves insufficient, fall back to explicitly trying `npm.cmd` before erroring (standard Rust-on-Windows pattern).

### Error Handling Additions

| Condition | Error Message | Exit Code |
|-----------|--------------|-----------|
| `npm` not in PATH (during `--install-prereqs`) | `"npm not found on PATH — install Node.js first"` | 1 (GeneralError) |
| `npm install -g lighthouse` exits non-zero | `"Failed to install lighthouse: <stderr>"` | 1 (GeneralError) |
| Post-install `lighthouse --version` still fails | `"lighthouse installed but not on PATH — open a new shell and retry"` | 1 (GeneralError) |
| Extended not-found hint (no install flag) | `"lighthouse binary not found. Install it with: npm install -g lighthouse\nOr run: agentchrome audit lighthouse --install-prereqs"` | 1 (GeneralError) |

### Output Types

New success payload for `--install-prereqs`:

```rust
#[derive(Serialize)]
struct InstallPrereqsResult<'a> {
    installed: &'a str,  // "lighthouse"
    version: String,     // parsed from `lighthouse --version`
}
```

### Help-Text Rendering

`clap` derives help from `#[command(about = ...)]` and `#[command(long_about = ...)]`. The top-level short `about` appears in `agentchrome --help`'s command list AND in `agentchrome audit --help`'s parent description — a single edit covers both AC12 entry points. `long_about` only renders on `agentchrome audit lighthouse --help` (AC9).

### Alternatives Considered (Amendment)

#### 1. Auto-install on first failure (no flag required)

Rejected: violates the issue's explicit "flag IS the consent" requirement. Silently running `npm install -g` is a side effect users should opt into; an automation context could easily trigger this repeatedly in CI.

#### 2. Bundle `lighthouse` into the `agentchrome` binary

Deferred per FR15. Lighthouse pulls in Chromium/Node.js dependencies that exceed the 10 MB binary-size target in `steering/tech.md`. Revisit once cross-platform packaging is explored.

#### 3. Prompt interactively before running `npm install`

Rejected: agentchrome is a non-interactive CLI. The `--install-prereqs` flag is the interaction — an additional prompt would break automation.

#### 4. Add a `doctor` subcommand instead of a per-command `--install-prereqs`

Considered for future work. A `agentchrome doctor` that inspects all prerequisites across all command groups is a cleaner long-term shape, but for Issue #231's scope (audit lighthouse specifically), a per-command flag is the minimal fix.

### Testing Strategy (Amendment)

**CLI-testable (no Chrome, no npm side-effect)**:
- `agentchrome audit lighthouse --help` includes `"lighthouse npm package"` / install hint above examples
- `agentchrome audit --help` and `agentchrome --help` mention the lighthouse prerequisite in the audit group line
- `agentchrome audit lighthouse` when lighthouse missing emits extended error mentioning `--install-prereqs`

**Mocked-subprocess tests**:
- `install_lighthouse_prereqs` error path when `npm` is absent (stub `Command` via trait or dependency injection for the probe)

**Manual smoke (side-effectful)**:
- On a machine *without* lighthouse: run `--install-prereqs`, verify install succeeds, verify subsequent invocation locates the binary across a fresh shell
- On a machine *without* npm: verify the npm-missing error fires and exit code is non-zero

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #169 | 2026-03-16 | Initial technical design |
| #231 | 2026-04-22 | Added `--install-prereqs` flag, help-text prerequisite surfacing (subcommand + group), extended not-found error with install-flag pointer, and npm-missing handling |
