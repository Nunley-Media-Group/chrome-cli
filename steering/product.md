# agentchrome Product Steering

This document defines the product vision, target users, and success metrics.
All feature development should align with these guidelines.

---

## Mission

**agentchrome provides browser automation for developers and AI agents by exposing the Chrome DevTools Protocol through a fast, ergonomic command-line interface.**

---

## Target Users

### Primary: AI Agents (Codex, MCP clients)

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
2. **Simplicity** — Single binary, no Node.js/Python dependency, just `agentchrome <command>`
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
1. Agent runs: agentchrome connect --launch --headless
2. Agent runs: agentchrome navigate https://example.com
3. Agent runs: agentchrome page snapshot
4. Agent reads accessibility tree, identifies form fields by UID
5. Agent runs: agentchrome form fill s5 "value"
6. Agent runs: agentchrome page screenshot --file result.png
7. Agent runs: agentchrome connect disconnect
```

### Journey 2: Shell Script Automation

```
1. User connects: agentchrome connect
2. User navigates: agentchrome navigate https://example.com
3. User extracts: agentchrome js exec "document.title" | jq -r .result
4. User screenshots: agentchrome page screenshot --file shot.png
5. Exit code 0 confirms success
```

---

## Brand Voice

| Attribute | Do | Don't |
|-----------|----|-------|
| Precise | Use concrete command names, output fields, and failure modes. | Describe browser automation behavior in vague or marketing language. |
| Agent-oriented | Write docs and examples for an AI agent or script that must decide its next command from structured output. | Assume a human will inspect an interactive browser to recover missing context. |
| Operational | Surface setup, verification, and cleanup expectations directly. | Hide required preconditions behind prose or implicit examples. |

---

## Privacy Commitment

| Data | Usage | Shared |
|------|-------|--------|
| Browser page content | Returned only when the user explicitly requests snapshots, text, DOM, screenshots, console, or network details. | Not shared by AgentChrome itself. Callers decide where command output goes. |
| Local session metadata | Used to reconnect to the selected Chrome instance and clean up launched processes. | Stored locally in the configured session path. |
| Screenshots and artifacts | Written only to user-selected output paths. | Not uploaded by AgentChrome. |

---

## References

- Technical spec: `steering/tech.md`
- Code structure: `steering/structure.md`
