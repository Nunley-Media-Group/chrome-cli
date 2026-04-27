# Root Cause Analysis: Fix frame auto selector targeting in DOM commands

**Issue**: #275
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (write-spec)

---

## Root Cause

`dom select` accepts a selector argument, but `src/dom.rs::execute_select` resolves the optional frame with `resolve_optional_frame(&client, &mut managed, frame, None)`. When the user passes `--frame auto`, `src/output.rs::resolve_optional_frame` parses that value as `FrameArg::Auto` and calls `src/frame.rs::resolve_frame_auto` with `uid.unwrap_or_default()`. Because `dom select` supplied `None`, the target hint becomes the empty string.

`src/frame.rs::resolve_frame_auto` currently searches only the UID-oriented path: it checks a persisted snapshot UID map, then iterates frames and builds transient accessibility UID maps. It never evaluates a CSS selector inside candidate frames. For `dom --frame auto select body`, the frame resolver therefore fails with `AppError::element_not_in_any_frame()` before `execute_select` can run its selector query in the resolved child frame.

The original iframe targeting spec required `--frame auto` to support a UID or selector argument. The implementation covers UID-driven commands such as `interact --frame auto click <uid>`, but the DOM selector path does not pass enough target information into the auto resolver, and the resolver has no selector-search branch.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/dom.rs` | 759-920 | `execute_select` resolves `--frame auto` before running CSS/XPath selection, but passes no target hint |
| `src/output.rs` | 215-268 | `resolve_optional_frame` accepts an optional target hint and calls `resolve_frame_auto` for `--frame auto` |
| `src/frame.rs` | 527-582 | `resolve_frame_auto` searches UID maps only and returns `Element not found in any frame` when no UID matches |
| `src/interact.rs` | 1772-1787, 2185-2198 | UID-targeted interact commands pass `Some(&args.target)` and must keep working |
| `tests/features/iframe-frame-targeting.feature` | 157-169 | Existing AC15/AC16 cover UID auto frame detection, not DOM selector auto detection |

### Triggering Conditions

- The command is in the DOM command group.
- The command is `dom select` with a selector target argument.
- The user passes `--frame auto`.
- The matching element exists in a child frame or the expected behavior depends on scanning frames.
- The target is not a UID present in the snapshot state or transient accessibility tree.

---

## Fix Strategy

### Approach

Extend frame auto resolution to accept a target hint that can be either a UID or a selector, and make `dom select` pass its selector argument when `--frame auto` is used. Keep the existing UID path unchanged for interact/form behavior, and add a selector branch that tests candidate frames in document order using the same frame context resolution used for explicit `--frame <index>`.

The selector branch should be deliberately narrow. It should support `dom select` CSS selector targets and the existing `css:` prefix convention. XPath auto-search is out of scope unless the implementation can route it through the same command-local selector evaluation without widening the defect fix. If no candidate frame contains the selector, the branch returns the existing `AppError::element_not_in_any_frame()` so stderr JSON and exit code remain stable.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/frame.rs` | Introduce a typed auto-frame target, or equivalent structured branch, that distinguishes UID targets from selector targets. | Prevents selector strings from being treated as UID-only input while preserving existing UID semantics. |
| `src/frame.rs` | Add selector-based auto search that iterates `list_frames()` in document order, resolves each candidate frame to `FrameContext`, and checks whether the selector exists in that frame. | Implements the iframe targeting contract for selector-bearing DOM commands. |
| `src/output.rs` | Update `resolve_optional_frame` or add a sibling helper so callers can request selector-based auto resolution without breaking current UID callers. | Centralizes frame parsing while preserving the existing shared helper contract. |
| `src/dom.rs` | Pass `Some(&args.selector)` or the structured selector target when resolving the frame for `DomCommand::Select`. | Supplies the target hint that `--frame auto` requires. |
| `tests/features/275-fix-frame-auto-selector-targeting-in-dom-commands.feature` | Add regression scenarios for DOM selector auto-search, explicit frame targeting, UID auto targeting, and the no-match error. | Ensures the defect cannot recur and adjacent behavior remains covered. |
| `tests/bdd.rs` | Register the new regression feature following the existing Chrome-dependent BDD convention for frame/DOM scenarios. | Keeps the feature file discoverable during `cargo test --test bdd`; Chrome-dependent scenarios may be filtered out if the runner cannot launch Chrome. |
| `tests/fixtures/iframe-frame-targeting.html` | Reuse the existing iframe fixture unless implementation needs a smaller defect-specific fixture. | The fixture already contains the required child-frame body and iframe-owned elements. |

### Blast Radius

- **Direct impact**: `src/frame.rs`, `src/output.rs`, `src/dom.rs`.
- **Indirect impact**: UID-based `--frame auto` callers in `src/interact.rs`, `src/form.rs`, and any other commands using `resolve_optional_frame(..., Some(uid))`.
- **Risk level**: Medium - the fix touches shared frame-resolution plumbing. The change should be constrained by a typed target representation and regression tests for both selector and UID paths.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| UID auto frame detection regresses because target interpretation changes | Medium | Keep the UID branch behavior and snapshot hint fast path intact; add AC3 regression coverage. |
| Selector auto-search returns the main frame when the intended target is in a child frame | Medium | Define and test document-order behavior explicitly; ensure frame context is included so callers can see the selected frame. |
| Same-origin iframe CSS queries use the wrong CDP document root | Medium | Reuse existing same-origin frame query helpers that evaluate selectors in the frame execution context. |
| Cross-origin/OOPIF frame selector checks fail differently from explicit `--frame N` | Low | Resolve each candidate through the existing `resolve_frame_by_info` path and use the returned effective session. |
| Missing selector errors change shape or exit code | Low | Continue returning `AppError::element_not_in_any_frame()` when all frames are exhausted. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Pass the selector string into the current UID-only resolver without type changes | Minimal call-site change only | Ambiguous target semantics make it easy to treat selectors as UIDs and miss future selector-specific behavior. |
| Implement DOM selector frame scanning entirely inside `execute_select` | Avoids changing shared frame resolver APIs | Duplicates frame enumeration and resolution logic that already belongs to `src/frame.rs` / `src/output.rs`. |
| Require users to pass explicit `--frame N` for DOM selectors | Keeps implementation unchanged | Violates the iframe targeting acceptance criteria and the reported expected behavior. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal - focused on selector-aware auto frame resolution
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #275 | 2026-04-27 | Initial defect report |
