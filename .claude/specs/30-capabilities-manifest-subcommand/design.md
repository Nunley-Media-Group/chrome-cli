# Design: Machine-Readable Capabilities Manifest Subcommand

**Issue**: #30
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (spec generation)

---

## Overview

This feature adds a `chrome-cli capabilities` subcommand that outputs a complete, machine-readable JSON manifest describing every command, subcommand, flag, argument, and their types. Unlike the `examples` command (which uses static data), this command walks the clap `Command` tree at runtime via `Cli::command()` — the same introspection already used by `completions` and `man` — ensuring the manifest stays automatically in sync as the CLI evolves.

The implementation adds a new `capabilities.rs` module with a visitor that traverses the clap `Command` tree, extracts metadata (names, descriptions, args, flags, types, defaults, enum values), and serializes it into a structured JSON manifest. Exit code documentation is sourced from the `ExitCode` enum. The command supports `--command <CMD>` filtering, `--compact` mode, and respects the existing `--pretty` output format flag.

---

## Architecture

### Component Diagram

```
CLI Input: chrome-cli capabilities [--command <CMD>] [--compact] [--pretty]
    ↓
┌──────────────────────────────────┐
│       CLI Layer (clap)           │ ← Parse args: --command, --compact, output format
│  src/cli/mod.rs                  │
│  Command::Capabilities(args)     │
└──────────┬───────────────────────┘
           ↓
┌──────────────────────────────────┐
│    Capabilities Module           │ ← Walk clap Command tree + format output
│  src/capabilities.rs             │
│  execute_capabilities()          │
│                                  │
│  ┌───────────────────────────┐   │
│  │  Clap Introspection       │   │
│  │  Cli::command()            │   │
│  │  get_subcommands()         │   │
│  │  get_arguments()           │   │
│  │  get_possible_values()     │   │
│  └───────────────────────────┘   │
│                                  │
│  ┌───────────────────────────┐   │
│  │  Exit Code Metadata       │   │
│  │  Static from ExitCode     │   │
│  └───────────────────────────┘   │
└──────────┬───────────────────────┘
           ↓
┌──────────────────────────────────┐
│    Output (stdout)               │ ← JSON (compact or pretty)
└──────────────────────────────────┘
```

No CDP, Chrome, or session layers are involved.

### Data Flow

```
1. User runs: chrome-cli capabilities [--command navigate] [--compact] [--pretty]
2. Clap parses into Command::Capabilities(CapabilitiesArgs { command, compact })
3. main.rs dispatches to capabilities::execute_capabilities(&global, &args)
4. execute_capabilities() calls Cli::command() to get the clap Command tree
5. Walk the command tree:
   a. Extract global flags from the root command
   b. For each subcommand: extract name, description, args, flags
   c. For nested subcommands: recurse one level deeper
   d. For each arg/flag: extract name, type, required, default, possible values
6. Build exit_codes from a static mapping of ExitCode variants
7. If --command <CMD>: filter to only the matching command (or return error)
8. If --compact: strip args, flags, and return types from output
9. Serialize to JSON (pretty if --pretty, compact otherwise)
10. Print to stdout, exit 0
```

---

## API / Interface Changes

### New CLI Subcommand

```
chrome-cli capabilities [--command <CMD>] [--compact] [--pretty]
```

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| --command | String (flag) | No | Filter output to a specific command group |
| --compact | Bool (flag) | No | Minimal output (names + descriptions only) |

Note: `--pretty` is already a global flag; no new flag needed for it.

### New Enum Variant

Added to `Command` enum in `src/cli/mod.rs`:

```rust
/// Output a machine-readable manifest of all CLI capabilities
#[command(
    long_about = "Output a complete, machine-readable JSON manifest describing every command, \
        subcommand, flag, argument, and type in the CLI. Designed for AI agents and tooling \
        that need to programmatically discover the CLI surface. The manifest is generated at \
        runtime from the clap command tree, so it is always in sync with the binary.",
    after_long_help = "\
EXAMPLES:
  # Full capabilities manifest
  chrome-cli capabilities

  # Pretty-printed for readability
  chrome-cli capabilities --pretty

  # Capabilities for a specific command
  chrome-cli capabilities --command navigate

  # Compact listing (names and descriptions only)
  chrome-cli capabilities --compact"
)]
Capabilities(CapabilitiesArgs),
```

### New Args Struct

```rust
#[derive(Args)]
pub struct CapabilitiesArgs {
    /// Show capabilities for a specific command only
    #[arg(long)]
    pub command: Option<String>,

    /// Minimal output: command names and descriptions only
    #[arg(long)]
    pub compact: bool,
}
```

### Output Schema

**Full mode (default):**

```json
{
  "name": "chrome-cli",
  "version": "0.1.0",
  "commands": [
    {
      "name": "navigate",
      "description": "URL navigation and history",
      "subcommands": [
        {
          "name": "navigate <URL>",
          "description": "Navigate to a URL",
          "args": [
            {
              "name": "url",
              "type": "string",
              "required": false,
              "description": "URL to navigate to"
            }
          ],
          "flags": [
            {
              "name": "--wait-until",
              "type": "enum",
              "values": ["load", "domcontentloaded", "networkidle", "none"],
              "default": "load",
              "description": "Wait strategy after navigation"
            },
            {
              "name": "--timeout",
              "type": "integer",
              "required": false,
              "description": "Navigation timeout in milliseconds"
            }
          ]
        },
        {
          "name": "navigate back",
          "description": "Go back in browser history",
          "args": [],
          "flags": []
        }
      ]
    }
  ],
  "global_flags": [
    {
      "name": "--port",
      "type": "integer",
      "default": 9222,
      "description": "Chrome DevTools Protocol port number"
    },
    {
      "name": "--host",
      "type": "string",
      "default": "127.0.0.1",
      "description": "Chrome DevTools Protocol host address"
    }
  ],
  "exit_codes": [
    { "code": 0, "name": "Success", "description": "Command completed successfully" },
    { "code": 1, "name": "GeneralError", "description": "Invalid arguments or internal failure" },
    { "code": 2, "name": "ConnectionError", "description": "Chrome not running or session expired" },
    { "code": 3, "name": "TargetError", "description": "Tab not found or no page targets" },
    { "code": 4, "name": "TimeoutError", "description": "Navigation or trace timeout" },
    { "code": 5, "name": "ProtocolError", "description": "CDP protocol failure" }
  ]
}
```

**Compact mode (`--compact`):**

```json
{
  "name": "chrome-cli",
  "version": "0.1.0",
  "commands": [
    { "name": "connect", "description": "Connect to or launch a Chrome instance" },
    { "name": "tabs", "description": "Tab management (list, create, close, activate)" },
    { "name": "navigate", "description": "URL navigation and history" }
  ]
}
```

**Single command mode (`--command navigate`):**

Same schema as the full mode but with only the matched command in the `commands` array, plus `global_flags` and `exit_codes` still present.

### Errors

| Code | Condition |
|------|-----------|
| ExitCode::GeneralError (1) | Unknown command name passed to `--command` |

---

## Database / Storage Changes

None. This is a purely in-memory, stateless feature.

---

## State Management

None. The capabilities module walks the clap command tree on each invocation and formats the result. No state is stored.

---

## Module Design

### `src/capabilities.rs`

```rust
// 1. Output types (Serialize structs) — the manifest schema
#[derive(Serialize)]
struct CapabilitiesManifest {
    name: String,
    version: String,
    commands: Vec<CommandDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    global_flags: Option<Vec<FlagDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_codes: Option<Vec<ExitCodeDescriptor>>,
}

#[derive(Serialize)]
struct CommandDescriptor {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subcommands: Option<Vec<SubcommandDescriptor>>,
}

#[derive(Serialize)]
struct SubcommandDescriptor {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<ArgDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<FlagDescriptor>>,
}

#[derive(Serialize)]
struct ArgDescriptor {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    required: bool,
    description: String,
}

#[derive(Serialize)]
struct FlagDescriptor {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<String>>,
    description: String,
}

#[derive(Serialize)]
struct ExitCodeDescriptor {
    code: u8,
    name: String,
    description: String,
}

// 2. Clap tree walking — the core introspection logic
fn build_manifest(cmd: &clap::Command, compact: bool) -> CapabilitiesManifest { ... }
fn visit_command(cmd: &clap::Command) -> CommandDescriptor { ... }
fn visit_subcommand(parent_name: &str, cmd: &clap::Command) -> SubcommandDescriptor { ... }
fn extract_args(cmd: &clap::Command) -> Vec<ArgDescriptor> { ... }
fn extract_flags(cmd: &clap::Command) -> Vec<FlagDescriptor> { ... }
fn infer_type(arg: &clap::Arg) -> String { ... }
fn extract_default(arg: &clap::Arg) -> Option<serde_json::Value> { ... }
fn global_flags(cmd: &clap::Command) -> Vec<FlagDescriptor> { ... }
fn exit_codes() -> Vec<ExitCodeDescriptor> { ... }

// 3. Output formatting
fn print_output(value: &impl Serialize, output: &OutputFormat) -> Result<(), AppError> { ... }

// 4. Dispatcher
pub fn execute_capabilities(global: &GlobalOpts, args: &CapabilitiesArgs) -> Result<(), AppError> { ... }
```

### Clap Introspection API Usage

The key clap `Command` methods used for tree walking:

| Method | Purpose |
|--------|---------|
| `Cli::command()` | Get the root `clap::Command` (via `CommandFactory` trait) |
| `cmd.get_name()` | Command/subcommand name |
| `cmd.get_about()` | Short description |
| `cmd.get_subcommands()` | Iterator over nested subcommands |
| `cmd.get_arguments()` | Iterator over all `Arg` entries |
| `arg.get_id()` | Argument name/ID |
| `arg.get_help()` | Help text / description |
| `arg.is_positional()` | Distinguish positional args from flags |
| `arg.is_required_set()` | Whether the arg is required |
| `arg.get_default_values()` | Default values |
| `arg.get_possible_values()` | Enum values (from `ValueEnum`) |
| `arg.get_long()` | Long flag name (e.g., "timeout") |
| `arg.get_value_names()` | Value type hints (e.g., "URL") |

### Type Inference Strategy

Since clap doesn't expose Rust types directly, we infer the displayed type from heuristics:

| Heuristic | Inferred Type |
|-----------|---------------|
| `arg.get_possible_values()` is non-empty | `"enum"` |
| `arg.get_action()` is `SetTrue` or `SetFalse` | `"bool"` |
| `arg.get_num_args()` allows multiple | `"array"` |
| Value name contains "PORT", "TIMEOUT", "QUALITY", number-like names | `"integer"` |
| Value name contains "PATH", "FILE", "DIR" | `"path"` |
| Default: everything else | `"string"` |

This is imperfect but covers the actual CLI well. The heuristics can be refined over time.

### Handling Commands Without Subcommands

Some commands (e.g., `connect`) have flat args without a subcommand enum. Others (e.g., `navigate`) mix direct args with subcommands. The visitor handles both:

- **Flat command** (connect): treated as having a single implicit subcommand with the same name
- **Nested command** (tabs, page, interact): subcommands listed directly
- **Hybrid command** (navigate): default positional args + explicit subcommands (back, forward, reload) are all listed

### Dispatcher Pattern

Follows the synchronous pattern of `execute_completions` and `execute_man`:

```rust
// In main.rs run():
Command::Capabilities(args) => capabilities::execute_capabilities(&global, args),
```

This is a sync function (no `async`), since no I/O is needed beyond stdout.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Static manifest data** | Hardcode the manifest in `capabilities.rs` (like `examples.rs`) | Full control over schema, can include return types | Manual maintenance, goes stale when commands change, duplicates clap definitions | Rejected — violates "kept in sync automatically" requirement |
| **B: Clap introspection at runtime** | Walk `Cli::command()` tree to generate manifest | Automatic sync, zero maintenance, matches what completions/man already do | Type inference is heuristic-based, can't include return type schemas | **Selected** — auto-sync is the primary requirement |
| **C: Hybrid (introspection + annotations)** | Walk clap tree + add manual annotations for return types | Best of both worlds | More complex, annotations can go stale | Considered for future enhancement |
| **D: Custom derive macro** | Write a proc-macro that generates manifest metadata at compile time | Type-safe, no runtime overhead | Significant engineering effort, macros are hard to debug | Rejected — overkill for this feature |

---

## Security Considerations

- [x] **No security impact**: Command prints metadata about the CLI itself, no user data
- [x] **Input Validation**: `--command` value validated against known command names
- [x] **No network access**: Purely local, no CDP or HTTP calls

---

## Performance Considerations

- [x] **No CDP connection**: Sub-millisecond response time
- [x] **Runtime tree walking**: Clap command tree is small (~20 commands); traversal is trivial
- [x] **Serialization**: Single JSON serialization of a small object

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Unit | #[test] in capabilities.rs | `build_manifest()` returns correct structure; all commands present |
| Unit | #[test] in capabilities.rs | `visit_command()` extracts correct metadata for known commands |
| Unit | #[test] in capabilities.rs | `infer_type()` returns expected types for known args |
| Unit | #[test] in capabilities.rs | Compact mode omits args/flags |
| Unit | #[test] in capabilities.rs | `--command` filter works for valid and invalid names |
| Integration | BDD (Gherkin) | End-to-end: run binary, validate JSON schema, check command coverage |

### Auto-Sync Verification Test

A critical test that ensures the manifest stays in sync:

```rust
#[test]
fn manifest_covers_all_commands() {
    let cmd = Cli::command();
    let manifest = build_manifest(&cmd, false);
    let expected_names: HashSet<_> = cmd.get_subcommands()
        .filter(|s| !s.is_hide_set())
        .map(|s| s.get_name().to_string())
        .collect();
    let manifest_names: HashSet<_> = manifest.commands.iter()
        .map(|c| c.name.clone())
        .collect();
    assert_eq!(expected_names, manifest_names);
}
```

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Type inference heuristics produce wrong types for some args | Medium | Low | Document heuristics, add unit tests for known args, refine over time |
| Clap API changes in future versions break introspection | Low | Medium | Pin clap version, wrap introspection in abstraction layer |
| Manifest schema is too verbose for some consumers | Low | Low | `--compact` mode provides minimal output |
| Missing return type information limits agent usefulness | Medium | Medium | Document as out-of-scope; plan hybrid approach for future |

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
