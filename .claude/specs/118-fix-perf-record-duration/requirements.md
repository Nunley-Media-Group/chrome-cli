# Defect Report: perf record --duration reports incorrect duration_ms

**Issue**: #118
**Date**: 2026-02-16
**Status**: Approved
**Author**: Claude
**Severity**: Medium
**Related Spec**: `.claude/specs/22-performance-tracing/`

---

## Reproduction

### Steps to Reproduce

1. `chrome-cli connect --launch --headless`
2. `chrome-cli navigate https://www.google.com`
3. `chrome-cli perf record --duration 2000 --file /tmp/trace.json`
4. Observe `duration_ms` in the JSON output — it shows ~21 instead of ~2000
5. `chrome-cli perf record --reload --duration 3000 --file /tmp/trace2.json`
6. Observe `duration_ms` — it shows ~133 instead of ~3000

### Environment

| Factor | Value |
|--------|-------|
| **OS / Platform** | macOS Darwin 25.3.0 |
| **Version / Commit** | 0.1.0 (commit 112e231) |
| **Browser / Runtime** | HeadlessChrome/144.0.0.0 |

### Frequency

Always

---

## Expected vs Actual

| | Description |
|---|-------------|
| **Expected** | `duration_ms` in `perf record` output reflects the actual trace recording duration (approximately matching the `--duration` value) |
| **Actual** | `duration_ms` only measures the trace stop/collection overhead (21–133ms), regardless of the `--duration` value |

### Error Output

```json
// With --duration 2000:
{"file":"/tmp/trace.json","duration_ms":21,"size_bytes":...,"vitals":{...}}
// Expected duration_ms ≈ 2000
```

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Duration reflects actual recording time

**Given** a connected Chrome session on a loaded page
**When** I run `chrome-cli perf record --duration 2000 --file /tmp/trace.json`
**Then** the `duration_ms` in the JSON output is approximately 2000 (within ±500ms tolerance)

### AC2: Duration includes reload time when applicable

**Given** a connected Chrome session on a loaded page
**When** I run `chrome-cli perf record --reload --duration 3000 --file /tmp/trace.json`
**Then** the `duration_ms` in the JSON output reflects the total recording time (approximately ≥ 3000ms)

### AC3: Existing perf record output structure is preserved

**Given** a connected Chrome session on a loaded page
**When** I run `chrome-cli perf record --duration 1000 --file /tmp/trace.json`
**Then** the JSON output contains `file`, `duration_ms`, `size_bytes`, and `vitals` fields
**And** the trace file is written successfully

---

## Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1 | Start the duration timer before the recording begins (before `Tracing.start`), not after it ends (inside `stop_and_collect`) | Must |
| FR2 | Pass the recorded duration from `execute_record` into `stop_and_collect` so it reports the actual elapsed time | Should |

---

## Out of Scope

- Changes to trace analysis or vitals extraction
- Changes to the trace file format
- Changes to `perf vitals` or `perf analyze` commands
- Refactoring beyond the minimal timer relocation

---

## Validation Checklist

Before moving to PLAN phase:

- [x] Reproduction steps are repeatable and specific
- [x] Expected vs actual behavior is clearly stated
- [x] Severity is assessed
- [x] Acceptance criteria use Given/When/Then format
- [x] At least one regression scenario is included (AC3)
- [x] Fix scope is minimal — no feature work mixed in
- [x] Out of scope is defined
