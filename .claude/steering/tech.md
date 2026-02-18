# chrome-cli Technical Steering

This document defines the technology stack, constraints, and integration standards.
All technical decisions should align with these guidelines.

---

## Architecture Overview

```
CLI (clap args parsing)
    ↓
Command Dispatcher (main.rs)
    ↓
Command Modules (navigate.rs, tabs.rs, page.rs, ...)
    ↓
CDP Client (WebSocket JSON-RPC)
    ↓
Chrome Browser (DevTools Protocol)
```

---

## Technology Stack

| Layer | Technology | Version |
|-------|------------|---------|
| Language | Rust | Edition 2024 |
| Toolchain | rustc | 1.85.0 |
| Build system | Cargo | workspace, resolver v3 |
| CLI framework | clap | 4 (derive mode) |
| Async runtime | tokio | 1 (multi-thread) |
| WebSocket | tokio-tungstenite | 0.26 |
| Serialization | serde + serde_json | 1 |
| Config files | toml | 0.8 |
| Shell completions | clap_complete | 4 |
| Man pages | clap_mangen | 0.2 |
| Linting | Clippy | all=deny, pedantic=warn |
| Formatting | rustfmt | edition 2024 |

### External Services

| Service | Purpose | Notes |
|---------|---------|-------|
| Chrome/Chromium | Browser target | Connected via CDP over WebSocket |

---

## Versioning

The `VERSION` file (plain text semver at project root) is the **single source of truth** for the project's current version.

| File | Path | Notes |
|------|------|-------|
| VERSION | (entire file) | Plain text semver string |
| Cargo.toml | `package.version` | Root workspace package version |

---

## Technical Constraints

### Performance

| Metric | Target | Rationale |
|--------|--------|-----------|
| Startup time | < 50ms | CLI tools must feel instant |
| Binary size | < 10MB | Easy distribution |
| Memory usage | < 50MB idle | Don't hog resources while waiting for CDP |

### Security

| Requirement | Implementation |
|-------------|----------------|
| No telemetry | No data collection or phone-home |
| Local only | CDP connections only to localhost by default |
| Secrets management | No secrets stored; Chrome debug port is ephemeral |

---

## Off-Limits Files

Do NOT modify these files during SDLC steps unless the issue explicitly requires it:

- `.gitignore` — managed by the project owner
- `Cargo.lock` — updated only by `cargo` commands, never edited directly
- `.claude/` contents — managed by the SDLC runner

---

## Coding Standards

### Rust

```rust
// GOOD: Idiomatic Rust patterns
// - Use Result<T, E> for fallible operations
// - Derive common traits (Debug, Clone) where appropriate
// - Use thiserror or anyhow for error handling
// - Prefer &str over String in function parameters
// - Use builder pattern for complex configuration

// BAD: Patterns to avoid
// - unwrap() in library/non-test code
// - String for error types (use typed errors)
// - Bare println! for user-facing output (use a structured output layer)
// - Clippy allows/suppressions without justification
```

### Clippy Configuration

- `all = "deny"` — All clippy lints are errors
- `pedantic = "warn"` — Pedantic lints are warnings

This is a strict configuration. All clippy warnings should be addressed before merging.

### Formatting (rustfmt)

`cargo fmt` **MUST** pass during verification. Before proceeding past any verification gate:

1. Run `cargo fmt --check` to detect formatting violations
2. If violations are found, run `cargo fmt` to auto-correct them
3. Stage and include the formatting fixes in the current work (do not create a separate commit)
4. Re-run `cargo fmt --check` to confirm all violations are resolved

**Do not skip or defer formatting fixes.** Correct them immediately and automatically before continuing.

---

## CLI Interface Standards

### Command Structure

```
chrome-cli <command> [subcommand] [options] [arguments]

# Examples:
chrome-cli connect --launch --headless
chrome-cli navigate <url>
chrome-cli page screenshot --full-page --file shot.png
chrome-cli js exec "document.title"
chrome-cli tabs list
chrome-cli form fill <uid> <value>
```

### Output Format

```
# All data output: structured JSON on stdout
{"title": "Example Domain", "url": "https://example.com"}

# All errors: structured JSON on stderr
{"error": "message", "code": 2}

# Exit codes: 0=success, 1=general, 2=connection, 3=target, 4=timeout, 5=protocol
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `CHROME_CLI_PORT` | CDP port number (default: 9222) |
| `CHROME_CLI_HOST` | CDP host address (default: 127.0.0.1) |
| `CHROME_CLI_TIMEOUT` | Default command timeout in milliseconds |
| `CHROME_CLI_CONFIG` | Path to configuration file |
| `NO_COLOR` | Disable colored output (standard convention) |

---

## Testing Standards

### Chrome Instance Cleanup (CRITICAL)

**Always close any headed Chrome instance you open.** During implementation and verification, if you launch a headed (non-headless) Chrome browser for testing or debugging, you MUST ensure it is closed/killed when you are done. Leaving headed Chrome instances running wastes system resources and can interfere with subsequent test runs or CDP connections.

- After running integration/BDD tests that launch headed Chrome, verify the process is terminated
- If a test or command opens a headed Chrome instance, ensure cleanup happens even on failure
- Before finishing any implementation or verification session, check for orphaned Chrome processes and kill them

### Manual Smoke Test (Required for Verification)

**Every feature and bug fix MUST include a manual smoke test against a real headless Chrome instance during `/verifying-specs`.** Automated BDD tests skip Chrome-dependent scenarios in CI, so the smoke test is the only end-to-end verification that the implementation works against a real browser.

#### Procedure

1. Build in debug mode: `cargo build`
2. Launch headless Chrome: `./target/debug/chrome-cli connect --launch --headless`
3. Exercise the feature/fix using the reproduction steps from `requirements.md` or the acceptance criteria
4. Verify each AC produces the expected output against the real browser
5. **Run the SauceDemo smoke test** (see below)
6. Disconnect: `./target/debug/chrome-cli connect disconnect`
7. Kill any orphaned Chrome processes: `pkill -f 'chrome.*--remote-debugging' || true`

#### SauceDemo Smoke Test (Required)

**Every `/verifying-specs` run MUST include a smoke test against https://www.saucedemo.com/.** This validates the debug build against a real-world web application with login forms, navigation, and dynamic content.

Minimum steps:

1. Navigate to the site: `./target/debug/chrome-cli navigate https://www.saucedemo.com/`
2. Take a snapshot: `./target/debug/chrome-cli page snapshot`
3. Exercise the feature/fix against the site where applicable (e.g., form fill on the login page, screenshot, element finding, dialog handling)
4. Verify the command output is correct and the page responds as expected

If the feature under test is not directly exercisable against SauceDemo (e.g., a pure config or shell-completion change), the SauceDemo test still runs the navigate + snapshot steps as a baseline integration check.

#### Requirements

- The smoke test MUST appear as a task in `tasks.md` (typically the final task before "Verify No Regressions")
- During `/verifying-specs`, execute the smoke test task and record pass/fail results in the verification report
- If the smoke test fails, treat it as a Critical finding — the implementation does not meet acceptance criteria
- For defect fixes, the smoke test MUST reproduce the exact steps from the issue's reproduction section and confirm the bug no longer occurs
- The SauceDemo smoke test results MUST be recorded in the verification report alongside the feature-specific smoke test results

### BDD Testing (Required for nmg-sdlc)

**Every acceptance criterion MUST have a Gherkin test.**

| Layer | Framework | Location | Run Command |
|-------|-----------|----------|-------------|
| BDD/Acceptance | cucumber-rs 0.21 | `tests/features/*.feature` | `cargo test --test bdd` |
| BDD step definitions | cucumber-rs | `tests/bdd.rs` | (single file, all worlds) |
| Integration | built-in (#[test]) | `tests/*.rs` | `cargo test --test '*'` |
| Unit | built-in (#[test]) | `src/**/*.rs` (inline) | `cargo test --lib` |

### Test Pyramid

```
        /\
       /  \  BDD Integration (Gherkin + cucumber-rs)
      /----\  - Acceptance criteria as tests
     /      \ - End-to-end CLI invocations
    /--------\
   /          \  Integration Tests
  /            \ - CDP client behavior
 /--------------\
/                \  Unit Tests
 \________________/ - Argument parsing, output formatting
```

---

## References

- `.claude/steering/product.md` for product direction
- `.claude/steering/structure.md` for code organization
