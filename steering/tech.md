# agentchrome Technical Steering

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
| Toolchain | rustc | 1.93.0 |
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
| Cargo.toml | `package.rust-version` | Minimum supported Rust version |

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
- `.codex/` contents — managed by Codex and the SDLC runner

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
agentchrome <command> [subcommand] [options] [arguments]

# Examples:
agentchrome connect --launch --headless
agentchrome navigate <url>
agentchrome page screenshot --full-page --file shot.png
agentchrome js exec "document.title"
agentchrome tabs list
agentchrome form fill <uid> <value>
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
| `AGENTCHROME_PORT` | CDP port number (default: 9222) |
| `AGENTCHROME_HOST` | CDP host address (default: 127.0.0.1) |
| `AGENTCHROME_TIMEOUT` | Default command timeout in milliseconds |
| `AGENTCHROME_CONFIG` | Path to configuration file |
| `NO_COLOR` | Disable colored output (standard convention) |

### Progressive Disclosure for Listings (Required for New Commands)

**Every command that returns a collection of items MUST support progressive disclosure.** AI agents are a primary user (see `product.md`), and flooding agent context with full item bodies on every discovery call wastes tokens and money.

#### The rule

| Call shape | Payload |
|------------|---------|
| **Listing** — `<cmd> list` or `<cmd> <collection>` (no item selected) | ONLY identifying fields: `name` / `id`, a short `title`, and a one-line `summary`. Nothing else. Aim for \u2264 4 KB JSON and \u2264 1 KB plain text for a typical listing of ~10 items. |
| **Detail** — `<cmd> <collection> <item-name>` (item selected) | Full body: all fields, nested objects, long-form text. No size cap. |

#### When it applies

- New CLI features that emit collections of items (examples: strategy guides, capability entries, tabs, network requests, media elements, cookies, frames)
- Whenever an item's full representation exceeds ~500 bytes of JSON

Rule of thumb: if an agent might call the listing just to decide which item to read next, the listing must stay cheap.

#### When it does NOT apply

- Commands that return a single object per invocation (e.g., `navigate`, `page screenshot`, `emulate status`)
- Commands where the "full" representation is already small (e.g., `tabs list` returns minimal per-tab data already)

#### Spec + test obligations for new features

New feature specs MUST include:

1. An AC that explicitly verifies the listing path returns only the lightweight fields (no full bodies) \u2014 e.g., "AC: `<cmd> list --json` output contains `name`/`title`/`summary` only; detailed fields X, Y, Z are absent."
2. An AC that verifies the detail path (`<cmd> <item-name> --json`) returns the full body.
3. A unit or BDD test that fails if a detail-only field leaks into the listing.

#### In-tree example

`agentchrome examples strategies` (feature #201) is the reference implementation:

- `agentchrome examples strategies --json` \u2192 `[{ name, title, summary }, \u2026]` \u2014 tight listing
- `agentchrome examples strategies iframes --json` \u2192 full `{ name, title, summary, scenarios, capabilities, limitations, workarounds, recommended_sequence }` \u2014 detail only when asked

### Clap Help Entries (Required for New Commands and Flags)

**Every new CLI surface MUST carry first-class `clap` help metadata.** The `capabilities` manifest, generated man pages, and shell completions all derive from clap attributes \u2014 if the clap definition is bare, every downstream documentation surface is degraded for free.

#### What each new surface must include

| Surface | Required clap attributes |
|---------|-------------------------|
| New subcommand (top-level or nested) | `#[command(about = "\u2026")]` one-line \u2022 `long_about = "\u2026"` multi-paragraph \u2022 `after_long_help = "\u2026"` with 3\u20135 worked **EXAMPLES** (including at least one `--json` invocation if the command supports it) |
| New positional argument | doc comment describing purpose, valid values, and interaction with sibling args (e.g., mutual exclusion). Optional positionals document what happens when absent. |
| New flag (`--foo`) | doc comment explaining effect and default \u2022 `conflicts_with_all = [\u2026]` when mutually exclusive with other flags \u2022 `value_parser` or `value_enum` for constrained values (no free-form strings when an enum fits) |
| New enum arg value | `#[derive(ValueEnum)]` with per-variant doc comments so they appear in `--help` and completions |
| Positional that acts as a discriminator (e.g., `examples strategies <name>`) | doc comment enumerating valid names, or a pointer to the listing command (`see \`<cmd> list\` for valid names`) |

#### Spec + test obligations for new features

New feature specs MUST include:

1. An AC that `<cmd> --help` prints a short description of the new surface and mentions its canonical invocation shape.
2. An AC that `<cmd> --help` (long form, triggered by clap `long_about` / `after_long_help`) includes at least one worked example, and, when the command emits JSON, at least one `--json` example.
3. Verification that `agentchrome capabilities` output reflects the new clap surface. The capabilities manifest is clap-driven \u2014 missing clap metadata shows up here first.
4. Verification that `cargo xtask man <cmd>` (or the aggregated `agentchrome man` flow) renders a man page with the new content. Man pages are also clap-driven.
5. When a new flag is added to an existing command: update the command's `after_long_help` examples to cover the flag with at least one realistic invocation.

#### Why this is non-negotiable

- **Shell completions** depend on clap metadata. Flags without `value_parser` / `value_enum` become free-form strings in completions.
- **Man pages** are auto-generated via `clap_mangen`. A subcommand without `long_about` produces a sparse, unhelpful man section.
- **Capabilities manifest** (a machine-readable contract for AI agents, per `/capabilities`) is clap-driven. Missing doc comments mean agents cannot discover what the surface does.
- **`examples` subcommand** also references clap for its command listing. Missing examples in `after_long_help` is a silent degradation of the discovery story.

#### In-tree example

The `Examples` variant in `src/cli/mod.rs` demonstrates the expected shape:

```rust
/// Show usage examples for commands
#[command(
    long_about = "Show usage examples for agentchrome commands\u2026 (paragraph explaining behavior)",
    after_long_help = "\
EXAMPLES:
  # List all command groups with summary examples
  agentchrome examples

  # Show detailed examples for the navigate command
  agentchrome examples navigate

  # Get all examples as JSON (for programmatic use)
  agentchrome examples --json

  # Pretty-printed JSON output
  agentchrome examples --pretty"
)]
Examples(ExamplesArgs),
```

Every new subcommand should look like this. If adding a flag or positional to an existing command, append a matching `EXAMPLES` entry.

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
2. Launch headless Chrome: `./target/debug/agentchrome connect --launch --headless`
3. Create a feature-specific test site (see Verification Gates below)
4. Exercise the feature/fix against the test site using the acceptance criteria
5. Verify each AC produces the expected output against the real browser
6. Disconnect: `./target/debug/agentchrome connect disconnect`
7. Kill any orphaned Chrome processes: `pkill -f 'chrome.*--remote-debugging' || true`

#### Requirements

- The smoke test MUST appear as a task in `tasks.md` (typically the final task before "Verify No Regressions")
- During `/verifying-specs`, execute the smoke test task and record pass/fail results in the verification report
- If the smoke test fails, treat it as a Critical finding — the implementation does not meet acceptance criteria
- For defect fixes, the smoke test MUST reproduce the exact steps from the issue's reproduction section and confirm the bug no longer occurs
- **Spec sync after smoke-test fixes**: If the smoke test reveals new findings that require code changes beyond the original spec (e.g., a deeper root cause, an additional affected code path, or a different fix mechanism), the spec documents (`requirements.md`, `design.md`, `tasks.md`, `feature.gherkin`) MUST be updated to reflect the actual implementation before completing verification. The spec is the single source of truth — it must never diverge from what was shipped.

---

## Verification Gates

Every `/verifying-specs` run MUST execute these gates. Gate results act as a ceiling on the overall verification status — a failing gate caps the status at "Partial" or lower.

| Gate | Condition | Action | Pass Criteria |
|------|-----------|--------|---------------|
| Debug Build | Always | `cargo build 2>&1` | Exit code 0 |
| Unit Tests | Always | `cargo test --lib 2>&1` | Exit code 0 |
| Clippy | Always | `cargo clippy --all-targets 2>&1` | Exit code 0 |
| Format Check | Always | `cargo fmt --check 2>&1` | Exit code 0 |
| Feature Exercise | Always | See **Feature Exercise Gate** below | Exit code 0 AND all ACs verified |

### Feature Exercise Gate

This gate replaces generic third-party site testing with a **purpose-built test site** that targets the exact behavior under verification. It ensures the implementation works end-to-end against a real Chrome instance with controlled, deterministic content.

#### Procedure

1. **Create a test site.** Write a self-contained HTML file at `tests/fixtures/<feature-name>.html` that contains the exact DOM structure needed to exercise the feature's acceptance criteria. The file must:
   - Be deterministic — no external dependencies, CDNs, or network requests
   - Include all element types, roles, and structures referenced in the ACs
   - Be minimal — only what's needed to verify the feature, not a full application
   - Include an HTML comment at the top documenting which ACs it covers

2. **Build from source.** Run `cargo build` to produce a fresh debug binary. Do NOT use a cached or pre-existing build.

3. **Launch headless Chrome.** `./target/debug/agentchrome connect --launch --headless`

4. **Navigate to the test site.** `./target/debug/agentchrome navigate file://<absolute-path-to-fixture>`

5. **Exercise each AC.** Run the agentchrome commands that correspond to each acceptance criterion and verify the output matches expectations. Record pass/fail per AC.

6. **Cleanup.** Disconnect and kill any orphaned Chrome processes.

#### Test Fixture Guidelines

- **One fixture per feature.** Each feature gets its own HTML file tailored to its ACs. Bug fixes reuse the fixture from the feature they affect, or create a minimal reproduction fixture.
- **Name matches feature.** Use the same slug as the spec directory (e.g., `tests/fixtures/compact-snapshot-mode.html` for `feature-add-compact-snapshot-mode-*`).
- **Committed to the repo.** Test fixtures are checked in alongside the feature code — they serve as living documentation of what the feature handles.
- **No JavaScript required** unless the feature explicitly tests JS-dependent behavior (e.g., SPA navigation, dynamic content). Static HTML exercises most agentchrome features.

#### Example

For a compact snapshot feature, the fixture might include:
```html
<!-- AC1: mixed interactive/decorative elements -->
<!-- AC2: nested interactive elements inside landmarks -->
<!-- AC3: enough nodes to verify 50% reduction -->
<!-- AC6: all interactive roles with UIDs -->
<html>
<body>
  <nav><a href="#">Home</a><a href="#">About</a></nav>
  <main>
    <h1>Test Page</h1>
    <div><div><div><button>Click Me</button></div></div></div>
    <form>
      <input type="text" placeholder="Name">
      <select><option>A</option><option>B</option></select>
      <input type="checkbox">
      <button type="submit">Submit</button>
    </form>
  </main>
</body>
</html>
```

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

- `steering/product.md` for product direction
- `steering/structure.md` for code organization
