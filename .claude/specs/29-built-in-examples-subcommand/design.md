# Design: Built-in Examples Subcommand

**Issue**: #29
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (spec generation)

---

## Overview

This feature adds a `chrome-cli examples` subcommand that prints usage examples for each command group. The command is a "meta" command (like `completions` and `man`) that requires no Chrome/CDP connection. It serves static, embedded example data in plain text or JSON format.

The implementation follows the established patterns in the codebase: a new `Examples` variant in the `Command` enum, a new `examples.rs` module with static example data, and output formatting via the existing `OutputFormat` mechanism (--json, --pretty, --plain).

---

## Architecture

### Component Diagram

```
CLI Input: chrome-cli examples [command] [--json|--pretty|--plain]
    ↓
┌──────────────────────────────┐
│       CLI Layer (clap)       │ ← Parse args: optional command name, output format
│  src/cli/mod.rs              │
│  Command::Examples(args)     │
└──────────┬───────────────────┘
           ↓
┌──────────────────────────────┐
│    Examples Module            │ ← Static example data + formatting
│  src/examples.rs             │
│  execute_examples()          │
└──────────┬───────────────────┘
           ↓
┌──────────────────────────────┐
│    Output (stdout)           │ ← Plain text or JSON
└──────────────────────────────┘
```

No CDP, Chrome, or session layers are involved.

### Data Flow

```
1. User runs: chrome-cli examples [navigate] [--json]
2. Clap parses into Command::Examples(ExamplesArgs { command: Option<String> })
3. main.rs dispatches to examples::execute_examples(&global, &args)
4. execute_examples() looks up example data:
   a. If no command arg: return all command groups (summary view)
   b. If command arg given: find matching group or return error
5. Format output based on OutputFormat flags:
   a. --plain (or default): human-readable text with # comment descriptions
   b. --json: compact JSON
   c. --pretty: pretty-printed JSON
6. Print to stdout, exit 0
```

---

## API / Interface Changes

### New CLI Subcommand

```
chrome-cli examples [COMMAND] [--json|--pretty|--plain]
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| COMMAND | positional String | No | Command group name to show examples for |

### New Enum Variant

Added to `Command` enum in `src/cli/mod.rs`:

```rust
/// Show usage examples for commands
#[command(
    long_about = "Show usage examples for chrome-cli commands. Without arguments, lists all \
        command groups with a brief description and one example each. With a command name, \
        shows detailed examples for that specific command group.",
    after_long_help = "\
EXAMPLES:
  # List all command groups with summary examples
  chrome-cli examples

  # Show detailed examples for the navigate command
  chrome-cli examples navigate

  # Get all examples as JSON (for programmatic use)
  chrome-cli examples --json

  # Pretty-printed JSON output
  chrome-cli examples --pretty"
)]
Examples(ExamplesArgs),
```

### New Args Struct

```rust
#[derive(Args)]
pub struct ExamplesArgs {
    /// Command group to show examples for (e.g., navigate, tabs, page)
    pub command: Option<String>,
}
```

### Output Schemas

**Summary mode (no command arg) — JSON:**

```json
[
  {
    "command": "connect",
    "description": "Connect to or launch a Chrome instance",
    "examples": [
      {
        "cmd": "chrome-cli connect",
        "description": "Connect to Chrome on the default port"
      },
      {
        "cmd": "chrome-cli connect --launch --headless",
        "description": "Launch a new headless Chrome instance"
      }
    ]
  }
]
```

**Detail mode (with command arg) — JSON:**

```json
{
  "command": "navigate",
  "description": "URL navigation and history",
  "examples": [
    {
      "cmd": "chrome-cli navigate https://example.com --wait-until load",
      "description": "Navigate to a URL and wait for load",
      "flags": ["--wait-until"]
    },
    {
      "cmd": "chrome-cli navigate https://app.example.com --wait-until networkidle",
      "description": "Navigate and wait for network idle (for SPAs)",
      "flags": ["--wait-until"]
    }
  ]
}
```

**Plain text (summary mode):**

```
connect — Connect to or launch a Chrome instance
  chrome-cli connect

tabs — Tab management (list, create, close, activate)
  chrome-cli tabs list

navigate — URL navigation and history
  chrome-cli navigate https://example.com
...
```

**Plain text (detail mode):**

```
navigate — URL navigation and history

  # Navigate to a URL and wait for load
  chrome-cli navigate https://example.com --wait-until load

  # Navigate and wait for network idle (for SPAs)
  chrome-cli navigate https://app.example.com --wait-until networkidle

  # Go back in history
  chrome-cli navigate back

  # Reload without cache
  chrome-cli navigate reload --ignore-cache
```

### Errors

| Code | Condition |
|------|-----------|
| ExitCode::GeneralError (1) | Unknown command group name |

---

## Database / Storage Changes

None. This is a purely static, in-memory feature.

---

## State Management

None. The examples module contains only static data and pure formatting functions.

---

## Module Design

### `src/examples.rs`

```rust
// 1. Output types (Serialize structs)
#[derive(Serialize)]
struct CommandGroupSummary {
    command: String,
    description: String,
    examples: Vec<ExampleEntry>,
}

#[derive(Serialize)]
struct ExampleEntry {
    cmd: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<String>>,
}

// 2. Static example data — one function returning Vec<CommandGroupSummary>
fn all_examples() -> Vec<CommandGroupSummary> { ... }

// 3. Output formatting (print_output, format_plain_summary, format_plain_detail)
fn print_output(value: &impl Serialize, output: &OutputFormat) -> Result<(), AppError> { ... }
fn format_plain_summary(groups: &[CommandGroupSummary]) -> String { ... }
fn format_plain_detail(group: &CommandGroupSummary) -> String { ... }

// 4. Dispatcher
pub fn execute_examples(global: &GlobalOpts, args: &ExamplesArgs) -> Result<(), AppError> { ... }
```

### Dispatcher Pattern

Follows the same synchronous pattern as `execute_completions` and `execute_man`:

```rust
// In main.rs run():
Command::Examples(args) => examples::execute_examples(&global, args),
```

This is a sync function (no `async`), since no I/O is needed.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Extract from clap help text** | Parse `after_long_help` at runtime | Single source of truth, no duplication | Fragile parsing, format limitations, can't add structured fields like `flags` | Rejected — brittle |
| **B: Static embedded data** | Hardcode examples in `examples.rs` | Fast, simple, fully structured, easy to test | Duplication with clap help text | **Selected** — simplicity and structure |
| **C: External data file** | Load examples from JSON/TOML at runtime | Easy to edit | Runtime file I/O, deployment complexity, single-binary principle violated | Rejected — adds file dependency |

---

## Security Considerations

- [x] **No security impact**: Command prints static data, no user input affects behavior beyond command group lookup
- [x] **Input Validation**: Command name is validated against known group names

---

## Performance Considerations

- [x] **No CDP connection**: Sub-millisecond response time
- [x] **Static data**: No I/O, no allocation beyond formatting

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Unit | #[test] in examples.rs | all_examples() returns expected groups, correct count |
| Unit | #[test] in examples.rs | format_plain_summary(), format_plain_detail() produce expected output |
| Integration | BDD (Gherkin) | End-to-end: run binary, verify output format and content |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Examples become stale when commands change | Medium | Low | Test that example command names match Command enum variants |
| Duplication between help text and examples | Low | Low | Accept as trade-off; examples module is the canonical structured source |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] No state management needed
- [x] No UI components needed (CLI only)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
