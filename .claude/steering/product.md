# chrome-cli Product Steering

This document defines the product vision, target users, and success metrics.
All feature development should align with these guidelines.

---

## Mission

**chrome-cli provides browser automation for developers and power users by exposing the Chrome DevTools Protocol through a fast, ergonomic command-line interface.**

<!-- TODO: Refine this mission statement to match your specific vision -->

---

## Target Users

### Primary: Developer / Automation Engineer

| Characteristic | Implication |
|----------------|-------------|
| Comfortable with CLI tools | CLI-first UX, scriptable output formats (JSON, plain text) |
| Writes CI/CD pipelines | Non-interactive mode, exit codes, deterministic behavior |
| Needs browser automation | CDP commands for navigation, screenshots, DOM interaction |

### Secondary: QA / Testing Engineer

| Characteristic | Implication |
|----------------|-------------|
| Runs browser-based test suites | Headless Chrome management, page load waiting |
| Needs reproducible results | Consistent behavior across runs, timeout controls |

<!-- TODO: Customize personas to match your actual target users -->

---

## Core Value Proposition

1. **Speed** — Native Rust binary, instant startup, no runtime overhead
2. **Simplicity** — Single binary, no Node.js/Python dependency, just `chrome-cli <command>`
3. **Scriptability** — Composable CLI commands with structured output for shell pipelines

---

## Product Principles

| Principle | Description |
|-----------|-------------|
| CLI-first | Every feature should work non-interactively in scripts and pipelines |
| Zero config | Sensible defaults; auto-discover Chrome installation |
| Cross-platform | macOS, Linux, and Windows support from day one |

<!-- TODO: Refine principles to guide decision-making when requirements conflict -->

---

## Success Metrics

| Metric | Target | Why It Matters |
|--------|--------|----------------|
| Startup time | < 50ms | CLI tools must feel instant |
| Binary size | < 10MB | Easy to distribute, fast CI downloads |
| Platform coverage | macOS + Linux + Windows | Broad adoption |

---

## Feature Prioritization

### Must Have (MVP)
- Connect to running Chrome instance via CDP
- Navigate to URL
- Take page screenshot
- Execute JavaScript in page context
- List open tabs/targets

### Should Have
- Launch and manage Chrome process
- Wait for page load / network idle
- DOM query and interaction commands
- Structured JSON output mode

### Could Have
- PDF export
- Performance profiling commands
- Network request interception
- Cookie management

### Won't Have (Now)
- GUI / TUI interface
- Firefox/Safari support (CDP-only for now)
- Built-in test runner

<!-- TODO: Adjust MoSCoW priorities to match your roadmap -->

---

## Key User Journeys

### Journey 1: Take a Screenshot

```
1. User runs: chrome-cli screenshot https://example.com -o screenshot.png
2. chrome-cli launches headless Chrome (or connects to existing)
3. Navigates to URL, waits for page load
4. Captures screenshot, saves to file
5. Exits with code 0
```

### Journey 2: Script Browser Automation

```
1. User runs: chrome-cli navigate https://example.com
2. User runs: chrome-cli eval "document.title"
3. Output: "Example Domain"
4. User pipes output into other CLI tools
```

<!-- TODO: Define your key user journeys — they become the basis for BDD acceptance criteria -->

---

## Brand Voice

| Attribute | Do | Don't |
|-----------|-----|-------|
| Concise | Short, actionable error messages | Verbose stack traces by default |
| Helpful | Suggest fixes in error messages | Just print cryptic error codes |

---

## Privacy Commitment

| Data | Usage | Shared |
|------|-------|--------|
| Browsing data | Transient, only during command execution | Never — all local |
| No telemetry | No usage data collected | N/A |

---

## References

- Technical spec: `.claude/steering/tech.md`
- Code structure: `.claude/steering/structure.md`
