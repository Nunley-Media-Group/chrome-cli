# Design: Shell Completions Generation

**Issue**: #25
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This feature adds a `completions` subcommand to chrome-cli that generates shell completion scripts for bash, zsh, fish, powershell, and elvish. It uses the `clap_complete` crate which introspects the existing `Cli` parser definition to produce accurate completions automatically — no manual maintenance as commands evolve.

The implementation is minimal: add a new `Command::Completions` variant, a `Shell` enum argument, and a handler that calls `clap_complete::generate()` with the shell and the `Cli::command()` builder. The completion script is written to stdout so users can redirect it to the appropriate shell-specific location.

---

## Architecture

### Component Diagram

```
CLI Input: `chrome-cli completions bash`
    ↓
┌─────────────────┐
│   CLI Layer      │ ← Parse args, match Command::Completions(shell)
└────────┬────────┘
         ↓
┌─────────────────┐
│  Completions     │ ← Call clap_complete::generate() with Cli::command()
│  Handler         │   and write to stdout
└────────┬────────┘
         ↓
   stdout (completion script)
```

No CDP connection, Chrome process, or async runtime is needed. This command is entirely local and synchronous.

### Data Flow

```
1. User runs `chrome-cli completions <shell>`
2. Clap parses args → Command::Completions(CompletionsArgs { shell })
3. main.rs matches Command::Completions, calls execute_completions()
4. execute_completions() calls Cli::command() to get the clap Command builder
5. clap_complete::generate(shell, &mut cmd, "chrome-cli", &mut stdout)
6. Completion script is written to stdout
7. Exit code 0
```

---

## API / Interface Changes

### New CLI Subcommand

| Subcommand | Argument | Type | Purpose |
|------------|----------|------|---------|
| `completions` | `<SHELL>` | Shell enum (positional, required) | Generate completion script for the given shell |

### Shell Enum

Uses `clap_complete::Shell` which implements `clap::ValueEnum` and supports: `bash`, `zsh`, `fish`, `powershell`, `elvish`.

### Command Help Text

The `completions` subcommand help will include per-shell installation instructions in the `long_about`:

```
chrome-cli completions <SHELL>

Generate shell completion scripts.

INSTALLATION:
  bash:       chrome-cli completions bash > /etc/bash_completion.d/chrome-cli
  zsh:        chrome-cli completions zsh > ~/.zfunc/_chrome-cli
  fish:       chrome-cli completions fish > ~/.config/fish/completions/chrome-cli.fish
  powershell: chrome-cli completions powershell >> $PROFILE
  elvish:     chrome-cli completions elvish >> ~/.elvish/rc.elv
```

---

## Database / Storage Changes

None — this feature has no persistence or state.

---

## State Management

None — the completion generation is a pure function with no state transitions.

---

## UI Components

None — CLI-only, stdout output.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: `clap_complete::Shell` directly** | Use clap_complete's built-in `Shell` enum as the `ValueEnum` argument | Zero boilerplate, automatically supports all shells clap_complete supports, `Shell` already implements `ValueEnum` | Adds `clap_complete` as a runtime dependency (though it's tiny) | **Selected** |
| **B: Custom Shell enum + match** | Define our own `Shell` enum and map it to `clap_complete::Shell` | More control over shell names | Unnecessary indirection, must be kept in sync | Rejected — adds maintenance burden with no benefit |
| **C: Build-time generation** | Generate completions at `cargo build` via build.rs | Completions available as files in the repo | Stale if CLI changes; harder for users to generate for their version | Rejected — runtime generation is more flexible |

---

## Security Considerations

- [x] **No authentication**: Completion generation requires no Chrome connection or privileges
- [x] **No input validation risk**: Shell argument is constrained by `ValueEnum` enum — only valid shells accepted
- [x] **Output is inert**: Completion scripts are shell functions, not executable commands; the user explicitly sources them

---

## Performance Considerations

- [x] **Instant execution**: `clap_complete::generate()` is a pure in-memory string generation — sub-millisecond
- [x] **No network**: No CDP connection, no Chrome process needed
- [x] **No async**: Can run synchronously; no tokio runtime needed for this path

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI parsing | Unit | `Command::Completions` variant parsed correctly |
| Generation | Integration (BDD) | Each shell generates non-empty output with exit code 0 |
| Content validation | Integration (BDD) | Output contains known subcommand names |
| Error case | Integration (BDD) | Invalid shell name produces error |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `clap_complete` version incompatible with clap 4 | Low | High | Pin compatible version; both are maintained by same team |
| Generated completions missing dynamic values | Low | Low | All values are static enums — clap_complete handles `ValueEnum` natively |

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
