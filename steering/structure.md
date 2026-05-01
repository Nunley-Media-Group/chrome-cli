# agentchrome Code Structure Steering

This document defines code organization, naming conventions, and patterns.
All code should follow these guidelines for consistency.

---

## Project Layout

```
agentchrome/
├── src/
│   ├── main.rs               # Entry point, CLI parsing, command dispatch
│   ├── lib.rs                 # Library target (exposes modules for tests/xtask)
│   ├── cli/
│   │   └── mod.rs             # clap derive types (Cli, Command, GlobalOpts, subcommand args)
│   ├── cdp/
│   │   ├── mod.rs             # CDP module root
│   │   ├── client.rs          # WebSocket CDP client (send/receive JSON-RPC)
│   │   ├── transport.rs       # Low-level WebSocket transport
│   │   ├── types.rs           # CDP protocol types and message structs
│   │   └── error.rs           # CDP-specific error types
│   ├── chrome/
│   │   ├── mod.rs             # Chrome module root (re-exports)
│   │   ├── discovery.rs       # Chrome instance discovery (port scanning, version query)
│   │   ├── launcher.rs        # Chrome process launch and management
│   │   ├── platform.rs        # Platform-specific Chrome paths (macOS/Linux/Windows)
│   │   └── error.rs           # Chrome-specific error types
│   ├── connection.rs          # Connection management (connect, disconnect, auto-discover)
│   ├── session.rs             # Session file persistence (~/.config/agentchrome/session.json)
│   ├── config.rs              # Configuration file loading and merging
│   ├── error.rs               # Top-level error types (AppError, ExitCode)
│   ├── snapshot.rs            # Accessibility tree snapshot and formatting
│   ├── navigate.rs            # URL navigation commands (navigate, back, forward, reload)
│   ├── tabs.rs                # Tab management commands (list, create, close, activate)
│   ├── page.rs                # Page commands (screenshot, text, snapshot, find)
│   ├── js.rs                  # JavaScript execution commands
│   ├── form.rs                # Form fill/submit commands
│   ├── interact.rs            # Mouse, keyboard, scroll interaction commands
│   ├── console.rs             # Console message reading and following
│   ├── network.rs             # Network monitoring and interception commands
│   ├── emulate.rs             # Device/network/CPU emulation commands
│   ├── perf.rs                # Performance tracing and metrics commands
│   ├── dialog.rs              # Browser dialog handling commands
│   ├── capabilities.rs        # Capabilities manifest subcommand
│   └── examples.rs            # Built-in examples subcommand
├── tests/
│   ├── bdd.rs                 # BDD test runner (all cucumber World definitions + steps)
│   ├── cdp_integration.rs     # CDP client integration tests
│   └── features/              # Gherkin feature files (one per feature/fix)
│       ├── cli-skeleton.feature
│       ├── session-connection-management.feature
│       ├── tab-management.feature
│       ├── url-navigation.feature
│       ├── ...                # ~50 feature files covering all commands and bug fixes
│       └── 102-fix-network-list-empty-array.feature
├── xtask/
│   ├── Cargo.toml             # xtask package (man page generation)
│   └── src/
│       └── main.rs            # `cargo xtask man` — generates man pages from clap
├── docs/
│   └── codex.md               # Codex integration guide
├── examples/
│   └── AGENTS.md.example      # Template AGENTS.md for users
├── .github/
│   ├── workflows/
│   │   ├── ci.yml             # CI pipeline (build, test, lint, fmt)
│   │   └── release.yml        # Cross-platform release pipeline
│   └── dependabot.yml         # Dependency update automation
├── .cargo/
│   └── config.toml            # Cargo aliases (xtask)
├── Cargo.toml                 # Workspace + package manifest
├── Cargo.lock                 # Dependency lock file
├── VERSION                    # Single source of truth for version (0.1.0)
├── rust-toolchain.toml        # Pinned Rust toolchain (1.93.0)
├── rustfmt.toml               # Formatter config (edition 2024)
├── LICENSE-APACHE             # Apache 2.0 license
├── LICENSE-MIT                # MIT license
└── README.md                  # Project readme
```

---

## Layer Architecture

### Request / Data Flow

```
CLI Input (args)
    ↓
┌─────────────────┐
│   CLI Layer      │ ← Parse args (clap derive), validate input
└────────┬────────┘
         ↓
┌─────────────────┐
│   main.rs        │ ← Command dispatch, config loading, session management
└────────┬────────┘
         ↓
┌─────────────────┐
│ Command Modules  │ ← Business logic per command (navigate.rs, tabs.rs, ...)
└────────┬────────┘
         ↓
┌─────────────────┐
│   CDP Client     │ ← WebSocket JSON-RPC communication with Chrome
└────────┬────────┘
         ↓
┌─────────────────┐
│  Chrome Layer    │ ← Chrome process discovery, launch, lifecycle
└────────┬────────┘
         ↓
   Chrome Browser
```

### Layer Responsibilities

| Layer | Does | Doesn't Do |
|-------|------|------------|
| CLI (`cli/mod.rs`) | Parse arguments, define subcommands, validate user input | Business logic, CDP communication, output formatting |
| Dispatch (`main.rs`) | Load config, manage session, route to command modules, handle errors | CDP protocol details, Chrome process management |
| Command modules (`*.rs`) | Implement command semantics, call CDP client, format JSON output | Parse CLI args, manage Chrome process directly |
| CDP (`cdp/`) | Send/receive CDP messages, manage WebSocket, handle JSON-RPC | Know about CLI commands, format output |
| Chrome (`chrome/`) | Launch/discover Chrome, manage process lifecycle, platform paths | CDP protocol details, CLI concerns |
| Session (`session.rs`) | Persist/load session data (WebSocket URL, PID) | Business logic, CDP communication |
| Config (`config.rs`) | Load/parse TOML config files, merge with CLI args | Command execution, Chrome management |

---

## Naming Conventions

### Rust

| Element | Convention | Example |
|---------|------------|---------|
| Files | snake_case | `cdp_client.rs` |
| Modules | snake_case | `mod chrome` |
| Types/Structs | PascalCase | `CdpClient`, `SessionData` |
| Traits | PascalCase | `CommandFactory` |
| Functions | snake_case | `connect_to_chrome()` |
| Constants | SCREAMING_SNAKE | `DEFAULT_CDP_PORT` |
| Variables | snake_case | `page_url` |
| Enum variants | PascalCase | `ExitCode::ConnectionError` |

### Feature Files

| Element | Convention | Example |
|---------|------------|---------|
| Feature files | kebab-case | `tab-management.feature` |
| Bug fix features | `{issue#}-{description}` | `102-fix-network-list-empty-array.feature` |

---

## File Templates

### CLI Command Module

```rust
use crate::error::Result;
use crate::output::OutputMode;

pub async fn run(/* parsed args */) -> Result<()> {
    // Validate CLI-level arguments, call domain helpers, and write structured output.
    Ok(())
}
```

### BDD Feature

```gherkin
Feature: Capability name
  As an AI agent
  I want a precise browser automation command
  So that I can continue from machine-readable evidence

  Scenario: Acceptance criterion title
    Given a launched browser session
    When I run the documented command
    Then the JSON output contains the expected field
```

---

## Import Order

```rust
// 1. Standard library
use std::path::PathBuf;

// 2. External crates
use clap::Parser;
use tokio::net::TcpStream;

// 3. Crate-level imports
use agentchrome::cdp::CdpClient;
use agentchrome::error::Result;

// 4. Local module imports
use crate::cli::Command;
```

---

## Anti-Patterns to Avoid

| Anti-Pattern | Problem | Solution |
|--------------|---------|----------|
| `unwrap()` in non-test code | Panics on error, bad UX | Use `?` operator with typed errors |
| Leaking CDP details to CLI layer | Tight coupling | Use command modules as boundary |
| Platform-specific code in shared modules | Breaks cross-platform | Isolate in `chrome/platform.rs` |
| Blocking I/O in async context | Deadlocks, poor performance | Use async equivalents or `spawn_blocking` |
| Hard-coded Chrome paths | Breaks across OS/installs | Use `chrome/discovery.rs` with env var override |
| Unstructured output to stdout | Breaks AI agent consumption | Always output JSON to stdout |

---

## References

- `steering/product.md` for product direction
- `steering/tech.md` for technical standards
