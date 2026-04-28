# Tasks: Network list filters and detail lookup lose captured request data

**Issue**: #285
**Date**: 2026-04-28
**Status**: Planning
**Author**: Codex (write-spec)

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| T001 | Fix network capture correlation, filtering, and list-to-get ID resolution | [ ] |
| T002 | Add BDD and focused regression coverage | [ ] |
| T003 | Verify no regressions with focused tests and live headless Chrome smoke | [ ] |

---

### T001: Fix the Defect

**File(s)**: `src/network.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] Network event correlation merges request, response, loading-finished, and loading-failed fragments by CDP `requestId` even when those fragments are drained out of lifecycle order.
- [ ] Completed requests populate `status`, `size`, `duration_ms`, and `timestamp` when CDP provides the underlying data or existing documented fallbacks apply.
- [ ] `network list --type document` filters over normalized lowercase resource types and includes document entries that appear in the unfiltered capture set.
- [ ] CLI numeric IDs are assigned after request normalization using a deterministic ordering for the capture set.
- [ ] `network list` persists enough active-target snapshot state for `network get <id-from-list>` to resolve the listed ID in the subsequent invocation.
- [ ] `network get` validates cached snapshot context, recaptures only when the snapshot is missing/stale, and returns `Network request <id> not found` only when the request is genuinely unavailable.
- [ ] The existing structured JSON error shape and typed exit code behavior are preserved for genuine missing IDs.
- [ ] No unrelated changes are made to `network follow`, console, performance, page, tab, or navigation command behavior.

**Notes**: Keep the fix in the network command module unless a tiny session-cache helper is needed. If cache persistence is added, use the existing `~/.agentchrome` ownership boundary and atomic-write style from `src/session.rs`; do not introduce a background daemon.

### T002: Add Regression Coverage

**File(s)**: `tests/features/285-network-list-filters-and-detail-lookup-lose-captured-request-data.feature`, `tests/bdd.rs`, `src/network.rs`, `tests/fixtures/285-network-list-detail.html`
**Type**: Create + Modify
**Depends**: T001
**Acceptance**:
- [ ] New Gherkin feature file exists with every scenario tagged `@regression`.
- [ ] Scenarios map 1:1 to AC1-AC4 in `requirements.md`.
- [ ] Chrome-dependent scenarios are tagged consistently with existing network regression scenarios.
- [ ] `tests/bdd.rs` registers the feature file using the repo's current Chrome-dependent filtering pattern, or implements live step bindings if the focused runner can provide Chrome.
- [ ] Focused unit tests in `src/network.rs` prove out-of-order response/finish fragments still populate summary metadata.
- [ ] Focused unit tests prove `document` type filtering uses normalized summary values and does not drop matching document requests.
- [ ] Focused unit tests prove an ID returned by the list snapshot can be resolved by the detail path, and a genuinely unavailable ID still emits the existing not-found error.
- [ ] A deterministic local fixture is added or reused so verification does not depend on `qaplayground.vercel.app` in CI.

### T003: Verify No Regressions

**File(s)**: none (verification only)
**Type**: Verify
**Depends**: T001, T002
**Acceptance**:
- [ ] `cargo fmt --check` exits 0.
- [ ] `cargo test --lib network` exits 0, or the nearest focused unit-test command for `src/network.rs` exits 0.
- [ ] `cargo test --test bdd -- --input tests/features/285-network-list-filters-and-detail-lookup-lose-captured-request-data.feature --fail-fast` exits 0, or the repository-supported equivalent focused BDD command exits 0.
- [ ] Existing network regression features continue to pass or remain correctly registered as Chrome-dependent documentation scenarios: `tests/features/network.feature`, `tests/features/102-fix-network-list-empty-array.feature`, `tests/features/116-fix-network-list-timestamps.feature`, and `tests/features/117-fix-network-list-size-zero.feature`.
- [ ] Manual smoke with a fresh debug binary and headless Chrome confirms:
  - `./target/debug/agentchrome network list --pretty` returns at least one completed request with non-null response metadata when available.
  - `./target/debug/agentchrome network list --type document --pretty` returns at least one `document` request.
  - `./target/debug/agentchrome network get <id-from-list> --pretty` exits 0 and returns `request`, `response`, and `timing` sections.
- [ ] `cargo clippy --all-targets` exits 0.
- [ ] No orphaned Chrome processes are left running after live-browser verification.

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Tasks are focused on the fix - no feature work
- [x] Regression test is included (T002)
- [x] Each task has verifiable acceptance criteria
- [x] No scope creep beyond the defect
- [x] File paths reference actual project structure (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #285 | 2026-04-28 | Initial defect tasks |
