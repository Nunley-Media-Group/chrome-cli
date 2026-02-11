# chrome-cli Code Structure Steering

This document defines code organization, naming conventions, and patterns.
All code should follow these guidelines for consistency.

---

## Project Layout

```
chrome-cli/
├── src/
│   └── main.rs              # Entry point (currently prints version)
├── Cargo.toml                # Workspace + package manifest
├── Cargo.lock                # Dependency lock file
├── rust-toolchain.toml       # Pinned Rust toolchain (1.85.0)
├── rustfmt.toml              # Formatter config (edition 2024)
├── LICENSE-APACHE            # Apache 2.0 license
├── LICENSE-MIT               # MIT license
└── README.md                 # Project readme
```

### Planned Structure (as project grows)

```
chrome-cli/
├── src/
│   ├── main.rs               # Entry point, CLI setup
│   ├── cli/                   # CLI argument parsing and command dispatch
│   │   ├── mod.rs
│   │   └── commands/          # One module per command group
│   ├── cdp/                   # Chrome DevTools Protocol client
│   │   ├── mod.rs
│   │   ├── client.rs          # WebSocket CDP client
│   │   ├── types.rs           # CDP protocol types
│   │   └── commands/          # CDP domain implementations
│   ├── chrome/                # Chrome process management
│   │   ├── mod.rs
│   │   ├── launcher.rs        # Chrome process launch/discovery
│   │   └── platform/          # Platform-specific Chrome paths
│   ├── output/                # Output formatting (plain, JSON)
│   │   └── mod.rs
│   └── error.rs               # Error types
├── tests/
│   ├── features/              # Gherkin feature files
│   ├── steps/                 # BDD step definitions
│   └── integration/           # Integration tests
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
└── README.md
```

---

## Layer Architecture

### Request / Data Flow

```
CLI Input (args)
    ↓
┌─────────────────┐
│   CLI Layer      │ ← Parse args, validate input, dispatch commands
└────────┬────────┘
         ↓
┌─────────────────┐
│  Command Layer   │ ← Business logic for each command
└────────┬────────┘
         ↓
┌─────────────────┐
│   CDP Layer      │ ← Chrome DevTools Protocol communication
└────────┬────────┘
         ↓
┌─────────────────┐
│  Chrome Layer    │ ← Chrome process management, discovery
└────────┬────────┘
         ↓
   Chrome Browser
```

### Layer Responsibilities

| Layer | Does | Doesn't Do |
|-------|------|------------|
| CLI | Parse arguments, validate user input, format output | Business logic, CDP communication |
| Command | Orchestrate CDP calls, implement command semantics | Parse CLI args, manage Chrome process directly |
| CDP | Send/receive CDP messages, manage WebSocket | Know about CLI commands, format output |
| Chrome | Launch/discover Chrome, manage process lifecycle | CDP protocol details, CLI concerns |
| Output | Format results as plain text or JSON | Business logic, know about CDP |

---

## Naming Conventions

### Rust

| Element | Convention | Example |
|---------|------------|---------|
| Files | snake_case | `cdp_client.rs` |
| Modules | snake_case | `mod chrome_launcher` |
| Types/Structs | PascalCase | `CdpClient`, `NavigateCommand` |
| Traits | PascalCase | `CommandHandler`, `OutputFormatter` |
| Functions | snake_case | `connect_to_chrome()` |
| Constants | SCREAMING_SNAKE | `DEFAULT_CDP_PORT` |
| Variables | snake_case | `page_url` |
| Enum variants | PascalCase | `OutputFormat::Json` |

---

## File Templates

### Command Module

```rust
// src/cli/commands/navigate.rs

use crate::cdp::CdpClient;
use crate::error::Result;

pub struct NavigateArgs {
    pub url: String,
    pub wait: Option<WaitCondition>,
}

pub async fn execute(client: &CdpClient, args: NavigateArgs) -> Result<()> {
    client.navigate(&args.url).await?;
    // ...
    Ok(())
}
```

### CDP Domain Module

```rust
// src/cdp/commands/page.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct NavigateParams {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct NavigateResult {
    pub frame_id: String,
}
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
use crate::cdp::CdpClient;
use crate::error::Result;

// 4. Super/self imports
use super::CommandHandler;
```

---

## Anti-Patterns to Avoid

| Anti-Pattern | Problem | Solution |
|--------------|---------|----------|
| `unwrap()` in non-test code | Panics on error, bad UX | Use `?` operator with typed errors |
| Leaking CDP details to CLI layer | Tight coupling | Use command layer as boundary |
| Platform-specific code in shared modules | Breaks cross-platform | Isolate in `platform/` modules |
| Blocking I/O in async context | Deadlocks, poor performance | Use async equivalents or `spawn_blocking` |
| Hard-coded Chrome paths | Breaks across OS/installs | Use discovery logic with env var override |

---

## References

- CLAUDE.md for project overview
- `.claude/steering/product.md` for product direction
- `.claude/steering/tech.md` for technical standards
