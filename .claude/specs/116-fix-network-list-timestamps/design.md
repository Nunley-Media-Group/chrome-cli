# Root Cause Analysis: Network list timestamps showing 1970-01-01 instead of real wall-clock time

**Issue**: #116
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude

---

## Root Cause

The `timestamp_to_iso()` function in `src/network.rs` (lines 259-284) incorrectly assumes that CDP `Network.requestWillBeSent` timestamps are "seconds since Unix epoch." In reality, CDP Network domain timestamps are **monotonic clock values** — seconds since an arbitrary origin (typically browser startup), not wall-clock time.

When Chrome sends a monotonic timestamp like `62090.044` (seconds since browser startup), the function converts it as `62090.044 * 1000 = 62090044` milliseconds from Unix epoch, which yields `1970-01-01T17:14:50.044Z`. This explains the consistent `1970-01-01T17:XX:XX` pattern reported in the bug.

The `console.rs` module handles timestamps correctly because `Runtime.consoleAPICalled` provides timestamps as **milliseconds since Unix epoch** (true wall-clock time), which is a different convention from the Network domain. The two CDP domains use fundamentally different timestamp origins.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/network.rs` | 259-284 | `timestamp_to_iso()` — incorrectly treats monotonic seconds as epoch seconds |
| `src/network.rs` | 689 | Captures `timestamp` from `Network.requestWillBeSent` event params |
| `src/network.rs` | 730 | Captures `timestamp` from `Network.loadingFinished` event params |
| `src/network.rs` | 769 | Calls `timestamp_to_iso()` for list output |
| `src/network.rs` | 976 | Calls `timestamp_to_iso()` for detail output |
| `src/network.rs` | 1073 | Captures `timestamp` from streaming `requestWillBeSent` events |
| `src/network.rs` | 1119 | Captures `timestamp` from streaming `loadingFinished` events |
| `src/network.rs` | 1226 | Calls `timestamp_to_iso()` for streaming output |

### Triggering Conditions

- Any use of `chrome-cli network list`, `network get`, or `network follow` — the bug affects all network subcommands that display timestamps
- The monotonic timestamp value is small enough (typically 10k-200k seconds) that it maps to a date in early 1970 when treated as epoch seconds
- This was not caught before because the incorrect comment (`"seconds since epoch"`) masked the real semantics of CDP Network timestamps

---

## Fix Strategy

### Approach

Use the `wallTime` field from CDP `Network.requestWillBeSent` events. This field provides the wall-clock time as **seconds since Unix epoch** (floating-point) and is available since Chrome 89. Since `requestWillBeSent` is the event where the initial timestamp is captured, we simply read `wallTime` instead of `timestamp` for display purposes.

For `Network.loadingFinished` and `Network.loadingFailed` events (which do NOT have a `wallTime` field), compute a monotonic-to-epoch offset from the `requestWillBeSent` event's `wallTime` and `timestamp` (monotonic) fields: `offset = wallTime - timestamp`. Apply this offset to convert the monotonic `timestamp` in loading events to wall-clock time.

The `timestamp_to_iso()` function continues to receive epoch seconds — only the input values change. Duration calculations remain based on raw monotonic differences and are unaffected.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/network.rs` | Read `wallTime` from `requestWillBeSent` event params and store as the display timestamp; compute monotonic-to-epoch offset (`wallTime - timestamp`) for converting `loadingFinished` timestamps | Uses Chrome's own wall-clock value, the most accurate approach |
| `src/network.rs` | Update `timestamp_to_iso()` doc comment to clarify it expects epoch seconds | Corrects the misleading documentation |

### Blast Radius

- **Direct impact**: `src/network.rs` — the `timestamp_to_iso()` callers (lines 769, 976, 1226) and the event capture code (lines 689, 730, 1073, 1119)
- **Indirect impact**: None — `timestamp_to_iso()` is a private function within `network.rs` with no external callers. Console timestamps in `console.rs` use a separate `timestamp_to_iso()` function and are unaffected.
- **Risk level**: Low — the change is confined to timestamp conversion logic within a single module, and the formatter algorithm itself is unchanged

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Duration calculations (based on difference between two monotonic timestamps) could be affected if offset is applied inconsistently | Low | The offset is applied only at display time in `timestamp_to_iso()` calls; duration calculations use raw monotonic values (e.g., line 1217: `(end - req.timestamp) * 1000.0`) and remain unchanged |
| Console timestamps could be accidentally broken | Low | Console uses its own `timestamp_to_iso()` in `console.rs` — completely separate code path; AC3 regression test confirms |
| Offset calculation could be slightly inaccurate due to network latency between system clock capture and first CDP event | Low | Sub-second accuracy is sufficient for debugging timestamps; the offset is captured as close to the first event as possible |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Compute offset from `SystemTime::now()` at monitoring start | Capture system clock when monitoring starts, compute offset from first monotonic timestamp | Less accurate — network/processing delay between system clock capture and first CDP event introduces error; `wallTime` from CDP is directly authoritative |
| Add a chrono/time dependency | Use the `chrono` crate for timestamp handling | Over-engineered for this fix; the existing manual calendar math works fine and avoids adding a dependency |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
