---
name: executing-e2e-test
description: "Build debug binary, run end-to-end CLI tests against a real website, file defect issues for all findings."
disable-model-invocation: true
allowed-tools: Read, Glob, Grep, Bash(cargo build:*), Bash(kill:*), Bash(pkill:*), Bash(pgrep:*)
---

# Executing End-to-End Tests

Build the debug binary, run every CLI command against a real website (google.com) in headless mode, identify defects through actual usage, and file GitHub issues for all findings using `/creating-issues`.

## When to Use

- After a release milestone to validate all commands work end-to-end
- After major refactoring to catch regressions
- When you want a full smoke test of every CLI command group against a live site

## Key Constraints

1. **Do NOT fix anything** — only observe, analyze, and report
2. **Do NOT use existing BDD tests** — run the actual CLI binary directly
3. **Run headless** — always use `--headless` flag on connect
4. **Use spec files** for expected behavior — don't look at GitHub issues
5. **Build debug** — use the debug binary for richer diagnostics
6. **Monitor output** — capture and display all stdout/stderr for root cause analysis
7. **Use `/creating-issues`** — invoke the existing skill for each defect found

## Workflow

### Step 1: Read Spec Files for Command Inventory

1. Use `Glob` for `.claude/specs/*/requirements.md` and read each spec to build an inventory of all CLI commands and subcommands
2. Read `.claude/steering/product.md` for the product vision and feature list
3. Read `.claude/steering/tech.md` for build instructions and test standards
4. Compile a complete list of command groups and their expected behaviors

### Step 2: Build Debug Binary

1. Run `cargo build` (debug mode for richer error output)
2. Capture and display the build output
3. If the build fails, **stop immediately** and report the build failure — do not continue
4. Set `CLI` variable to `./target/debug/chrome-cli` for all subsequent commands

### Step 3: Enter Plan Mode

Enter plan mode and design a comprehensive test plan covering all major command groups. The plan must include test cases for each of the following:

- **connect**: launch headless, status, disconnect
- **tabs**: list, create, create --background, activate, close
- **navigate**: url, back, forward, reload, wait-until variants
- **page**: text, snapshot, screenshot, find, resize
- **js**: exec various expressions, error cases
- **console**: read, errors-only
- **network**: list, filter by type/url, get
- **interact**: scroll, key, click
- **form**: fill
- **emulate**: set viewport/mobile, color-scheme, network throttling, reset, status
- **perf**: vitals, start/stop trace
- **dialog**: info, auto-dismiss
- **config**: show, path, init
- **session**: implicit via connect/disconnect lifecycle
- **capabilities**, **examples**, **completions**, **man**, **--version**, **--help**

All tests target **google.com** and run **headless**.

Present the plan to the user for approval before proceeding.

### Step 4: Execute Tests Systematically

1. Launch Chrome headless via `chrome-cli connect --launch --headless`
2. For each command group in the approved plan, run the actual CLI binary and capture:
   - **stdout** (JSON output)
   - **stderr** (error output)
   - **exit code**
3. Log all output with clear headers for each test case
4. After each test, note **pass/fail** and any unexpected behavior
5. Compare actual behavior against spec expectations from Step 1

### Step 5: Collect and Analyze Findings

For each failure or unexpected behavior:

1. **Record** the exact command and arguments used
2. **Record** stdout, stderr, and exit code
3. **Identify** the relevant spec file from Step 1
4. **Perform root cause analysis** by reading source code (`src/*.rs`) to understand why the behavior differs from the spec
5. **Classify severity**:
   - **Crash** — process panics or segfaults
   - **Wrong output** — output does not match spec
   - **Missing feature** — command or option not implemented
   - **Cosmetic** — minor formatting or message issues

### Step 6: Clean Up Chrome

1. Run `chrome-cli connect --disconnect`
2. Verify no orphan Chrome processes remain using `pgrep -f chrome`
3. Kill any remaining headless Chrome processes

### Step 7: File Defect Issues

For each finding from Step 5, invoke `/creating-issues` with:

- A description of the defect
- The root cause analysis from Step 5
- Reproduction steps (the exact CLI command)
- Expected vs actual behavior (referencing the spec)

**Do NOT attempt to fix any issues.** Only observe, analyze, and report.

### Step 8: Summary Report

Output a summary table of all tests run:

```
| Command Group | Test Case           | Result | Issue |
|---------------|---------------------|--------|-------|
| connect       | launch headless     | PASS   | —     |
| tabs          | create --background | FAIL   | #NNN  |
| ...           | ...                 | ...    | ...   |

Total: X passed, Y failed
Issues created: #N1, #N2, ...
```

List all created GitHub issue numbers with their titles.
