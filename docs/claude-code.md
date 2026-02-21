# Claude Code Integration Guide

Use AgentChrome to give Claude Code browser automation capabilities — navigate pages,
inspect content, fill forms, take screenshots, and debug web apps, all from the CLI.

## Prerequisites

- **AgentChrome** installed and on your `PATH` ([Installation](../README.md#installation))
- **Chrome or Chromium** installed (any channel: stable, beta, canary, dev)

Verify your setup:

```sh
# Confirm agentchrome is available
agentchrome --help

# Machine-readable command manifest (JSON)
agentchrome capabilities

# See examples for any command
agentchrome examples navigate
```

## Discovery & Setup

Claude Code discovers AgentChrome through three mechanisms:

1. **PATH lookup** — Claude Code runs shell commands, so `agentchrome` must be on `PATH`.
2. **`--help` text** — Every command and subcommand supports `--help` for usage details.
3. **`capabilities` command** — `agentchrome capabilities` outputs a complete JSON manifest of all commands, flags, arguments, and exit codes. This is the fastest way for an AI agent to learn the full CLI surface.
4. **`examples` command** — `agentchrome examples <command>` shows practical usage examples for each command group.

### Setup checklist

1. Install AgentChrome (`cargo install agentchrome` or download a release binary)
2. Verify: `agentchrome --help`
3. Launch Chrome: `agentchrome connect --launch --headless`
4. Test the connection: `agentchrome page text`
5. Add AgentChrome to your project's `CLAUDE.md` (see [CLAUDE.md Template](#claudemd-template) below)

## CLAUDE.md Template

Drop the file [`examples/CLAUDE.md.example`](../examples/CLAUDE.md.example) into your project root as `CLAUDE.md` (or append its contents to an existing one). This teaches Claude Code how to use AgentChrome for browser automation in your project.

Customize the template:
- Replace `http://localhost:3000` with your dev server URL
- Add project-specific workflows (e.g., "log in as test user", "navigate to admin panel")
- Include any environment-specific notes (ports, authentication tokens, etc.)

## Common Workflows

### Testing Web Apps

Verify that your web application renders correctly and interactive elements work:

```sh
# 1. Connect to Chrome (launches headless if not running)
agentchrome connect --launch --headless

# 2. Navigate to the app
agentchrome navigate http://localhost:3000 --wait-until networkidle

# 3. Capture the accessibility tree to see the page structure
agentchrome page snapshot

# 4. Click a button (using the UID from the snapshot)
agentchrome interact click s5

# 5. Verify the result — take a new snapshot
agentchrome page snapshot
```

### Scraping Data

Extract structured content from web pages:

```sh
# 1. Navigate to the target page
agentchrome navigate https://example.com/data --wait-until networkidle

# 2. Get plain text content
agentchrome page text

# 3. Or get the accessibility tree for structured data
agentchrome page snapshot

# 4. Run JavaScript to extract specific data
agentchrome js exec "JSON.stringify([...document.querySelectorAll('tr')].map(r => r.textContent))"
```

### Debugging UI Issues

Investigate visual bugs, console errors, and network problems:

```sh
# 1. Take a screenshot to see the current state
agentchrome page screenshot --file debug.png

# 2. Check for console errors
agentchrome console read --errors-only

# 3. Follow console messages in real time while reproducing the issue
agentchrome console follow --timeout 10000

# 4. Monitor network requests to find failed API calls
agentchrome network follow --timeout 10000 --type xhr,fetch

# 5. Inspect a specific element by searching for it
agentchrome page find "Submit" --role button
```

### Form Automation

Fill and submit forms reliably:

```sh
# 1. Snapshot the page to discover form field UIDs
agentchrome page snapshot

# 2. Fill multiple fields at once (most reliable approach)
agentchrome form fill-many '[{"uid": "s5", "value": "user@example.com"}, {"uid": "s8", "value": "password123"}]'

# 3. Or fill fields individually
agentchrome form fill s5 "user@example.com"
agentchrome form fill s8 "password123"

# 4. Click the submit button
agentchrome interact click s10

# 5. Verify submission succeeded
agentchrome page snapshot
```

## Recommended Workflow Loops

### Interaction Loop

Use this loop whenever you need to click, type, or otherwise interact with elements:

```
snapshot → identify target → interact → snapshot (verify)
```

```sh
# Step 1: Get the current page state
agentchrome page snapshot
# Step 2: Identify the target element UID from the snapshot output
# Step 3: Interact with the element
agentchrome interact click s5
# Step 4: Verify the action took effect
agentchrome page snapshot
```

Always snapshot before interacting — UIDs are assigned dynamically and change when the page updates.

### Data Extraction Loop

Use this loop when navigating to pages and extracting content:

```
navigate → wait → snapshot/text → extract
```

```sh
# Step 1: Navigate to the page
agentchrome navigate https://example.com --wait-until networkidle
# Step 2: Wait is handled by --wait-until above
# Step 3: Extract the content
agentchrome page text
# Or for structured data:
agentchrome page snapshot
```

## Efficiency Tips

- **Use `form fill-many` for batch form filling.** A single `form fill-many` call is faster than multiple `form fill` calls and reduces round-trips.
- **Use `--wait-until` to avoid race conditions.** After navigation, use `--wait-until networkidle` or `--wait-until load` to ensure the page is ready before interacting. This prevents "element not found" errors from acting on a partially loaded page.
- **Choose the right inspection command:**
  - `page snapshot` — returns the accessibility tree with UIDs. Use this when you need to interact with elements.
  - `page text` — returns visible text content. Use this for simple content extraction when you don't need UIDs.
  - `page screenshot --file shot.png` — captures a visual screenshot. Use this for visual debugging.
- **Minimize round-trips.** Combine related operations rather than making many small calls. For example, use `form fill-many` instead of multiple `form fill` calls.
- **Use `--timeout` to prevent hangs.** Set a timeout on long-running commands so they fail fast instead of blocking indefinitely.
- **Use `--include-snapshot` on interaction commands.** Commands like `interact click`, `form fill`, and `form fill-many` support `--include-snapshot` to return the updated accessibility tree in the same response, saving a separate `page snapshot` call.

## Error Handling for AI Agents

### Exit Codes

AgentChrome uses structured exit codes so agents can programmatically detect error types:

| Exit Code | Name | Description | Recovery Strategy |
|-----------|------|-------------|-------------------|
| 0 | Success | Command completed successfully | — |
| 1 | GeneralError | Invalid arguments or internal failure | Check command syntax with `--help` |
| 2 | ConnectionError | Chrome not running or session expired | Re-run `agentchrome connect --launch --headless` |
| 3 | TargetError | Tab not found or no page targets | Run `agentchrome tabs list` to find valid targets |
| 4 | TimeoutError | Navigation or command timeout | Increase `--timeout` value or check if the page is loading |
| 5 | ProtocolError | CDP protocol failure | Disconnect and reconnect: `agentchrome connect --disconnect` then `agentchrome connect --launch --headless` |

### Common Failure Modes

**Connection refused** (exit code 2):
Chrome is not running or the debug port is unavailable. Recovery:
```sh
agentchrome connect --launch --headless
```

**Element not found** (exit code 1):
The target UID or selector does not match any element. This usually happens when the page has updated since the last snapshot. Recovery:
```sh
# Re-snapshot to get fresh UIDs
agentchrome page snapshot
# Then retry the interaction with the correct UID
```

**Timeout** (exit code 4):
A navigation or command exceeded the time limit. Recovery:
```sh
# Retry with a longer timeout
agentchrome navigate https://slow-page.example.com --timeout 30000
```

**Page not loaded** (exit code 1 or 4):
Commands that inspect the page may fail if called before the page finishes loading. Recovery:
```sh
# Navigate with an explicit wait strategy
agentchrome navigate https://example.com --wait-until networkidle
```

### Parsing Errors

Errors are written to stderr. In JSON output mode (`--json`), error details are included in the JSON response. Always check the exit code first, then parse stderr for details if the code is non-zero.

## Best Practices

- **Always `page snapshot` before interaction commands.** UIDs are dynamic — they change when the DOM updates. A fresh snapshot ensures you target the correct element.
- **Use JSON output for reliable parsing.** Pass `--json` (or `--pretty` for readable JSON) to get structured output that's easy for AI agents to parse. JSON is the default for most commands.
- **Check exit codes for error handling.** Use the exit code to determine the type of failure before parsing error messages.
- **Use `--timeout` flags.** Set timeouts on navigation and long-running commands to prevent indefinite hangs. Example: `agentchrome navigate https://example.com --timeout 10000`.
- **Prefer `form fill` over `interact type` for form inputs.** `form fill` sets the value directly via the DOM, which is more reliable than simulating keystrokes character by character with `interact type`.
- **Use `console follow` and `network follow` for debugging.** These commands stream real-time console messages and network requests, which is invaluable for diagnosing runtime errors and failed API calls. Use `--timeout` to auto-exit after a set duration.
- **Use `--include-snapshot` to reduce round-trips.** Interaction commands (`interact click`, `form fill`, etc.) support `--include-snapshot` to return the updated accessibility tree, saving a separate `page snapshot` call.
- **Use `page find` to locate elements by text or role.** Instead of scanning the full snapshot manually, use `page find "Submit" --role button` to locate specific elements quickly.

## Example Conversation

Below is a realistic multi-turn example of Claude Code using AgentChrome to debug a web application that has a broken login form.

---

**User:** The login form on our app isn't working. Can you check what's going on?

**Claude Code:** Let me connect to Chrome and inspect the login page.

```sh
agentchrome connect --launch --headless
```
```
{"status":"connected","ws_url":"ws://127.0.0.1:9222/devtools/page/ABC123"}
```

```sh
agentchrome navigate http://localhost:3000/login --wait-until networkidle
```
```
{"url":"http://localhost:3000/login","status":200}
```

```sh
agentchrome page snapshot
```
```
[s1] heading "Sign In"
[s2] textbox "Email" [focused]
[s3] textbox "Password"
[s4] button "Log In" [disabled]
[s5] link "Forgot password?"
```

The "Log In" button is disabled. Let me fill in the form fields and see if it enables.

```sh
agentchrome form fill-many '[{"uid": "s2", "value": "test@example.com"}, {"uid": "s3", "value": "password123"}]'
```
```
{"filled":2,"fields":[{"uid":"s2","status":"ok"},{"uid":"s3","status":"ok"}]}
```

```sh
agentchrome page snapshot
```
```
[s1] heading "Sign In"
[s2] textbox "Email" value="test@example.com"
[s3] textbox "Password" value="password123"
[s4] button "Log In" [disabled]
[s5] link "Forgot password?"
```

The button is still disabled even after filling both fields. Let me check for console errors.

```sh
agentchrome console read --errors-only
```
```
{"messages":[{"id":1,"type":"error","text":"Uncaught TypeError: Cannot read properties of null (reading 'addEventListener')","source":"http://localhost:3000/js/login.js","line":15}]}
```

Found it — there's a JavaScript error in `login.js` at line 15. The code is trying to call `addEventListener` on a null element, which means the form validation script isn't attaching its event listener. This is likely preventing the button from being enabled when fields are filled.

The bug is in `js/login.js` line 15. The script is probably looking for an element by an ID or selector that doesn't exist in the DOM. Want me to look at that file?

---

## Reference

- **Full command manifest:** `agentchrome capabilities` — outputs JSON describing every command, subcommand, flag, and exit code
- **Per-command examples:** `agentchrome examples <command>` — shows practical usage examples (e.g., `agentchrome examples navigate`)
- **Man pages:** `agentchrome man <command>` — displays the man page for any command
- **Help text:** `agentchrome <command> --help` — quick usage reference for any command or subcommand
