# Defect Report: Network list filters and detail lookup lose captured request data

**Issue**: #285
**Date**: 2026-04-28
**Status**: Draft
**Author**: Codex (write-spec)
**Severity**: High
**Related Spec**: `specs/feature-network-request-monitoring/`

---

## Reproduction

### Steps to Reproduce

1. Build the debug binary with `cargo build`.
2. Launch Chrome with `./target/debug/agentchrome connect --launch --headless`.
3. Navigate a fresh tab with `./target/debug/agentchrome --port <port> tabs create https://qaplayground.vercel.app/`.
4. Wait for content with `./target/debug/agentchrome --port <port> page wait --text 'QA AUTOMATION PRACTICE PLAYGROUND' --timeout 30000`.
5. Run `./target/debug/agentchrome --port <port> network list --pretty`.
6. Run `./target/debug/agentchrome --port <port> network list --type document --pretty`.
7. Run `./target/debug/agentchrome --port <port> network get <id-from-step-5> --pretty`.

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 arm64 |
| **Version / Commit** | agentchrome 1.60.0, commit 8dadf23 |
| **Browser / Runtime** | Headless Chrome via CDP |
| **Configuration** | Default network list/get behavior with a debug binary |

### Frequency

Always for the observed live regression sequence: the first list returns a partially populated document request, the document filter loses it, and detail lookup cannot resolve the listed ID.

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `network list` returns completed request summaries with response and timing metadata when CDP provides it, `network list --type document` includes document requests shown by the unfiltered list, and `network get <id>` resolves IDs returned by `network list` in the supported list/filter/get workflow. |
| **Actual** | The unfiltered list can show a document request with `status`, `size`, and `duration_ms` set to `null`; `network list --type document` can return `[]`; and `network get <id-from-list>` returns `Network request <id> not found`. |

### Error Output

`network list --pretty` after fresh navigation returned a partially captured document request:

```json
[
  {
    "id": 2,
    "method": "GET",
    "url": "https://qaplayground.vercel.app/",
    "status": null,
    "type": "document",
    "size": null,
    "duration_ms": null,
    "timestamp": "2026-04-28T12:24:11.082Z"
  }
]
```

`network list --type document --pretty` returned:

```json
[]
```

`network get 2 --pretty` returned:

```json
{"error":"Network request 2 not found","code":1}
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Document Filter Returns Matching Captured Requests

**Given** Chrome is connected with a page that has completed a document request
**When** I run `agentchrome network list --type document`
**Then** the output includes at least one request whose `type` is `document`
**And** the command exits 0

### AC2: Completed Requests Include Response Metadata

**Given** Chrome is connected with a page that has completed a request
**When** I run `agentchrome network list`
**Then** each completed request includes `status`, `size`, `duration_ms`, and `timestamp`
**And** those fields are non-null when CDP provided the response, transfer-size, timing, and wall-clock timestamp data

### AC3: Detail Lookup Resolves IDs Returned By List

**Given** `agentchrome network list` returns a request with id `<request-id>`
**When** I run `agentchrome network get <request-id>`
**Then** the command exits 0
**And** the output includes `request`, `response`, and `timing` sections for that request

### AC4: Regression Is Automatable

**Given** the fix is implemented
**When** the focused BDD feature for this defect and the existing network request monitoring feature are exercised against a headless Chrome session
**Then** the list, document filter, and list-to-get detail workflow pass without manual browser inspection

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Network event correlation must preserve response, loading-finished, and loading-failed data for a request even when per-event receivers deliver events to the collector out of request/response/finish order. | Must |
| FR2 | `network list --type <types>` must filter the same normalized resource-type values emitted by unfiltered `network list`; `document` entries shown by the unfiltered list must be included by `--type document`. | Must |
| FR3 | `network get <id>` must resolve IDs returned by `network list` for the same Chrome target workflow by preserving or reacquiring enough captured request state to build the detail response. | Must |
| FR4 | Completed request summaries must serialize `status`, `size`, `duration_ms`, and `timestamp` when CDP provides the corresponding data; `null` is reserved for genuinely unavailable values. | Must |
| FR5 | The regression must be covered by an automatable BDD feature and focused tests that prove list, filter, and get work as one workflow. | Must |

---

## Out of Scope

- Adding network interception, mutation, request blocking, or HAR export features.
- Changing `network follow` streaming semantics except for shared helpers needed to keep output contracts consistent.
- Changing unrelated console, performance, navigation, tab, or page command behavior.
- Guaranteeing response metadata when Chrome/CDP never emits the underlying response, transfer-size, timing, or wall-clock fields.
- Requiring a public website in CI when a deterministic local HTTP fixture can reproduce the workflow.
- Introducing a long-running background daemon for network capture.

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included
- [x] Fix scope is minimal - no feature work mixed in
- [x] Out of scope is defined

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #285 | 2026-04-28 | Initial defect report |
