# Design: Man Page Generation

**Issue**: #27
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This feature adds man page generation for chrome-cli using the `clap_mangen` crate. It has two delivery mechanisms:

1. **Runtime inline display** — A `chrome-cli man [COMMAND]` subcommand that generates and renders man pages to stdout at runtime, for systems without `man` installed or for quick reference.

2. **Build-time file generation** — A `cargo xtask man` command that generates `.1` man page files to the `man/` directory for packaging in release archives and system installation.

Both mechanisms use `clap_mangen::Man::new()` with `Cli::command()` to introspect the clap definition, exactly paralleling how `chrome-cli completions` uses `clap_complete::generate()`. The comprehensive help text added in #26 flows directly into the man pages' DESCRIPTION and OPTIONS sections.

---

## Architecture

### Component Diagram

```
Runtime path: `chrome-cli man [connect]`
    ↓
┌─────────────────┐
│   CLI Layer      │ ← Parse args, match Command::Man(ManArgs)
└────────┬────────┘
         ↓
┌─────────────────┐
│  Man Handler     │ ← Call Cli::command(), find subcommand,
│  (src/main.rs)   │   render via clap_mangen::Man::render()
└────────┬────────┘
         ↓
   stdout (roff-format man page)


Build-time path: `cargo xtask man`
    ↓
┌─────────────────┐
│  xtask binary    │ ← Separate binary in xtask/ workspace member
└────────┬────────┘
         ↓
┌─────────────────┐
│  Man Generator   │ ← Walk Cli::command() tree, generate all man pages
│  (xtask/src/     │   via clap_mangen::Man::render()
│   main.rs)       │
└────────┬────────┘
         ↓
   man/*.1 files (written to disk)
```

No CDP connection, Chrome process, or async runtime is needed. Both paths are entirely local and synchronous.

### Data Flow — Runtime (`chrome-cli man connect`)

```
1. User runs `chrome-cli man connect`
2. Clap parses args → Command::Man(ManArgs { command: Some("connect") })
3. main.rs matches Command::Man, calls execute_man()
4. execute_man() calls Cli::command() to get clap Command builder
5. Finds the "connect" subcommand in the command tree
6. clap_mangen::Man::new(subcommand).render(&mut stdout)
7. Man page content written to stdout
8. Exit code 0
```

### Data Flow — Build-time (`cargo xtask man`)

```
1. Developer runs `cargo xtask man`
2. xtask binary calls Cli::command() to get clap Command builder
3. Iterates over all subcommands recursively
4. For each command, generates man page via clap_mangen::Man::new(cmd)
5. Writes to man/chrome-cli.1, man/chrome-cli-connect.1, etc.
6. Prints summary of generated files
```

---

## API / Interface Changes

### New CLI Subcommand

| Subcommand | Argument | Type | Purpose |
|------------|----------|------|---------|
| `man` | `[COMMAND]` | Optional string (positional) | Display the man page for chrome-cli or a subcommand |

### ManArgs Struct

```rust
#[derive(Args)]
pub struct ManArgs {
    /// Subcommand to display man page for (omit for top-level)
    pub command: Option<String>,
}
```

### Command Help Text

```
chrome-cli man [COMMAND]

Display man pages for chrome-cli commands.

Without arguments, displays the main chrome-cli man page. With a subcommand
name, displays the man page for that specific command.

EXAMPLES:
  # Display the main chrome-cli man page
  chrome-cli man

  # Display the man page for the connect command
  chrome-cli man connect

  # Display the man page for the tabs command
  chrome-cli man tabs

  # Pipe to a pager
  chrome-cli man navigate | less
```

### New xtask Command

```
cargo xtask man

Generates man pages for all chrome-cli commands and writes them to man/.
```

### Generated Man Page File Naming

| Command | Man Page File |
|---------|---------------|
| `chrome-cli` | `man/chrome-cli.1` |
| `chrome-cli connect` | `man/chrome-cli-connect.1` |
| `chrome-cli tabs` | `man/chrome-cli-tabs.1` |
| `chrome-cli tabs list` | `man/chrome-cli-tabs-list.1` |

---

## Database / Storage Changes

None — this feature has no persistence or state.

---

## State Management

None — man page generation is a pure function with no state transitions.

---

## UI Components

None — CLI-only, stdout output.

---

## Project Structure Changes

### New Files

```
chrome-cli/
├── xtask/                    # NEW: workspace member for dev tasks
│   ├── Cargo.toml            # Dependencies: clap, clap_mangen, chrome-cli (path)
│   └── src/
│       └── main.rs           # xtask entry point with `man` subcommand
├── man/                      # NEW: generated man page output directory
│   ├── chrome-cli.1
│   ├── chrome-cli-connect.1
│   ├── chrome-cli-tabs.1
│   ├── chrome-cli-tabs-list.1
│   └── ...                   # One .1 file per command/subcommand
└── .cargo/
    └── config.toml           # NEW: alias `cargo xtask` → `cargo run -p xtask --`
```

### Modified Files

| File | Change |
|------|--------|
| `Cargo.toml` | Add `clap_mangen` dependency; add `xtask` to workspace members |
| `src/cli/mod.rs` | Add `Command::Man(ManArgs)` variant with help text |
| `src/main.rs` | Add `execute_man()` handler |
| `.gitignore` | Add `man/` (generated files, not tracked) |

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Runtime-only (`chrome-cli man`)** | Generate man pages only at runtime | Simplest implementation, no generated files to manage | Can't ship pre-built man pages in releases; no `man chrome-cli` integration | Rejected — doesn't satisfy distribution requirement |
| **B: build.rs generation** | Generate man pages during `cargo build` in build.rs | Automatic, always fresh | Adds build-time dependency; complicates build; generated files end up in OUT_DIR not easily packageable | Rejected — xtask is cleaner for artifact generation |
| **C: xtask + runtime hybrid** | xtask for file generation, runtime subcommand for inline display | Best of both worlds: distributable files + instant access | Two implementations (but share same clap_mangen core) | **Selected** |

---

## Security Considerations

- [x] **No authentication**: Man page generation requires no Chrome connection or privileges
- [x] **No input validation risk**: Command argument is validated against known subcommand names
- [x] **Output is inert**: Man pages are documentation text, not executable

---

## Performance Considerations

- [x] **Instant execution**: `clap_mangen::Man::render()` is pure in-memory string generation — sub-millisecond
- [x] **No network**: No CDP connection, no Chrome process needed
- [x] **No async**: Can run synchronously; no tokio runtime needed for this path
- [x] **xtask is dev-only**: `cargo xtask man` is a developer/CI tool, not user-facing

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI parsing | Unit | `Command::Man` variant parsed correctly |
| Top-level man page | Integration (BDD) | `chrome-cli man` outputs non-empty man page with exit code 0 |
| Subcommand man pages | Integration (BDD) | `chrome-cli man <cmd>` for each subcommand |
| Content validation | Integration (BDD) | Output contains command name and standard man page sections |
| Error case | Integration (BDD) | Invalid subcommand name produces error |
| Help text | Integration (BDD) | `chrome-cli man --help` shows usage information |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `clap_mangen` version incompatible with clap 4 | Low | High | Pin compatible version; clap_mangen is maintained by the clap team |
| xtask workspace setup complexity | Low | Low | Well-documented pattern in the Rust ecosystem |
| Generated man pages become stale | Low | Medium | CI step to verify xtask generates without errors; man/ is gitignored |

---

## Open Questions

- None

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] No state management needed
- [x] No UI components needed
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
