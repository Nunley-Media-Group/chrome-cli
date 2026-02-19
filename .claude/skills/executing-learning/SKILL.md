---
name: executing-learning
description: "Build debug binary, launch headed Chrome, explore a website to learn its navigation and features, file bugs found using /creating-issues."
argument-hint: "[url]"
disable-model-invocation: true
allowed-tools: Read, Glob, Grep, Bash, Write, Edit, Task, AskUserQuestion, Skill, EnterPlanMode
---

# Executing Learning

Build the debug binary, launch Chrome in **headed mode**, and explore a website to learn how to navigate it using chrome-cli. Exercise every relevant command group (navigate, page, interact, form, tabs, js, console, network, emulate, perf, dialog) against the site. Any bugs, broken commands, or missing functionality discovered during exploration should be filed as GitHub issues using `/creating-issues`.

## Default Target

**https://www.saucedemo.com/** — a demo e-commerce site with login, inventory, cart, and checkout flows.

If an argument is provided, use that URL instead:
- `/executing-learning` → crawls https://www.saucedemo.com/
- `/executing-learning https://example.com` → crawls https://example.com

## When to Use

- Learning how chrome-cli interacts with a new website
- Exploratory testing of chrome-cli against real-world sites
- Discovering bugs through hands-on usage rather than systematic testing
- Validating that chrome-cli can automate a specific site's workflows

## Key Constraints

1. **Headed mode** — launch Chrome without `--headless` so you can observe the browser
2. **Explore organically** — navigate the site like a user would, don't follow a rigid script
3. **Exercise breadth** — try as many chrome-cli commands as the site allows
4. **Note everything** — track what works, what fails, and what's missing
5. **File bugs with `/creating-issues`** — use the existing skill for each defect found
6. **Do NOT fix anything** — only observe, analyze, and report
7. **Clean up Chrome** — always kill Chrome processes when done

---

## Workflow

```
/executing-learning [url]
    │
    ├─ 1. Gather context (specs, steering docs)
    ├─ 2. Build debug binary
    ├─ 3. Launch headed Chrome
    ├─ 4. Explore the website
    │     ├─ Navigate pages
    │     ├─ Take snapshots, read accessibility tree
    │     ├─ Interact with elements (click, type, scroll)
    │     ├─ Fill and submit forms
    │     ├─ Test tabs, screenshots, JS execution
    │     ├─ Monitor console, network, dialogs
    │     └─ Note bugs and missing functionality
    ├─ 5. Clean up Chrome
    ├─ 6. File defect issues
    └─ 7. Summary report
```

---

### Step 1: Gather Context

1. Read `.claude/steering/product.md` for product vision and command inventory
2. Read `.claude/steering/tech.md` for build instructions
3. Read `.claude/steering/structure.md` for code organization (useful for root cause analysis)
4. Determine the target URL:
   - If `$ARGUMENTS` is provided and is a valid URL, use it
   - Otherwise, default to `https://www.saucedemo.com/`

### Step 2: Build Debug Binary

1. Run `cargo build` (debug mode)
2. If the build fails, **stop immediately** and report the failure
3. Set `CLI` to `./target/debug/chrome-cli` for all subsequent commands

### Step 3: Launch Headed Chrome

1. Run `./target/debug/chrome-cli connect --launch` (no `--headless` flag)
2. Verify the connection succeeded by checking the output for a port number
3. If launch fails, report the error and stop

### Step 4: Explore the Website

Navigate to the target URL and explore it organically. The goal is to learn the site's structure and exercise chrome-cli's capabilities against real content.

#### 4a: Initial Reconnaissance

1. Navigate to the target URL: `chrome-cli navigate <url>`
2. Take an accessibility snapshot: `chrome-cli page snapshot`
3. Take a screenshot: `chrome-cli page screenshot --file /tmp/learning-initial.png`
4. Extract page text: `chrome-cli page text`
5. Read the snapshot to understand the page structure and available interactive elements

#### 4b: Navigate the Site

Explore the site by interacting with links, buttons, and forms discovered in snapshots:

1. **Click navigation elements** — use `interact click <uid>` on links and buttons
2. **Fill forms** — use `form fill <uid> <value>` on input fields
3. **Navigate back/forward** — test `navigate back` and `navigate forward`
4. **Open new tabs** — test `tabs create <url>` for interesting pages
5. **Take snapshots at each page** — `page snapshot` to understand new content
6. **Take screenshots** — `page screenshot` to capture visual state

At each page, ask yourself:
- What can a user do here?
- What interactive elements exist?
- What would an AI agent need to automate this page?

#### 4c: Exercise Command Groups

As you explore, systematically try these command groups against the site's content:

| Command Group | What to Try |
|---------------|-------------|
| **navigate** | URL, back, forward, reload |
| **page** | snapshot, text, screenshot, find, resize |
| **interact** | click, hover, scroll, key, type |
| **form** | fill, fill-many, clear |
| **tabs** | list, create, activate, close |
| **js** | exec various expressions against page content |
| **console** | read, follow (briefly) |
| **network** | list requests during navigation |
| **emulate** | set viewport, color-scheme |
| **perf** | vitals |
| **dialog** | info (if dialogs appear) |
| **dom** | try it even though it may not be implemented |

#### 4d: Track Findings

As you explore, maintain a mental inventory of:

- **Working commands** — commands that behave correctly
- **Bugs** — commands that produce incorrect output, timeout unexpectedly, or crash
- **Missing functionality** — things you can't achieve with current commands
- **Unexpected behavior** — things that work but behave surprisingly

For each finding, note:
- The exact command and arguments
- stdout, stderr, and exit code
- What you expected vs what happened
- Which page/state you were on

### Step 5: Clean Up Chrome

1. Disconnect: `chrome-cli connect disconnect`
2. Kill any orphaned Chrome processes: `pkill -f 'chrome.*--remote-debugging' || true`
3. Verify cleanup: `pgrep -f 'chrome.*--remote-debugging' || echo "clean"`

### Step 6: File Defect Issues

For each bug or missing functionality found in Step 4, invoke `/creating-issues` with:

- A description of the defect or enhancement need
- The exact reproduction steps (CLI commands used)
- Expected vs actual behavior
- Root cause analysis (read source code if helpful)

**Do NOT attempt to fix any issues.** Only observe, analyze, and report.

### Step 7: Summary Report

Output a summary of the exploration:

```
## Exploration Summary

**Target**: [url]
**Commands Tested**: [count]
**Pages Visited**: [count]

### Working Commands
| Command | Notes |
|---------|-------|
| navigate <url> | Navigated successfully to all pages |
| page snapshot | Accessibility tree captured correctly |
| ... | ... |

### Issues Found
| # | Title | Type |
|---|-------|------|
| #NNN | [title] | Bug |
| #NNN | [title] | Enhancement |
| ... | ... | ... |

### Missing Functionality
- [Things you couldn't do with current commands]

Total: X commands tested, Y issues filed
```

---

## Tips for Effective Exploration

- **Read snapshots carefully** — the accessibility tree UIDs (s1, s2, ...) are your primary way to target elements
- **Try edge cases** — empty forms, rapid navigation, multiple tabs
- **Test error paths** — invalid selectors, nonexistent elements, timeouts
- **Compare JSON output** — check that structured output is complete and correct
- **Use `--json` and `--plain`** — verify output format flags work
- **Chain commands** — test realistic multi-step workflows (login → browse → add to cart → checkout)
