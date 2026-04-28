# Root Cause Analysis: Support top-level await in js exec expressions

**Issue**: #279
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (write-spec)

---

## Root Cause

`src/js.rs::execute_expression_with_context` wraps every expression in a plain JavaScript block before sending it to Chrome with `Runtime.evaluate`. The current parameter shape is:

```rust
let wrapped = format!("{{ {code} }}");
let mut params = serde_json::json!({
    "expression": wrapped,
    "returnByValue": true,
    "awaitPromise": await_promise,
    "generatePreview": true,
});
```

The block wrapper solves the issue #183 scope-isolation bug for `let` and `const`, but the evaluated source is still treated as a classic script evaluation. `awaitPromise: true` only awaits a Promise value after evaluation succeeds; it does not make the `await` keyword legal during parsing. Therefore `await Promise.resolve("done")` fails with a syntax error before `awaitPromise` can do anything.

The related feature spec selected block-scope wrapping partly because it claimed top-level await would still work (`specs/feature-javascript-execution/design.md:198-208`). The implementation follows the block wrapper design, but the CDP evaluation parameters are missing the mode needed to permit REPL-style direct top-level await syntax.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/js.rs` | 300-322 | Primary `js exec` dispatch path selects `execute_expression_with_context` when `--uid` is absent. |
| `src/js.rs` | 434-486 | `execute_expression_with_context` constructs the wrapped expression and sends `Runtime.evaluate`. This is the root cause location. |
| `src/js.rs` | 488-541 | Worker execution calls `execute_expression`, which delegates to the same helper. |
| `src/js.rs` | 603-615 | Script-runner adapter calls the same helper for batch/session execution. |
| `tests/features/js-execution.feature` | 52-68 | Existing BDD coverage verifies returned Promise awaiting and `--no-await`, but not direct top-level `await` syntax. |

### Triggering Conditions

- The user invokes `agentchrome js exec` without `--uid`, so the command uses `Runtime.evaluate`.
- The supplied code contains direct `await` syntax outside any async function, for example `await Promise.resolve("done")`.
- `execute_expression_with_context` wraps the code in a block statement but does not enable an evaluation mode that permits direct top-level await syntax.
- Existing tests cover promise-returning expressions and declaration isolation, but do not exercise direct top-level `await`, so the design/implementation mismatch survived.

---

## Fix Strategy

### Approach

Keep the existing block-scope wrapper and add the minimal `Runtime.evaluate` parameter needed for REPL-style expression evaluation. Chrome DevTools Protocol exposes `replMode` on `Runtime.evaluate` for console-like evaluation semantics, including top-level await and lenient `let` redeclaration behavior for REPL-originated bindings. Setting this flag on the shared expression helper preserves the existing single-CDP-call design, retains block-scope isolation, and avoids introducing an async IIFE or module execution path.

The change belongs in `execute_expression_with_context`, not at each call site. That helper is already the shared boundary for primary page expression execution, same-origin frame expression execution, worker execution, and script-runner execution. Updating it once satisfies the path-audit requirement and keeps behavior consistent across all expression-evaluation entry points.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/js.rs` | Add `replMode: true` to the `Runtime.evaluate` params built by `execute_expression_with_context`. | Makes direct top-level `await` syntax legal while preserving existing `awaitPromise`, `returnByValue`, context ID, and block wrapping behavior. |
| `src/js.rs` | Add focused unit coverage around the evaluate-parameter builder if the params construction is extracted into a small helper. | Gives a fast regression check that `replMode` stays present without requiring a live Chrome for every test run. |
| `tests/features/279-support-top-level-await-in-js-exec-expressions.feature` | Add regression BDD scenarios for AC1-AC3. | Proves the observed bug is fixed and adjacent behavior is preserved through the CLI. |
| `tests/bdd.rs` | Add or reuse step bindings needed for the new regression feature. | Integrates the feature file with the repository's existing cucumber test runner. |

### Blast Radius

- **Direct impact**: `Runtime.evaluate` calls made through `execute_expression_with_context` in `src/js.rs`.
- **Indirect impact**: primary `js exec`, `js exec --frame` when it uses a same-origin frame context, `js exec --worker`, and the script-runner adapter all inherit the same expression-evaluation params.
- **Not impacted**: `--uid` function execution uses `Runtime.callFunctionOn` and does not pass through `Runtime.evaluate`; code input resolution, console event capture, result extraction, truncation, JSON/plain output formatting, and dialog interceptor setup remain unchanged.
- **Risk level**: Low to Medium. The code change is a single CDP parameter on an existing call, but it affects every expression-evaluation path. Regression coverage must prove promise awaiting and block-scope isolation still behave as specified.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Returned Promise awaiting regresses because `replMode` changes evaluation completion semantics. | Low | AC2 re-runs the existing `new Promise(...)` case and asserts the resolved result still appears when `awaitPromise` is enabled. |
| The `let`/`const` isolation fix from issue #183 regresses. | Low | AC3 repeats same-name declarations across consecutive invocations and fails if block wrapping stops isolating those declarations. |
| `--no-await` behavior changes for promise-returning expressions. | Low | Out of scope for semantic changes; verification should run the existing `--no-await` BDD scenario in `tests/features/js-execution.feature`. |
| Worker or script-runner expression execution diverges from the primary path. | Low | Implement the change in `execute_expression_with_context` only, so all expression paths that already delegate there share one parameter shape. |
| Older Chrome versions reject `replMode`. | Low | `Runtime.evaluate` accepts unknown/experimental params only when supported by the browser target; if compatibility evidence appears during implementation, fall back to a guarded param strategy that preserves AC1 on supported Chrome while maintaining structured errors elsewhere. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Add `replMode: true` to `Runtime.evaluate` | Keep block wrapping and enable console-like evaluation semantics for direct top-level await. | Selected - smallest change, preserves expression result behavior and shared helper architecture. |
| Wrap user code in an async IIFE | Transform expressions into `(async () => { ... })()` and rely on `awaitPromise`. | Would require return-value rewriting for expression snippets and could change `this`, `var`, and global binding semantics. Too disruptive for a bug fix. |
| Execute expressions as JavaScript modules | Add a module execution path for top-level await. | Explicitly out of scope in the issue and larger than needed for CLI expression evaluation. |
| Remove block wrapping | Return to raw `Runtime.evaluate` source. | Would likely reintroduce the issue #183 `let`/`const` redeclaration bug. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal - no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #279 | 2026-04-27 | Initial defect design |
