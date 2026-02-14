# Design: Console Message Reading with Filtering

**Issue**: #18
**Date**: 2026-02-14
**Status**: Approved
**Author**: Claude (writing-specs)

---

## Overview

This feature adds a `console` command group with two subcommands: `read` (list/detail) and `follow` (real-time streaming). The implementation creates a new `src/console.rs` module following the established command module pattern (similar to `dialog.rs`, `js.rs`). The console command uses the CDP `Runtime` domain — specifically `Runtime.consoleAPICalled` events — to collect and stream console messages with filtering, pagination, and navigation-aware collection.

The `read` subcommand enables the Runtime domain, evaluates JavaScript to trigger console collection, and returns collected messages. The `follow` subcommand subscribes to `Runtime.consoleAPICalled` events and streams them in real-time until interrupted or timed out.

A key design decision is to collect messages by subscribing to `Runtime.consoleAPICalled` events from the CDP session. For `console read`, the CLI enables the Runtime domain and collects events that have accumulated. For `console follow`, events are streamed as they arrive. Message arguments are formatted by extracting `value` or `description` from CDP `RemoteObject` entries, following the existing pattern in `src/js.rs:extract_console_entries()`.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────┐
│                    CLI Layer (src/cli/mod.rs)             │
├──────────────────────────────────────────────────────────┤
│  Command::Console(ConsoleArgs)                           │
│  ConsoleCommand::Read(ReadArgs) | Follow(FollowArgs)     │
│  ReadArgs: MSG_ID, --type, --errors-only, --limit,       │
│            --page, --include-preserved, --tab             │
│  FollowArgs: --type, --errors-only, --timeout, --tab     │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│              Command Layer (src/console.rs)               │
├──────────────────────────────────────────────────────────┤
│  execute_console()        [dispatcher]                   │
│  ├── setup_session()      [reuse pattern]                │
│  ├── execute_read()       [list or detail mode]          │
│  │   ├── collect_messages()     [gather from CDP]        │
│  │   ├── filter_messages()      [by type]                │
│  │   ├── paginate_messages()    [limit + page]           │
│  │   └── format_message_args()  [RemoteObject → text]    │
│  └── execute_follow()     [streaming mode]               │
│      ├── subscribe to Runtime.consoleAPICalled           │
│      ├── filter and print messages as they arrive        │
│      └── exit on timeout or Ctrl+C                       │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                CDP Layer (src/cdp/)                       │
├──────────────────────────────────────────────────────────┤
│  Runtime.enable           → Enable console event capture │
│  Runtime.consoleAPICalled → Console message events       │
│  Runtime.evaluate         → Trigger pending console      │
│  Page.enable              → Navigation tracking          │
│  Page.frameNavigated      → Navigation boundary events   │
└──────────────────────────────────────────────────────────┘
```

### Data Flow — `console read`

```
1. User runs: chrome-cli console read [OPTIONS]
2. Clap parses args into ConsoleArgs → ConsoleCommand::Read(ReadArgs)
3. execute_read() sets up CDP session via setup_session()
4. Enable "Runtime" domain (and "Page" domain if --include-preserved)
5. Subscribe to "Runtime.consoleAPICalled" events
6. Drain collected events from the channel (non-blocking)
7. Format each event's args into text representation
8. Apply type filter (--type or --errors-only)
9. Apply pagination (--limit and --page)
10. If MSG_ID specified, find message by index and return detail view
11. Output as JSON array or plain text
```

### Data Flow — `console follow`

```
1. User runs: chrome-cli console follow [OPTIONS]
2. Clap parses args into ConsoleArgs → ConsoleCommand::Follow(FollowArgs)
3. execute_follow() sets up CDP session via setup_session()
4. Enable "Runtime" domain
5. Subscribe to "Runtime.consoleAPICalled" events
6. Loop: await next event (with optional timeout)
   a. Format message from event params
   b. Apply type filter
   c. If passes filter, print to stdout (one JSON object per line)
   d. Track whether any error-level messages seen
7. Exit on: timeout expired, Ctrl+C (SIGINT), or connection closed
8. Return non-zero exit code if any error-level messages were seen
```

---

## API / Interface Changes

### New CLI Command Group

| Command | Purpose |
|---------|---------|
| `chrome-cli console read [OPTIONS] [MSG_ID]` | List or retrieve console messages |
| `chrome-cli console follow [OPTIONS]` | Stream console messages in real-time |

### ConsoleArgs / ConsoleCommand (Clap)

```
console
├── read [MSG_ID]
│   ├── --type <TYPES>          # Comma-separated: log,error,warn,info,debug,dir,table,trace,assert,count,timeEnd
│   ├── --errors-only           # Shorthand for --type error,assert
│   ├── --limit <N>             # Max messages (default: 50)
│   ├── --page <N>              # Pagination page (0-based, default: 0)
│   ├── --include-preserved     # Include messages from previous navigations
│   └── --tab <ID>              # Target specific tab
└── follow
    ├── --type <TYPES>          # Filter streamed messages by type
    ├── --errors-only           # Shorthand for --type error,assert
    ├── --timeout <MS>          # Auto-exit after N milliseconds
    └── --tab <ID>              # Target specific tab
```

### Conflict Groups

| Flag | Conflicts With |
|------|----------------|
| `--type` | `--errors-only` |
| `--errors-only` | `--type` |

### Request / Response Schemas

#### `console read` — List Mode

**Output (success — JSON):**
```json
[
  {
    "id": 0,
    "type": "log",
    "text": "Hello world",
    "timestamp": "2026-02-14T12:00:00.000Z",
    "url": "https://example.com/script.js",
    "line": 42,
    "column": 5
  }
]
```

#### `console read <MSG_ID>` — Detail Mode

**Output (success — JSON):**
```json
{
  "id": 0,
  "type": "error",
  "text": "Uncaught TypeError: Cannot read property 'foo' of null",
  "timestamp": "2026-02-14T12:00:00.000Z",
  "url": "https://example.com/script.js",
  "line": 42,
  "column": 5,
  "args": [
    {"type": "string", "value": "Uncaught TypeError: Cannot read property 'foo' of null"}
  ],
  "stackTrace": [
    {"file": "https://example.com/script.js", "line": 42, "column": 5, "functionName": "handleClick"},
    {"file": "https://example.com/app.js", "line": 100, "column": 10, "functionName": ""}
  ]
}
```

#### `console follow` — Stream Mode

**Output (one JSON object per line):**
```
{"type": "log", "text": "Page loaded", "timestamp": "2026-02-14T12:00:00.000Z"}
{"type": "error", "text": "Failed to fetch", "timestamp": "2026-02-14T12:00:01.000Z"}
```

**Errors:**

| Code / Type | Condition |
|-------------|-----------|
| `ExitCode::GeneralError` | Message ID not found, invalid type filter |
| `ExitCode::ConnectionError` | CDP connection lost during follow |
| `ExitCode::GeneralError` (exit 1) | `console follow` saw error-level messages |
| Clap arg error | `--type` and `--errors-only` used together |

---

## Database / Storage Changes

None. Console messages are transient — they live only in the CDP session's event stream.

---

## State Management

### In-Memory Message Collection

For `console read`, messages are collected from CDP events into a `Vec<ConsoleMessage>`:

```rust
struct ConsoleMessage {
    id: usize,
    msg_type: String,         // "log", "error", "warn", etc.
    text: String,             // Formatted from args
    timestamp: f64,           // CDP timestamp (epoch seconds)
    url: String,              // Source URL
    line: u32,                // Source line
    column: u32,              // Source column
    args: Vec<Value>,         // Raw CDP RemoteObject args (detail mode only)
    stack_trace: Vec<StackFrame>, // Stack frames (detail mode only)
    navigation_id: Option<u32>,   // Navigation boundary tracking
}

struct StackFrame {
    file: String,
    line: u32,
    column: u32,
    function_name: String,
}
```

### Navigation Awareness

To support `--include-preserved`:
- Subscribe to `Page.frameNavigated` events
- Track navigation boundaries by tagging messages with a navigation counter
- By default, only return messages from the current navigation
- With `--include-preserved`, return messages from up to the last 3 navigations

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Poll with Runtime.evaluate** | Periodically evaluate JS to read a console buffer | Simple, no event subscription | Misses messages between polls, no real-time streaming, requires page-side JS injection | Rejected — unreliable |
| **B: Runtime.consoleAPICalled events** | Subscribe to CDP events for console messages | Real-time, complete capture, no page modification, matches MCP approach | Requires event subscription lifecycle management | **Selected** |
| **C: Log domain (Log.entryAdded)** | Use the CDP Log domain | Captures browser-level logs | Different message format, doesn't capture console.log() calls | Rejected — wrong abstraction level |

**Decision**: Use `Runtime.consoleAPICalled` events (Option B). This matches the MCP server's approach, captures all console API calls (log, warn, error, etc.), provides full argument and stack trace data, and supports real-time streaming naturally.

---

## Security Considerations

- [x] **Input Validation**: Message types validated against known CDP console types. Limit/page validated by clap as positive integers.
- [x] **No code injection**: No user-supplied JavaScript is executed. Only CDP protocol commands are used.
- [x] **Sensitive Data**: Console messages may contain sensitive data from the target page. This is inherent to the debugging use case — no additional exposure beyond what Chrome DevTools shows.

---

## Performance Considerations

- [x] **Event buffering**: CDP events are buffered in the channel. For `console read`, drain with a short timeout (100ms) to capture pending events.
- [x] **Pagination**: Apply filtering and pagination in-memory after collection. With the default 50-message limit, memory usage is minimal.
- [x] **Streaming**: `console follow` processes events one at a time with no buffering — minimal memory footprint.
- [x] **Navigation tracking**: Lightweight — just a counter incremented on `Page.frameNavigated`.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Output types | Unit | `ConsoleMessage` serialization, `StackFrame` serialization |
| Type filtering | Unit | Filter logic for type lists and errors-only |
| Pagination | Unit | Limit and page offset calculations |
| Arg formatting | Unit | `RemoteObject` → text conversion (string, number, object, undefined) |
| CLI args | BDD (no Chrome) | `--help` output, `--type`/`--errors-only` conflict |
| Read list | BDD (Chrome) | List messages, filter by type, pagination |
| Read detail | BDD (Chrome) | Single message with stack trace, invalid ID error |
| Follow stream | BDD (Chrome) | Stream messages, filter, timeout, exit code |
| Plain text | BDD (Chrome) | `--plain` output formatting |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Messages missed if Runtime.enable is called after page scripts run | Medium | Medium | Document that `console read` captures messages from the point of connection. For pre-existing messages, recommend using `console follow` with page reload. |
| `console follow` hangs if no messages arrive and no timeout | Low | Low | Document `--timeout` usage. Ctrl+C always works as interrupt. |
| Large message volumes overwhelm stdout in follow mode | Low | Low | Messages are printed one per line. Shell piping handles backpressure naturally. |
| Stack trace formatting varies across Chrome versions | Low | Low | Use defensive parsing with fallback to empty frames |

---

## Implementation Notes

### Key CDP Events

| Event | Purpose |
|-------|---------|
| `Runtime.consoleAPICalled` | Console message event — contains `type`, `args[]`, `stackTrace`, `timestamp` |
| `Page.frameNavigated` | Navigation boundary tracking for `--include-preserved` |

### Argument Formatting

CDP `Runtime.consoleAPICalled` events provide args as an array of `RemoteObject`:
```json
{"type": "string", "value": "hello"}
{"type": "number", "value": 42}
{"type": "object", "className": "Object", "description": "Object"}
{"type": "undefined"}
```

Format strategy (matching existing `extract_console_entries()` in `src/js.rs`):
1. If `value` is a string → use directly
2. If `description` exists → use as text
3. Otherwise → JSON-serialize `value`
4. Join all args with space separator

### Console Message Types

Valid types from CDP `Runtime.consoleAPICalled`:
`log`, `debug`, `info`, `error`, `warning` (mapped to `warn`), `dir`, `dirxml`, `table`, `trace`, `clear`, `startGroup`, `startGroupCollapsed`, `endGroup`, `assert`, `profile`, `profileEnd`, `count`, `timeEnd`

The CLI exposes a simplified subset: `log`, `error`, `warn`, `info`, `debug`, `dir`, `table`, `trace`, `assert`, `count`, `timeEnd`.

### Exit Code for `console follow`

When `console follow` exits (via timeout or signal):
- Exit 0 if no error-level messages (`error`, `assert`) were seen
- Exit 1 if any error-level messages were seen
- This enables CI usage: `chrome-cli console follow --timeout 5000 || echo "Errors detected"`

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] No database/storage changes needed
- [x] State management approach is clear (in-memory event collection)
- [x] No new UI components (CLI-only)
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
