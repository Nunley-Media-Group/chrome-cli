# Design: Man Page Generation

**Issues**: #27, #232
**Date**: 2026-04-22
**Status**: Draft
**Author**: Claude (writing-specs)

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #27 | 2026-02-14 | Initial design — clap_mangen runtime + xtask generation |
| #232 | 2026-04-22 | Added xtask-time enrichment pass that injects capabilities-manifest entries and all examples content into each generated man page, with the `examples` subcommand's data as the single source of truth |

---

## Overview

This feature adds man page generation for agentchrome using the `clap_mangen` crate. It has two delivery mechanisms:

1. **Runtime inline display** — A `agentchrome man [COMMAND]` subcommand that generates and renders man pages to stdout at runtime, for systems without `man` installed or for quick reference.

2. **Build-time file generation** — A `cargo xtask man` command that generates `.1` man page files to the `man/` directory for packaging in release archives and system installation.

Both mechanisms use `clap_mangen::Man::new()` with `Cli::command()` to introspect the clap definition, exactly paralleling how `agentchrome completions` uses `clap_complete::generate()`. The comprehensive help text added in #26 flows directly into the man pages' DESCRIPTION and OPTIONS sections.

---

## Architecture

### Component Diagram

```
Runtime path: `agentchrome man [connect]`
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

### Data Flow — Runtime (`agentchrome man connect`)

```
1. User runs `agentchrome man connect`
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
5. Writes to man/agentchrome.1, man/agentchrome-connect.1, etc.
6. Prints summary of generated files
```

---

## API / Interface Changes

### New CLI Subcommand

| Subcommand | Argument | Type | Purpose |
|------------|----------|------|---------|
| `man` | `[COMMAND]` | Optional string (positional) | Display the man page for agentchrome or a subcommand |

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
agentchrome man [COMMAND]

Display man pages for agentchrome commands.

Without arguments, displays the main agentchrome man page. With a subcommand
name, displays the man page for that specific command.

EXAMPLES:
  # Display the main agentchrome man page
  agentchrome man

  # Display the man page for the connect command
  agentchrome man connect

  # Display the man page for the tabs command
  agentchrome man tabs

  # Pipe to a pager
  agentchrome man navigate | less
```

### New xtask Command

```
cargo xtask man

Generates man pages for all agentchrome commands and writes them to man/.
```

### Generated Man Page File Naming

| Command | Man Page File |
|---------|---------------|
| `agentchrome` | `man/agentchrome.1` |
| `agentchrome connect` | `man/agentchrome-connect.1` |
| `agentchrome tabs` | `man/agentchrome-tabs.1` |
| `agentchrome tabs list` | `man/agentchrome-tabs-list.1` |

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
agentchrome/
├── xtask/                    # NEW: workspace member for dev tasks
│   ├── Cargo.toml            # Dependencies: clap, clap_mangen, agentchrome (path)
│   └── src/
│       └── main.rs           # xtask entry point with `man` subcommand
├── man/                      # NEW: generated man page output directory
│   ├── agentchrome.1
│   ├── agentchrome-connect.1
│   ├── agentchrome-tabs.1
│   ├── agentchrome-tabs-list.1
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
| **A: Runtime-only (`agentchrome man`)** | Generate man pages only at runtime | Simplest implementation, no generated files to manage | Can't ship pre-built man pages in releases; no `man agentchrome` integration | Rejected — doesn't satisfy distribution requirement |
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
| Top-level man page | Integration (BDD) | `agentchrome man` outputs non-empty man page with exit code 0 |
| Subcommand man pages | Integration (BDD) | `agentchrome man <cmd>` for each subcommand |
| Content validation | Integration (BDD) | Output contains command name and standard man page sections |
| Error case | Integration (BDD) | Invalid subcommand name produces error |
| Help text | Integration (BDD) | `agentchrome man --help` shows usage information |

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

<!-- Added by issue #232 -->

## Addendum — Issue #232: Enriched Man Page Content

### Overview

Issue #27 established clap_mangen generation from `long_about` / `after_long_help` attributes. In practice this produced thin man pages — `agentchrome man dialog` on 1.33.1 showed only four example lines, while `agentchrome examples dialog` and `agentchrome capabilities` already carried much richer, maintained content. This addendum describes how `cargo xtask man` enriches each rendered page at build time by pulling the structured content from the existing `capabilities` and `examples` subcommands so there is exactly one source of truth per piece of content.

The enrichment happens entirely inside the xtask binary. The runtime path (`agentchrome man <cmd>`) remains unchanged in shape: it still reads the generated man file (or renders from the in-memory clap tree) and performs no extra I/O. FR5 / FR7 and AC3 / AC5 from #27 are preserved verbatim.

### Architecture — Enrichment Pipeline

```
cargo xtask man
    ↓
┌──────────────────────────┐
│ 1. Build clap Command     │  agentchrome::command()
└────────────┬─────────────┘
             ↓
┌──────────────────────────┐
│ 2. Pull enrichment data   │  agentchrome::capabilities::build_manifest()
│                           │  agentchrome::examples::all_examples()
└────────────┬─────────────┘
             ↓
┌──────────────────────────┐
│ 3. Per-command enricher   │  For each (top-level + nested) clap subcommand:
│                           │    - look up its CommandDescriptor in the manifest
│                           │    - look up its CommandGroupSummary in examples
│                           │    - emit EXAMPLES + CAPABILITIES as roff sections
└────────────┬─────────────┘
             ↓
┌──────────────────────────┐
│ 4. Render + append        │  clap_mangen::Man::render(...)  →  in-memory buf
│                           │  append enrichment roff         →  buf
└────────────┬─────────────┘
             ↓
        man/*.1 (byte-deterministic)
```

### Data Sourcing

| Concern | Source of Truth | Consumed by man-generation via |
|---------|-----------------|--------------------------------|
| Purpose / inputs / outputs / exit codes | `src/capabilities.rs` (`CommandDescriptor`, `SubcommandDescriptor`, `ExitCodeDescriptor`) | Library-level accessor: `agentchrome::capabilities::build_manifest()` (add if not already pub) |
| Example commands and descriptions | `src/examples/commands.rs` (`CommandGroupSummary` / `ExampleEntry`) | Library-level accessor: `agentchrome::examples::all_examples()` (add if not already pub) |
| Cross-process dialog flow (after #225) | A new multi-step entry in `src/examples/commands.rs` for the `dialog` group | Automatically picked up because the man generator iterates all example entries |

`after_long_help` strings are no longer the canonical home for EXAMPLES content; they may be trimmed to a one-line pointer ("See EXAMPLES section.") in a follow-up pass but that cleanup is **not** required to close #232 — FR10's "single source of truth" constraint is about the *generated man page*, not about removing `after_long_help` entirely.

### Interface Changes (Library-Facing)

Two crate-level re-exports may need to be added so `xtask/src/main.rs` can read the structured data:

| New / Widened API | Purpose |
|--------------------|---------|
| `pub fn agentchrome::capabilities::build_manifest() -> CapabilitiesManifest` (or equivalent already-pub accessor) | xtask-side manifest read |
| `pub fn agentchrome::examples::all_examples() -> Vec<CommandGroupSummary>` | xtask-side examples read |

If these accessors already exist as `pub(crate)`, widen to `pub` via a thin re-export in `src/lib.rs` rather than reshaping the modules.

### Enriched Man Page Layout (per command)

```
NAME
    agentchrome-dialog - Handle browser dialogs ...

SYNOPSIS        (from clap_mangen)
DESCRIPTION     (from clap_mangen — long_about)
OPTIONS         (from clap_mangen)

CAPABILITIES    (appended by xtask enricher — issue #232)
    Purpose:    <from CommandDescriptor.description>
    Inputs:     <from SubcommandDescriptor.args>
    Flags:      <from SubcommandDescriptor.flags>
    Exit codes: <from ExitCodeDescriptor list>

EXAMPLES        (appended by xtask enricher — issue #232)
    <every ExampleEntry in all_examples()[group=cmd].examples>
```

The CAPABILITIES header is a non-standard man section; this is acceptable for tool-specific man pages and mirrors how many Unix tools add custom sections (`git` uses `CONFIGURATION`, `docker` uses `REPORTING BUGS`, etc.).

### Determinism (AC14)

To guarantee byte-identical output across runs:

1. **Sort order.** Iterate `CapabilitiesManifest.commands` and `all_examples()` in their declared `Vec` order (already deterministic — they are static builders, not `HashMap` walks). Any ad-hoc lookup must be done via `.iter().find(...)` to avoid hash-order non-determinism.
2. **No timestamps.** Do not stamp the generated file with a build time. `clap_mangen` does include a date field in its header derived from `cmd.get_version()` metadata; verify the default path is date-free or explicitly pass `Man::date("")` / equivalent.
3. **Stable paths.** File paths emitted in logs (`println!`) do not land inside the `.1` file; they are stdout-only.
4. **CI verification.** Add a check: `cargo xtask man && git diff --exit-code man/` to assert no uncommitted drift. (This CI hook is the concrete mitigation for AC14 and is added as a task in Phase 3 below.)

### Runtime Impact (AC15)

The runtime `agentchrome man` subcommand is unchanged in code path and allocations. The enrichment is purely a build-time string-append inside xtask. The runtime path's two configurations:

- **If** `agentchrome man <cmd>` in #27 reads the shipped `.1` file → it is larger, but still one `read()` syscall; well under 50 ms.
- **If** `agentchrome man <cmd>` in #27 renders from the in-memory clap tree → the enrichment content is not visible via that path. This is a **known limitation** called out in Open Questions below. The two acceptable resolutions are (a) also run the enrichment in `execute_man()` at runtime (same accessors, same output — cost is a few hundred microseconds of string building), or (b) switch the runtime path to read the packaged `.1` file.

### Alternatives Considered (Issue #232 additions)

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **D: Duplicate content into `after_long_help`** | Copy capabilities/examples text into each subcommand's `after_long_help` doc string | Zero changes to xtask | Two sources of truth, guaranteed drift (the exact problem #232 is fixing) | Rejected |
| **E: Generate `.md` from data, then convert to roff via `pandoc`** | Emit Markdown from capabilities/examples, pipe through pandoc | Very flexible formatting | Adds a pandoc build dependency; two-step pipeline; determinism harder to guarantee across pandoc versions | Rejected |
| **F: Build-time xtask enrichment with xtask-side roff append (this design)** | xtask reads structured data and appends roff sections to each rendered man page | Single source of truth; no new build deps; deterministic | Adds ~100 LOC of roff-emission code in xtask | **Selected** |

### Risks & Mitigations (Issue #232 additions)

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Runtime `agentchrome man` path diverges from file-based path (no enrichment visible at runtime) | Medium | Medium | Resolve Open Question §O1 below in Phase 3; chosen approach is covered by a task |
| `clap_mangen` version emits a build date into the man header, breaking AC14 | Low | Medium | Explicitly set `Man::date("")` or equivalent; CI `git diff --exit-code man/` catches regressions |
| Dialog cross-process example (AC13) lands before #225 ships, leaving a broken example | Low | High | Gate the dialog cross-process `ExampleEntry` behind the PR that closes #225; don't merge the example until #225 is green |
| `HashMap`-backed capabilities lookup introduces non-deterministic ordering | Low | High | Iterate the source `Vec`s in declared order; code review to reject any hash-map-based enrichment lookup |

### Open Questions (Issue #232)

- **O1.** Should runtime `agentchrome man <cmd>` also emit the enrichment content, or should it read the packaged `.1` file? The spec's AC15 allows either path. Default proposal: runtime calls the same enrichment helpers (shared library code) so there is exactly one enrichment emitter and the runtime and build-time outputs match. Confirm during Phase 3 implementation planning.

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
