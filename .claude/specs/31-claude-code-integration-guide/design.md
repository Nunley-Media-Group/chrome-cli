# Design: Claude Code Integration Guide

**Issue**: #31
**Date**: 2026-02-14
**Status**: Draft
**Author**: Claude (nmg-sdlc)

---

## Overview

This feature creates two new documentation files and updates the README to provide comprehensive Claude Code integration guidance. The primary deliverable is `docs/claude-code.md` — a guide explaining how to use chrome-cli with Claude Code, covering discovery, workflows, best practices, error handling, and an example conversation. The secondary deliverable is `examples/CLAUDE.md.example` — a drop-in template that developers can copy into their projects.

This is a documentation-only change. No Rust code, CLI commands, or binary changes are required. The existing minimal "Claude Code Integration" section in README.md (lines 265-288) will be replaced with a concise summary that links to the full guide.

---

## Architecture

### File Layout

```
chrome-cli/
├── docs/
│   └── claude-code.md              # NEW — Full integration guide
├── examples/
│   └── CLAUDE.md.example           # NEW — Drop-in CLAUDE.md template
└── README.md                       # MODIFIED — Replace inline section with link
```

### Document Structure: `docs/claude-code.md`

```
1. Introduction
   - What chrome-cli is and why it's built for AI agents
   - Prerequisites (chrome-cli installed, Chrome available)

2. Discovery & Setup
   - How Claude Code discovers chrome-cli (PATH, --help)
   - Machine-readable discovery via `capabilities` command
   - Learning commands via `examples` subcommand
   - Setup checklist

3. CLAUDE.md Template
   - Reference to examples/CLAUDE.md.example
   - How to customize for your project

4. Common Workflows
   4a. Testing Web Apps (navigate → snapshot → interact → verify)
   4b. Scraping Data (navigate → wait → snapshot/text → extract)
   4c. Debugging UI Issues (screenshot → console follow → network follow)
   4d. Form Automation (snapshot → form fill-many → submit → verify)

5. Recommended Workflow Loops
   - Interaction loop: snapshot → identify → interact → snapshot (verify)
   - Data extraction loop: navigate → wait → snapshot → extract
   - Diagram of each loop

6. Efficiency Tips
   - Use `form fill-many` for batch form filling
   - Use `--wait-until` to avoid race conditions
   - Use `page text` for simple content extraction vs `page snapshot` for interaction
   - Minimize round-trips by combining related operations

7. Error Handling for AI Agents
   - Exit code conventions (0 = success, non-zero = error)
   - Common error patterns and recovery strategies
   - Using `--timeout` to prevent hangs
   - Parsing stderr for error details

8. Best Practices Checklist
   - Always `page snapshot` before interaction commands
   - Use `--json` output for reliable parsing
   - Check exit codes
   - Use `--timeout` flags
   - Prefer `form fill` over `interact type`
   - Use `console follow` / `network follow` for debugging

9. Example Conversation
   - Multi-turn Claude Code session debugging a web app
   - Shows realistic command usage and output parsing

10. Reference
    - Link to `chrome-cli capabilities` for full command manifest
    - Link to `chrome-cli examples <cmd>` for per-command examples
    - Link to man pages
```

### Document Structure: `examples/CLAUDE.md.example`

```
# Browser Automation

Brief intro: this project uses chrome-cli

## Quick Start
- Connect, navigate, snapshot, screenshot

## Key Commands
- capabilities, examples, page snapshot, interact, form fill

## Workflow Loop
- snapshot → identify → interact → verify

## Tips
- JSON output, exit codes, timeouts, form fill preference
```

### README.md Changes

Replace lines 265-288 (current "Claude Code Integration" section) with:

```markdown
## Claude Code Integration

chrome-cli is designed for AI agent consumption. See the full
[Claude Code Integration Guide](docs/claude-code.md) for workflows,
best practices, and a drop-in [CLAUDE.md template](examples/CLAUDE.md.example).
```

---

## API / Interface Changes

None — this is a documentation-only feature. No CLI, API, or binary changes.

---

## Database / Storage Changes

None.

---

## State Management

None.

---

## UI Components

None.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Expand README inline** | Add all content directly to README.md | Single file, easy to find | README already long (~320 lines); bloats the main file | Rejected — README is already comprehensive |
| **B: Dedicated docs/ guide + example template** | `docs/claude-code.md` + `examples/CLAUDE.md.example` | Keeps README focused; guide is linkable; template is copy-pasteable | Two new directories to create | **Selected** — matches issue's acceptance criteria |
| **C: mdBook or doc site** | Generate a full documentation site | Searchable, navigable | Over-engineered for one guide; adds build dependency | Rejected — premature complexity |

---

## Security Considerations

- [x] **No sensitive data**: Documentation only — no credentials, secrets, or user data
- [x] **Command examples are safe**: All example commands use localhost and example URLs
- [x] **No code execution**: Guide content is static markdown

---

## Performance Considerations

Not applicable — documentation files have no runtime performance impact.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Documentation | Content verification (BDD) | All acceptance criteria verified via file existence and content checks |
| Commands | Accuracy check | All commands in guide match `chrome-cli capabilities` output |

Since this is a documentation feature, BDD tests will verify:
1. Files exist at expected paths
2. Required sections are present in each file
3. Commands referenced in the guide are valid chrome-cli commands
4. README links to the guide correctly

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Commands in guide become stale as CLI evolves | Medium | Medium | Reference `chrome-cli capabilities` and `chrome-cli examples` as canonical sources; note in guide that these commands are always up-to-date |
| Example conversation output doesn't match real CLI output | Low | Low | Keep example conversation output realistic but clearly marked as illustrative |
| CLAUDE.md template too verbose or too terse | Low | Medium | Balance: include essential commands with brief explanations; link to full guide for details |

---

## Open Questions

- None

---

## Validation Checklist

- [x] Architecture follows existing project patterns (markdown docs, examples dir)
- [x] All file paths documented
- [x] No database/storage changes
- [x] No state management changes
- [x] No UI components
- [x] Security considerations addressed (none needed)
- [x] Performance impact analyzed (none)
- [x] Testing strategy defined (BDD content verification)
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
