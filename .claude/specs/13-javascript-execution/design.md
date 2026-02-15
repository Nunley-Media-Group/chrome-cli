# Design: JavaScript Execution in Page Context

**Issue**: #13
**Date**: 2026-02-12
**Status**: Draft
**Author**: Claude (writing-specs)

---

## Overview

This feature adds the `js exec` subcommand to execute arbitrary JavaScript in the browser page context. It exposes two CDP methods — `Runtime.evaluate` for expressions and `Runtime.callFunctionOn` for function calls with element context — through a unified CLI interface. The implementation follows the same layered patterns as existing commands (`page`, `perf`, `tabs`): parse CLI args, resolve connection, create a managed CDP session, execute CDP commands, and format output as JSON.

The key design decisions are: (1) using `Runtime.evaluate` with `awaitPromise: true` by default, (2) using `Runtime.callFunctionOn` with `DOM.resolveNode` for the `--uid` element context feature, (3) supporting code input from positional argument, `--file`, or stdin, and (4) capturing console messages via `Runtime.consoleAPICalled` events.

---

## Architecture

### Component Diagram

```
CLI Layer (cli/mod.rs)
  └── JsArgs → JsCommand::Exec(JsExecArgs)
        ↓
Command Layer (js.rs)       ← NEW FILE
  └── execute_js() → execute_exec()
        ↓
Connection Layer (connection.rs)  ← existing
  └── resolve_connection() → resolve_target() → ManagedSession
        ↓
CDP Layer (cdp/client.rs)     ← existing
  ├── Runtime.evaluate({ expression, awaitPromise, returnByValue })
  ├── Runtime.callFunctionOn({ functionDeclaration, objectId })
  ├── Runtime.consoleAPICalled (event subscription)
  └── DOM.resolveNode({ backendNodeId })
        ↓
Chrome Browser
  └── Executes JS, returns result
```

### Data Flow

```
1. User runs: chrome-cli js exec <CODE> [--file PATH] [--uid UID] [--no-await] [--timeout MS] [--max-size N]
2. CLI layer parses args into JsExecArgs
3. Resolve code source:
   a. If --file: read file contents
   b. If code is "-": read stdin
   c. Otherwise: use positional argument directly
4. Command layer resolves connection and target tab (standard setup_session)
5. Creates CdpSession via Target.attachToTarget
6. Enables Runtime domain (via ManagedSession.ensure_domain)
7. Subscribe to Runtime.consoleAPICalled for console capture
8. Branch on --uid:
   a. Without --uid: Runtime.evaluate with expression
   b. With --uid:
      i.   Read snapshot state → resolve UID to backendNodeId
      ii.  Enable DOM domain
      iii. DOM.resolveNode({ backendNodeId }) → get remote objectId
      iv.  Runtime.callFunctionOn({ functionDeclaration, objectId, arguments: [objectId] })
9. Handle result:
   - Success → JsExecResult { result, type, console, truncated }
   - Exception → structured error with error message + stack trace
   - Timeout → timeout error
10. Apply --max-size truncation if result exceeds limit
11. Format output via print_output (JSON / pretty JSON)
```

---

## API / Interface Changes

### New CLI Commands

| Command | Purpose |
|---------|---------|
| `chrome-cli js exec <CODE>` | Execute JavaScript in the page context |

### CLI Arguments (JsExecArgs)

| Argument | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `<code>` | `Option<String>` (positional) | Conditional | None | JavaScript code to execute; `-` reads stdin |
| `--file <PATH>` | `Option<PathBuf>` | No | None | Read JavaScript from a file |
| `--uid <UID>` | `Option<String>` | No | None | Element UID from snapshot; function receives element as first arg |
| `--no-await` | `bool` (flag) | No | `false` | Do not await promise results |
| `--timeout <MS>` | `Option<u64>` | No | Global timeout | Execution-specific timeout override |
| `--max-size <BYTES>` | `Option<usize>` | No | None (unlimited) | Truncate results exceeding this size |

Global flags `--tab`, `--json`, `--pretty`, `--plain`, `--host`, `--port`, `--ws-url` all apply as usual.

**Mutual exclusion**: `<code>` and `--file` are mutually exclusive. One of `<code>` or `--file` is required.

### Output Schema

**JSON mode** (success):

```json
{
  "result": "Example Domain",
  "type": "string"
}
```

With console output:

```json
{
  "result": 42,
  "type": "number",
  "console": [
    { "level": "log", "text": "hello" }
  ]
}
```

With truncation:

```json
{
  "result": "xxxxxxxxxx...",
  "type": "string",
  "truncated": true
}
```

**Error output** (stderr):

```json
{
  "error": "ReferenceError: foo is not defined",
  "stack": "ReferenceError: foo is not defined\n    at <anonymous>:1:1",
  "code": 1
}
```

### Errors

| Condition | Error Message | Exit Code |
|-----------|---------------|-----------|
| JS exception | `"JavaScript execution failed: {description}"` | `GeneralError` (1) |
| File not found | `"Script file not found: {path}"` | `GeneralError` (1) |
| File read error | `"Failed to read script file: {path}: {error}"` | `GeneralError` (1) |
| UID not found | Existing `uid_not_found` | `GeneralError` (1) |
| No snapshot | `"No snapshot state found. Run 'chrome-cli page snapshot' first."` | `GeneralError` (1) |
| No code provided | `"No JavaScript code provided. Specify code as argument, --file, or pipe via stdin."` | `GeneralError` (1) |
| No connection | Existing `no_session` / `no_chrome_found` | `ConnectionError` (2) |
| Tab not found | Existing `target_not_found` | `TargetError` (3) |
| Timeout | Existing `command_timeout` from CDP layer | `TimeoutError` (4) |

---

## New Files and Modifications

### New Files

| File | Purpose |
|------|---------|
| `src/js.rs` | JavaScript execution command implementation (dispatcher, exec logic, output types, helpers) |

### Modified Files

| File | Change |
|------|--------|
| `src/cli/mod.rs` | Add `JsArgs`, `JsCommand`, `JsExecArgs` structs; change `Js` variant to `Js(JsArgs)` |
| `src/main.rs` | Add `mod js;` import; wire `Command::Js(args)` to `js::execute_js()` |
| `src/error.rs` | Add `js_execution_failed()`, `script_file_not_found()`, `script_file_read_failed()`, `no_js_code()` helper constructors |

### No Changes Needed

| Component | Why |
|-----------|-----|
| `src/cdp/*` | `Runtime.evaluate` and event subscription already work; `Runtime.callFunctionOn` uses the same `send_command` pattern |
| `src/connection.rs` | `resolve_connection`, `resolve_target`, `ManagedSession` all reusable as-is |
| `src/snapshot.rs` | `read_snapshot_state()` already exists for UID resolution |
| `src/lib.rs` | No new public modules (js.rs is a binary-only module like page.rs) |

---

## JavaScript Execution Strategies

### Strategy 1: Expression evaluation (no --uid)

Uses `Runtime.evaluate`:

```json
{
  "expression": "<user code>",
  "returnByValue": true,
  "awaitPromise": true,
  "generatePreview": true
}
```

- `returnByValue: true` ensures we get the serialized value, not a remote object reference
- `awaitPromise: true` (default) waits for promises; disabled by `--no-await`
- `generatePreview: true` provides useful string representations for complex objects

### Strategy 2: Element context execution (--uid)

Uses `DOM.resolveNode` + `Runtime.callFunctionOn`:

```
Step 1: Read snapshot state → get backendNodeId for UID
Step 2: DOM.resolveNode({ backendNodeId }) → get objectId (remote object reference)
Step 3: Runtime.callFunctionOn({
  functionDeclaration: "<user code>",
  objectId: <resolved objectId>,
  arguments: [{ objectId: <resolved objectId> }],
  returnByValue: true,
  awaitPromise: true
})
```

The user provides a function like `(el) => el.textContent`. The resolved DOM element is passed as the first argument. `callFunctionOn` is the correct CDP method for this because it provides the object context.

### Console Capture

Before executing user code, subscribe to `Runtime.consoleAPICalled` events:

```
1. managed.subscribe("Runtime.consoleAPICalled")
2. Execute user code
3. Collect any console events received during execution
4. Include in output as "console" array
```

Each console entry: `{ "level": "log|warn|error|info", "text": "<message>" }`.

### Result Truncation (--max-size)

After receiving the result, check its serialized JSON size against `--max-size`:

```
1. Serialize result to JSON string
2. If byte length > max_size:
   a. Truncate the serialized string to max_size bytes
   b. Set truncated = true in output
3. Otherwise: use full result, omit truncated field
```

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: `Runtime.evaluate` only** | Use `evaluate` for all cases, wrap element into expression | Simple, single code path | Cannot pass live DOM element references; would need to re-query by selector | Rejected — cannot reliably pass element context |
| **B: `Runtime.evaluate` + `callFunctionOn`** | Use `evaluate` for expressions, `callFunctionOn` for `--uid` | Clean separation, correct semantics for element context | Two code paths | **Selected** — matches CDP design intent |
| **C: Wrap user code in IIFE** | Always wrap in `(function() { return <code>; })()` | Uniform execution | Breaks expressions like `document.title`, changes semantics | Rejected — too opinionated |

---

## Security Considerations

- [x] **No sandboxing needed**: This is a power-user tool; the user controls the browser and the code they execute. The issue explicitly states "no sandboxing needed."
- [x] **Local CDP only**: CDP connections are localhost-only by default (per `tech.md`), so remote code execution is not a concern.
- [x] **File access**: `--file` reads local files, which is standard CLI behavior. No path traversal concern since the user controls the argument.
- [x] **stdin**: Reading from stdin is standard Unix pipeline behavior.

---

## Performance Considerations

- [x] **Single CDP round-trip** for expressions (Runtime.evaluate)
- [x] **Two CDP round-trips** for element context (DOM.resolveNode + Runtime.callFunctionOn)
- [x] **`returnByValue: true`** avoids an extra round-trip to fetch remote objects
- [x] **Console subscription** is lightweight — events arrive asynchronously during execution
- [x] **Truncation** happens client-side after receiving the full result (no way to limit at CDP level)

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output types | Unit | Serialization of `JsExecResult`, `JsExecError` (JSON fields, skip_serializing_if) |
| Error helpers | Unit | New error constructors produce correct messages and exit codes |
| Code resolution | Unit | Positional arg, --file, stdin, mutual exclusion |
| Result type mapping | Unit | All 7 JS types → correct `type` string |
| Truncation logic | Unit | Result exceeds --max-size, truncated flag set |
| Feature | BDD (Gherkin) | All 16 acceptance criteria as scenarios |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Large JS results cause memory pressure | Low | Med | `--max-size` truncation; `returnByValue: true` streams result directly |
| `DOM.resolveNode` fails for stale snapshots | Med | Low | Clear error message: "Run `page snapshot` first" |
| Console capture races (messages arrive after result) | Low | Low | Small delay or drain after receiving result; console messages are best-effort |
| Stdin blocks indefinitely if no data piped | Low | Med | Document that `-` reads until EOF; bounded by global `--timeout` |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] No state management changes needed
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
