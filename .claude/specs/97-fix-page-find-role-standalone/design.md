# Root Cause Analysis: page find --role does not work as standalone search criterion

**Issue**: #97
**Date**: 2026-02-15
**Status**: Draft
**Author**: Claude

---

## Root Cause

The `execute_find()` function in `src/page.rs` (line 513) validates that at least one of `query` (positional text argument) or `--selector` is provided. If neither is present, it returns an error before ever checking whether `--role` was supplied. This means `--role` alone is rejected even though it represents a valid, independent search criterion.

The downstream `search_tree()` function in `src/snapshot.rs` (lines 309-369) already handles this case correctly: when `query` is empty and a `role_filter` is provided, it matches all nodes of that role (line 346-348 treats an empty query as matching everything). The bug is entirely in the input validation gate, not in the search logic.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/page.rs` | 511-519 | `execute_find()` — input validation rejects `--role` without `query` or `--selector` |
| `src/page.rs` | 544-546 | Accessibility search path — already passes `query.as_deref().unwrap_or("")` which would work with role-only |
| `src/snapshot.rs` | 309-369 | `search_tree()` — already handles empty query + role filter correctly |

### Triggering Conditions

- User invokes `page find --role <role>` without a text query or `--selector`
- `args.query` is `None` and `args.selector` is `None`
- `args.role` is `Some(...)` but is never checked in the validation guard
- Validation returns error prematurely

---

## Fix Strategy

### Approach

Expand the validation guard in `execute_find()` to also accept `--role` as a sufficient search criterion. The condition should reject the command only when **none** of `query`, `--selector`, or `--role` is provided.

This is a single-line change to the `if` condition. No other code changes are required because `search_tree()` already handles the empty-query-with-role-filter case.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/page.rs` (line 513) | Change `args.query.is_none() && args.selector.is_none()` to `args.query.is_none() && args.selector.is_none() && args.role.is_none()` | Allows `--role` as a standalone search criterion while preserving the requirement that *something* must be specified |

### Blast Radius

- **Direct impact**: `execute_find()` in `src/page.rs` — only the validation condition changes
- **Indirect impact**: None — the accessibility search path (`search_tree()`) and CSS selector path are unaffected; both already handle the presence/absence of role correctly
- **Risk level**: Low — the change only loosens validation; no execution paths are altered

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Combined text+role search breaks | Low | AC2 regression test explicitly verifies this path |
| Running `page find` with no arguments at all stops producing an error | Low | The new condition still requires at least one of query, selector, or role; AC-covered by existing Gherkin scenario "Neither query nor selector provided" which should be updated to also omit `--role` |
| Error message becomes misleading | Low | Update the error message to mention `--role` as a valid alternative |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Add a clap arg group requiring at least one of query/selector/role | Moves validation to clap, which would produce a different error message format | The existing in-function validation is simpler and consistent with how other commands validate; a one-line fix is preferable to restructuring argument parsing |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
