# Design: CLI Skeleton with Clap Derive Macros

**Issue**: #3
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## Overview

This feature replaces the current `main.rs` (which only prints version info) with a full clap-based CLI skeleton. The implementation introduces argument parsing via clap derive macros, 13 subcommand group stubs, global connection/output options, structured JSON error output, and exit code conventions.

The design follows the planned project structure from `structure.md`, placing CLI parsing in `src/cli/` and error types in `src/error.rs`. Each subcommand is a stub that returns a "not yet implemented" error in structured JSON format on stderr.

Since this is a CLI-only feature with no backend, CDP, or Chrome process management, the scope is limited to the CLI and Output layers.

---

## Architecture

### Component Diagram

```
CLI Input (args)
    ↓
┌─────────────────────────────────────────────────────┐
│   main.rs                                           │
│   - Create Cli struct via clap::Parser::parse()     │
│   - Match on Command enum                           │
│   - Dispatch to stub handlers                       │
│   - Handle errors via process::exit()               │
└────────────────────┬────────────────────────────────┘
                     │
          ┌──────────┼──────────┐
          ↓          ↓          ↓
┌──────────────┐ ┌────────┐ ┌───────────┐
│  cli/mod.rs  │ │error.rs│ │output (*)│
│  - Cli struct│ │- ExitCode│ │- JSON err │
│  - Command   │ │- AppError│ │  to stderr│
│  - GlobalOpts│ │         │ │           │
│  - OutputFmt │ │         │ │           │
└──────────────┘ └────────┘ └───────────┘

(*) Output formatting is minimal in this issue —
    only structured JSON error output to stderr.
    A full output layer comes in later issues.
```

### Data Flow

```
1. User runs: chrome-cli [global-opts] <subcommand>
2. clap parses args into Cli struct
3. main() matches on Cli.command
4. Stub handler returns AppError::NotImplemented
5. Error is serialized to JSON on stderr
6. Process exits with appropriate exit code
```

---

## Module Design

### `src/cli/mod.rs` — CLI Argument Parsing

The core clap structs using derive macros:

```rust
#[derive(Parser)]
#[command(
    name = "chrome-cli",
    version,
    about = "Browser automation via the Chrome DevTools Protocol",
    long_about = "...",  // Comprehensive AI-agent-friendly description
    term_width = 100,
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: Command,
}
```

#### Global Options (flattened via `#[command(flatten)]`)

```rust
#[derive(Args)]
pub struct GlobalOpts {
    // Connection options
    #[arg(long, default_value = "9222", global = true, help = "...")]
    pub port: u16,

    #[arg(long, default_value = "127.0.0.1", global = true, help = "...")]
    pub host: String,

    #[arg(long, global = true, help = "...")]
    pub ws_url: Option<String>,

    #[arg(long, global = true, help = "...")]
    pub timeout: Option<u64>,

    #[arg(long, global = true, help = "...")]
    pub tab: Option<String>,

    // Output format (mutually exclusive group)
    #[command(flatten)]
    pub output: OutputFormat,
}
```

#### Output Format (mutually exclusive group)

```rust
#[derive(Args)]
#[group(multiple = false)]
pub struct OutputFormat {
    #[arg(long, global = true, help = "Output as JSON (default)")]
    pub json: bool,

    #[arg(long, global = true, help = "Output as pretty-printed JSON")]
    pub pretty: bool,

    #[arg(long, global = true, help = "Output as human-readable plain text")]
    pub plain: bool,
}
```

Using `#[group(multiple = false)]` ensures clap rejects `--json --plain` at parse time with a clear error.

#### Command Enum

```rust
#[derive(Subcommand)]
pub enum Command {
    /// Connect to or launch a Chrome instance
    Connect,
    /// Tab management (list, create, close, activate)
    Tabs,
    /// URL navigation and history
    Navigate,
    /// Page inspection (screenshot, text, accessibility-tree, find)
    Page,
    /// DOM inspection and manipulation
    Dom,
    /// JavaScript execution in page context
    Js,
    /// Console message reading and monitoring
    Console,
    /// Network request monitoring and interception
    Network,
    /// Mouse, keyboard, and scroll interactions
    Interact,
    /// Form input and submission
    Form,
    /// Device and network emulation
    Emulate,
    /// Performance tracing and metrics
    Perf,
}
```

Each variant has a doc comment that becomes the short help text, plus a `long_about` for detailed AI-friendly descriptions.

### `src/error.rs` — Error Types and Exit Codes

```rust
#[repr(u8)]
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    ConnectionError = 2,
    TargetError = 3,
    TimeoutError = 4,
    ProtocolError = 5,
}

pub struct AppError {
    pub message: String,
    pub code: ExitCode,
}
```

The `AppError` type serializes to JSON for stderr output: `{"error": "message", "code": N}`.

### `src/main.rs` — Entry Point

```rust
fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        e.print_json_stderr();
        std::process::exit(e.code as i32);
    }
}

fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Command::Connect => Err(AppError::not_implemented("connect")),
        Command::Tabs => Err(AppError::not_implemented("tabs")),
        // ... all 13 stubs
    }
}
```

No async runtime is needed for this issue — all stubs are synchronous.

---

## File Layout

| File | Purpose | Type |
|------|---------|------|
| `src/main.rs` | Entry point, dispatch | Modify |
| `src/cli/mod.rs` | Cli, GlobalOpts, OutputFormat, Command | Create |
| `src/error.rs` | ExitCode, AppError, JSON error output | Create |
| `Cargo.toml` | Add clap, serde, serde_json dependencies | Modify |

---

## Dependency Changes

### New Dependencies (in `[dependencies]`)

| Crate | Version | Features | Purpose |
|-------|---------|----------|---------|
| `clap` | 4 | `derive`, `env` | CLI argument parsing |
| `serde` | 1 | `derive` | JSON serialization |
| `serde_json` | 1 | — | JSON error output |

Note: `serde` is already in `[dev-dependencies]` with `derive`. Moving it to `[dependencies]` (keeping it also in dev-deps is fine — Cargo deduplicates).

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Single file** | All CLI structs in main.rs | Simple, no modules | Gets unwieldy fast, doesn't follow structure.md | Rejected |
| **B: cli/ module** | Separate CLI module per structure.md | Clean separation, follows planned structure, easy to extend | One extra module | **Selected** |
| **C: cli/commands/ submodules** | One file per command group | Maximum modularity | Over-engineering for stubs; better when commands have real logic | Deferred to future issues |
| **D: clap builder API** | Use clap's programmatic builder | More control over help formatting | More verbose, harder to maintain, less idiomatic | Rejected |
| **E: Separate output module** | Full output layer now | Consistent with structure.md | Over-scoping; only error output needed now | Deferred |

---

## Security Considerations

- [x] **Input Validation**: clap handles type validation for --port (u16), --timeout (u64)
- [x] **No secrets**: No sensitive data handled in CLI parsing
- [x] **No network**: Stubs don't make any connections
- [x] **No file I/O**: Stubs don't read/write files

---

## Performance Considerations

- [x] **Startup time**: clap derive adds minimal overhead (< 1ms parsing)
- [x] **Binary size**: clap with derive adds ~1-2MB; well within 10MB target
- [x] **No async**: No runtime overhead — synchronous stubs

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI parsing | Unit tests (in cli/mod.rs) | Verify argument parsing, defaults, conflicts |
| Error output | Unit tests (in error.rs) | Verify JSON serialization, exit codes |
| Integration | BDD (cucumber-rs) | End-to-end CLI invocation tests |
| Feature | BDD (Gherkin) | All 13 acceptance criteria as scenarios |

BDD tests will invoke the compiled binary via `std::process::Command` and assert on stdout, stderr, and exit codes.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| clap 4 API changes | Low | Low | Pin to `4` major version |
| Help text formatting varies by terminal width | Low | Low | Set `term_width = 100` in clap settings |
| Output format flag defaults need future adjustment | Medium | Low | OutputFormat struct is easy to modify; --json default can be handled in run() logic |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All interface changes documented
- [x] No database/storage changes (N/A)
- [x] No state management needed (N/A — synchronous CLI)
- [x] No UI components (CLI tool)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
