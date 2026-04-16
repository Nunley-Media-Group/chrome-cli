# Design: Element-Targeted Scrolling for Inner Containers

**Issues**: #182
**Date**: 2026-04-16
**Status**: Draft
**Author**: Rich Nunley

---

## Overview

This feature adds dedicated `--selector` and `--uid` flags to the `interact scroll` command, providing ergonomic element-targeted scrolling without the `css:` prefix convention required by the existing `--container` flag. It also introduces scrollability validation that returns an error when the targeted element cannot scroll, replacing the current silent no-op behavior.

The implementation modifies two files: `src/cli/mod.rs` to add the new clap arguments with conflict rules, and `src/interact.rs` to add a scrollability check helper and wire the new flags into the existing container scroll code path. The existing `--container` flag is preserved for backward compatibility. A new `AppError` constructor is added to `src/error.rs` for the not-scrollable error case.

---

## Architecture

### Component Diagram

```
CLI Layer (src/cli/mod.rs)
  ScrollArgs struct
    +-- --selector <CSS>      [NEW] conflicts_with: uid, to_element, to_top, to_bottom
    +-- --uid <UID>           [NEW] conflicts_with: selector, to_element, to_top, to_bottom
    +-- --container <target>  [EXISTING] preserved for backward compat
    +-- --direction            [EXISTING] works with new flags
    +-- --amount               [EXISTING] works with new flags
    +-- --smooth               [EXISTING] works with new flags
        |
        v
Command Module (src/interact.rs)
  execute_scroll()
    +-- resolve --selector -> CSS querySelector -> backend_node_id
    +-- resolve --uid -> snapshot UID map -> backend_node_id
    +-- [NEW] check_element_scrollable(backend_node_id) -> error if not scrollable
    +-- get_container_scroll_position()     [EXISTING]
    +-- compute_scroll_delta()              [EXISTING]
    +-- dispatch_container_scroll()         [EXISTING]
    +-- wait_for_smooth_container_scroll()  [EXISTING]
        |
        v
Error Layer (src/error.rs)
  [NEW] AppError::element_not_scrollable(descriptor) -> JSON error on stderr
```

### Data Flow

```
1. User invokes: agentchrome interact scroll --selector ".stage" --direction down
2. clap parses --selector into ScrollArgs.selector: Option<String>
3. execute_scroll() detects selector is Some
4. CSS selector resolved to backend_node_id via DOM.getDocument + DOM.querySelector + DOM.describeNode
5. check_element_scrollable() runs Runtime.callFunctionOn to check scrollHeight > clientHeight || scrollWidth > clientWidth
6. If not scrollable -> return AppError::element_not_scrollable
7. If scrollable -> reuse existing container scroll path:
   a. get_container_scroll_position(backend_node_id) -> (before_x, before_y)
   b. compute_scroll_delta(direction, amount, vw, vh) -> (dx, dy)
   c. dispatch_container_scroll(backend_node_id, dx, dy, smooth)
   d. If smooth -> wait_for_smooth_container_scroll(backend_node_id)
   e. get_container_scroll_position(backend_node_id) -> (after_x, after_y)
8. compute_delta(before, after) -> ScrollResult JSON on stdout
```

---

## API / Interface Changes

### CLI Flag Changes

| Flag | Type | Conflicts With | Purpose |
|------|------|----------------|---------|
| `--selector <CSS>` | `Option<String>` | `--uid`, `--to-element`, `--to-top`, `--to-bottom` | Target container by CSS selector |
| `--uid <UID>` | `Option<String>` | `--selector`, `--to-element`, `--to-top`, `--to-bottom` | Target container by accessibility UID |

### Resolution Logic

The new flags resolve to a `backend_node_id` using different paths:

- `--selector` -> Prepends `css:` internally and calls existing `resolve_target_to_backend_node_id()` with `format!("css:{}", selector)`
- `--uid` -> Calls existing `resolve_target_to_backend_node_id()` directly with the UID string

This reuses the existing resolution infrastructure without duplicating code.

### New Error Type

| Error | Constructor | Exit Code | Condition |
|-------|-------------|-----------|-----------|
| Element not scrollable | `AppError::element_not_scrollable(descriptor)` | `ExitCode::GeneralError` (1) | Target element has no overflow |

**Output format:**
```json
{"error": "Element is not scrollable: content does not overflow the container", "code": 1}
```

---

## Scrollability Check

A new async helper function `check_element_scrollable` determines whether an element can scroll:

```
async fn check_element_scrollable(session, backend_node_id) -> Result<(), AppError>
```

**Logic**: Uses `Runtime.callFunctionOn` on the resolved element to evaluate:
```javascript
function() {
    return JSON.stringify({
        sh: this.scrollHeight,
        ch: this.clientHeight,
        sw: this.scrollWidth,
        cw: this.clientWidth
    });
}
```

The element is scrollable if `scrollHeight > clientHeight` OR `scrollWidth > clientWidth`. If neither condition holds, the function returns `AppError::element_not_scrollable`.

**Applied to**: Both `--selector` and `--uid` paths. Not applied to `--container` (to avoid breaking backward compat).

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Replace --container** | Remove `--container`, add `--selector` and `--uid` only | Cleaner API, no legacy flag | Breaking change for existing users | Rejected -- backward compat matters |
| **B: Add --selector/--uid alongside --container** | Keep `--container`, add new flags that internally reuse the same code path | No breaking change, ergonomic new flags, consistent with screenshot command | Three flags for same concept | **Selected** |
| **C: Add --target with type prefix** | Single `--target` flag with `css:` or `uid:` prefix | Fewer flags | Still requires prefix knowledge, not more ergonomic than --container | Rejected -- does not solve discoverability |

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI parsing | Unit | `--selector`/`--uid` conflict rules, argument presence |
| Error constructor | Unit | `element_not_scrollable` message and exit code |
| Scrollability check | BDD | Scrollable vs non-scrollable elements |
| Container scroll via selector | BDD | AC1: CSS selector scroll |
| Container scroll via UID | BDD | AC2: UID scroll |
| Smooth scroll | BDD | AC5: Smooth animation with targeted container |
| Direction coverage | BDD | AC6: All four directions |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Scrollability check gives false negatives for CSS `overflow: scroll` elements with no overflowing content | Low | Medium | Check `scrollHeight > clientHeight` which correctly handles `overflow: scroll` with no content |
| `--container` users confused by new flags | Low | Low | `--container` continues to work; document new flags as preferred |
| Breaking clap conflict rules when adding new flags | Low | High | Unit test all conflict combinations |

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #182 | 2026-04-16 | Initial feature spec |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] Database/storage changes planned with migrations (N/A -- no DB)
- [x] State management approach is clear (N/A -- stateless command)
- [x] UI components and hierarchy defined (N/A -- CLI tool)
- [x] Security considerations addressed (input validation via clap)
- [x] Performance impact analyzed (< 10ms overhead for scrollability check)
- [x] Testing strategy defined
- [x] Alternatives were considered and documented
- [x] Risks identified with mitigations
