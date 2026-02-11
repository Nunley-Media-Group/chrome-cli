# chrome-cli Technical Steering

This document defines the technology stack, constraints, and integration standards.
All technical decisions should align with these guidelines.

---

## Architecture Overview

```
CLI (clap/args parsing)
    ↓
Command Dispatcher
    ↓
CDP Client (WebSocket)
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
| Linting | Clippy | all=deny, pedantic=warn |
| Formatting | rustfmt | edition 2024 |

### External Services

| Service | Purpose | Notes |
|---------|---------|-------|
| Chrome/Chromium | Browser target | Connected via CDP over WebSocket |

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

---

## API / Interface Standards

### CLI Command Structure

```
chrome-cli <command> [subcommand] [options] [arguments]

# Examples:
chrome-cli navigate <url>
chrome-cli screenshot <url> --output <file>
chrome-cli eval <javascript>
chrome-cli tabs list
```

### Output Format

```
# Default: human-readable plain text
Example Domain

# With --json flag: structured JSON
{"title": "Example Domain", "url": "https://example.com"}

# Errors to stderr, data to stdout
# Exit code 0 for success, non-zero for failure
```

---

## Testing Standards

### BDD Testing (Required for nmg-sdlc)

**Every acceptance criterion MUST have a Gherkin test.**

<!-- TODO: Set up a BDD framework. Options for Rust:
     - cucumber-rs (https://github.com/cucumber-rs/cucumber)
     - Custom Gherkin parser with built-in test framework
-->

| Layer | Framework | Location |
|-------|-----------|----------|
| BDD/Acceptance | cucumber-rs (recommended) | tests/features/*.feature |

### Gherkin Feature Files

```gherkin
# tests/features/navigate.feature
Feature: Navigate to URL
  As a developer
  I want to navigate Chrome to a URL via CLI
  So that I can automate browser tasks from scripts

  Scenario: Navigate to a valid URL
    Given Chrome is running with CDP enabled
    When I run "chrome-cli navigate https://example.com"
    Then the page URL should be "https://example.com"
    And the exit code should be 0
```

### Step Definitions

```rust
// Rust step definition pattern (cucumber-rs)
// Path: tests/steps/

use cucumber::{given, when, then, World};

#[given("Chrome is running with CDP enabled")]
async fn chrome_running(world: &mut CliWorld) {
    // Setup Chrome instance
}

#[when(expr = "I run {string}")]
async fn run_command(world: &mut CliWorld, command: String) {
    // Execute CLI command
}

#[then(expr = "the exit code should be {int}")]
async fn check_exit_code(world: &mut CliWorld, code: i32) {
    assert_eq!(world.exit_code, code);
}
```

### Unit Tests

| Type | Framework | Location | Run Command |
|------|-----------|----------|-------------|
| Unit | built-in (#[test]) | src/**/*.rs (inline) | `cargo test --lib` |
| Integration | built-in (#[test]) | tests/*.rs | `cargo test --test '*'` |
| BDD | cucumber-rs | tests/features/*.feature | `cargo test --test bdd` |

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

## Environment Variables

### Optional

| Variable | Description |
|----------|-------------|
| `CHROME_PATH` | Path to Chrome/Chromium binary (auto-detected if not set) |
| `CHROME_PORT` | CDP debugging port (default: auto-assigned) |
| `NO_COLOR` | Disable colored output (standard convention) |

---

## References

- CLAUDE.md for project overview
- `.claude/steering/product.md` for product direction
- `.claude/steering/structure.md` for code organization
