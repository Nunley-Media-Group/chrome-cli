# Design: README with Quick-Start, Examples, and Architecture Overview

**Issue**: #28
**Date**: 2026-02-14
**Status**: Approved
**Author**: Claude (writing-specs)

---

## Overview

This feature replaces the minimal placeholder README.md with a comprehensive project documentation page. The README is a single Markdown file at the repository root, with no backend, state management, or API changes. The design focuses on content structure, section ordering, and Markdown formatting conventions that render correctly on GitHub, crates.io, and terminal viewers.

The key architectural decision is keeping everything in a single README.md file with collapsible `<details>` sections for lengthy content, linking out to man pages and `--help` for detailed command reference rather than duplicating content.

---

## Architecture

### Document Structure

```
README.md
├── Header (title, badges, description)
├── Features (bullet list + comparison table)
├── Installation (binary, cargo, source)
├── Quick Start (5-step guide)
├── Usage Examples (collapsible workflows)
├── Command Reference (table of all commands)
├── Architecture (CDP diagram, session model)
├── Claude Code Integration (CLAUDE.md snippet)
├── Contributing (dev setup, testing, style)
└── License (dual MIT/Apache-2.0)
```

### Content Flow

```
1. User lands on README (GitHub or crates.io)
2. Sees badges → trust signals (CI passing, license)
3. Reads description → understands what chrome-cli does
4. Scans Features → decides if it meets their needs
5. Follows Installation → gets the binary
6. Follows Quick Start → first successful interaction
7. Explores Usage Examples → deeper workflows
8. Checks Command Reference → discovers full capabilities
9. Reads Architecture → understands how it works
10. (Optional) Contributing → becomes a contributor
```

---

## File Changes

### Modified Files

| File | Change | Purpose |
|------|--------|---------|
| `README.md` | Replace content | Full documentation rewrite |

No other files are created or modified.

---

## Section Design Details

### Header Section

- H1 heading: `# chrome-cli`
- One-line description from Cargo.toml: "A CLI tool for browser automation via the Chrome DevTools Protocol"
- Badge row using Markdown image links:
  - CI status: `![CI](https://github.com/Nunley-Media-Group/chrome-cli/actions/workflows/ci.yml/badge.svg)`
  - License: `![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)`
  - Crates.io placeholder (commented out until published)

### Features Section

- Bullet list of 10+ capabilities derived from actual CLI commands
- Comparison table: chrome-cli vs Chrome DevTools MCP vs Puppeteer/Playwright CLI
  - Columns: Feature, chrome-cli, Alternatives
  - Key differentiators: standalone binary, no runtime, shell-native

### Installation Section

Three methods in order of ease:

1. **Pre-built binaries** — curl one-liners for macOS (ARM + Intel) and Linux (x64 + ARM), with platform detection
2. **Cargo install** — `cargo install chrome-cli`
3. **Build from source** — git clone + cargo build

Supported platforms table derived from release.yml targets:
- macOS ARM (aarch64-apple-darwin)
- macOS Intel (x86_64-apple-darwin)
- Linux x64 (x86_64-unknown-linux-gnu)
- Linux ARM (aarch64-unknown-linux-gnu)
- Windows (x86_64-pc-windows-msvc)

### Quick Start Section

Numbered steps with actual commands from the CLI help output:

```
1. Install chrome-cli (link to Installation)
2. Start Chrome: google-chrome --remote-debugging-port=9222
3. Connect: chrome-cli connect
4. Navigate: chrome-cli navigate https://example.com
5. Inspect: chrome-cli page snapshot
```

### Usage Examples Section

Each workflow in a collapsible `<details>` section:

1. **Taking a screenshot** — `chrome-cli page screenshot`
2. **Extracting page text** — `chrome-cli page text`
3. **Executing JavaScript** — `chrome-cli js exec`
4. **Filling forms** — `chrome-cli form fill` / `chrome-cli form fill-many`
5. **Monitoring network** — `chrome-cli network follow`
6. **Performance tracing** — `chrome-cli perf start` / `chrome-cli perf stop`

### Command Reference Section

Markdown table with columns: Command, Description. Derived from `chrome-cli --help` output. All 16 commands (connect, tabs, navigate, page, dom, js, console, network, interact, form, emulate, perf, dialog, config, completions, man).

### Architecture Section

ASCII diagram showing the layer architecture:

```
CLI (clap) → Command Dispatch → CDP Client (WebSocket) → Chrome Browser
```

Brief descriptions of:
- CDP communication model (JSON-RPC over WebSocket)
- Session management (connect, persistent session file)
- Output model (JSON to stdout, errors to stderr, exit codes)

### Claude Code Integration Section

- Brief explanation of chrome-cli as a tool for AI agents
- Example CLAUDE.md snippet showing how to configure chrome-cli in a project
- Common AI agent workflows (screenshot → analyze, navigate → extract text)

### Contributing Section

- Prerequisites: Rust 1.85.0+, Chrome/Chromium
- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy` and `cargo fmt --check`
- Man pages: `cargo xtask man`

### License Section

Dual license statement with links to LICENSE-MIT and LICENSE-APACHE.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Single README.md** | All docs in one file with collapsible sections | Simple, single source of truth, GitHub-native | Can get long | **Selected** |
| **B: README + docs/ directory** | Split into multiple files | Better organization for large docs | Overhead for this scope, harder to discover | Rejected — premature for current needs |
| **C: mdBook site** | Hosted documentation site | Rich navigation, search | Requires hosting setup, maintenance | Rejected — future consideration |

---

## Security Considerations

- [x] **No secrets**: README contains no credentials or tokens
- [x] **No user input**: Static file, no injection concerns
- [x] **Badge URLs**: Only reference trusted badge services (GitHub Actions, shields.io)

---

## Performance Considerations

- [x] **File size**: Keep README under 500 lines to ensure fast rendering
- [x] **Images**: No images embedded (ASCII diagrams only) to keep the repo lightweight
- [x] **Collapsible sections**: Reduce visual load for scanning

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Content | BDD/Acceptance | Verify all required sections exist with expected content |
| Accuracy | Manual | Verify commands match current CLI output |
| Rendering | Manual | Verify badges and formatting on GitHub |

BDD tests will parse the README.md file and verify:
- Required sections (headings) are present
- Badge Markdown syntax is valid
- Command examples reference actual CLI commands
- Collapsible sections use correct HTML syntax

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Commands change after README written | Medium | Low | Command reference derived from --help output; update when commands change |
| Badges break if repo moves | Low | Low | Use relative URLs where possible |
| crates.io badge invalid before publish | Medium | Low | Comment out crates.io badge until published |

---

## Open Questions

- None

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All file changes documented
- [x] No database/storage changes needed
- [x] No state management changes needed
- [x] No UI components (CLI tool — README is static docs)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
