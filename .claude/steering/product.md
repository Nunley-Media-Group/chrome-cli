# chrome-cli Product Steering

This document defines the product vision, target users, and success metrics.
All feature development should align with these guidelines.

---

## Mission

**chrome-cli provides browser automation for developers and AI agents by exposing the Chrome DevTools Protocol through a fast, ergonomic command-line interface.**

---

## Target Users

### Primary: AI Agents (Claude Code, MCP clients)

| Characteristic | Implication |
|----------------|-------------|
| Consumes structured JSON output | All commands produce JSON on stdout, JSON errors on stderr |
| Operates non-interactively | Deterministic exit codes, no prompts, timeout controls |
| Chains multi-step browser workflows | Session persistence, tab targeting, accessibility-tree-driven interaction |

### Secondary: Developer / Automation Engineer

| Characteristic | Implication |
|----------------|-------------|
| Comfortable with CLI tools | CLI-first UX, scriptable output, shell pipeline composition |
| Writes CI/CD pipelines | Headless mode, non-interactive, deterministic behavior |
| Needs browser automation | CDP commands for navigation, screenshots, DOM interaction |

---

## Core Value Proposition

1. **Speed** — Native Rust binary, sub-50ms startup, no runtime overhead
2. **Simplicity** — Single binary, no Node.js/Python dependency, just `chrome-cli <command>`
3. **Scriptability** — Composable CLI commands with structured JSON output for shell pipelines and AI agents
4. **AI-native** — Accessibility-tree snapshots, structured output, and session management designed for agent consumption

---

## Product Principles

| Principle | Description |
|-----------|-------------|
| CLI-first | Every feature works non-interactively in scripts and pipelines |
| Zero config | Sensible defaults; auto-discover Chrome installation |
| Cross-platform | macOS, Linux, and Windows support |
| Structured output | JSON on stdout, JSON errors on stderr, meaningful exit codes |

---

## Success Metrics

| Metric | Target | Why It Matters |
|--------|--------|----------------|
| Startup time | < 50ms | CLI tools must feel instant |
| Binary size | < 10MB | Easy to distribute, fast CI downloads |
| Platform coverage | macOS + Linux + Windows | Broad adoption |

---

## Feature Prioritization

### Must Have (Shipped)
- Connect to / launch Chrome instance via CDP
- Session management with auto-discovery and reconnection
- Tab management (list, create, close, activate)
- URL navigation (navigate, back, forward, reload)
- Page inspection (text, accessibility tree snapshot, element finding)
- Screenshots (viewport and full-page)
- JavaScript execution in page context
- Form filling by accessibility UID
- Mouse, keyboard, and scroll interactions
- Console message reading and monitoring
- Network request monitoring and interception
- Device / network / CPU emulation
- Performance tracing and Core Web Vitals
- Browser dialog handling (alert, confirm, prompt, beforeunload)
- Configuration file support
- Shell completions (Bash, Zsh, Fish, PowerShell, Elvish)
- Man page generation and viewer
- Built-in examples subcommand
- Capabilities manifest subcommand

### Won't Have (Now)
- GUI / TUI interface
- Firefox/Safari support (CDP-only)
- Built-in test runner

---

## Key User Journeys

### Journey 1: AI Agent Browser Automation

```
1. Agent runs: chrome-cli connect --launch --headless
2. Agent runs: chrome-cli navigate https://example.com
3. Agent runs: chrome-cli page snapshot
4. Agent reads accessibility tree, identifies form fields by UID
5. Agent runs: chrome-cli form fill s5 "value"
6. Agent runs: chrome-cli page screenshot --file result.png
7. Agent runs: chrome-cli connect disconnect
```

### Journey 2: Shell Script Automation

```
1. User connects: chrome-cli connect
2. User navigates: chrome-cli navigate https://example.com
3. User extracts: chrome-cli js exec "document.title" | jq -r .result
4. User screenshots: chrome-cli page screenshot --file shot.png
5. Exit code 0 confirms success
```

---

## References

- Technical spec: `.claude/steering/tech.md`
- Code structure: `.claude/steering/structure.md`
