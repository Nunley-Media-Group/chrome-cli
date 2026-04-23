# Root Cause Analysis: Script bind stores raw command output envelope for `js exec`

**Issue**: #248
**Date**: 2026-04-23
**Status**: Draft
**Author**: Rich Nunley

---

## Root Cause

The script runner binds the **raw JSON value returned by the command dispatcher** into `$vars` without any per-command post-processing. For most scriptable commands this is already the "useful" scalar or structured value — `navigate` returns `{url, title}`, `page find` returns an array of matches, `page text` returns a flattened string result. `js exec` is the outlier: it returns a three-field envelope `{result, type, truncated}` where the caller-visible value is nested one level deep under `.result`.

Because script expressions (the `if` / `while` / interpolation engine) treat `$vars.<name>` as the caller-visible value, users naturally expect `$vars.t` to be the scalar that `document.title` returned. Instead they get the envelope, and member access like `.includes(...)` fails with `TypeError`. The inconsistency is a protocol mismatch between `js exec`'s wire format (which keeps `truncated` / `type` as useful metadata for stdout consumers) and the script runner's in-process expression model (which only cares about the value).

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/script/runner.rs` | 128–134 | After `invoke(...)` returns a `serde_json::Value`, calls `ctx.bind(bind_name, value.clone())` unconditionally — no per-command unwrap. |
| `src/script/context.rs` | 23–24 | `ScriptContext::bind(&mut self, name: &str, value: serde_json::Value)` stores whatever it receives into the `$vars` map. |
| `src/js.rs` | 636–640 | `run_from_session` constructs the envelope `{"result": value, "type": js_type, "truncated": was_truncated}` — this is the envelope that leaks into `$vars`. |

### Triggering Conditions

- The script step’s `cmd` is `["js", "exec", ...]` (scalar, object, or array result — any shape).
- The step has a `bind` field.
- A later step dereferences `$vars.<bind>` as if it were the underlying JS value (e.g. `.includes(...)` on a string, `.length` on an array, `.field` on an object).
- The user is unaware of `js exec`’s internal envelope shape — which is the common case because the envelope is an implementation detail of the wire format, not documented as a bind contract.

---

## Fix Strategy

### Approach

Unwrap the `js exec` envelope **at the bind site** in `src/script/runner.rs`, so `$vars` receives the `result` field instead of the full envelope. The unwrap is gated on the command being `js exec` — other commands are untouched, preserving their current bind shapes (`page find` → array, `navigate` → `{url,title}`, etc.).

This is the minimal correct fix because (a) it changes only the script-runner-internal binding contract, (b) it leaves `agentchrome js exec`'s stdout envelope intact for standalone callers, and (c) it localizes the special case to the one command whose envelope shape diverges from user expectation. We explicitly do **not** introduce a general "auto-unwrap any envelope with a scalar `result`" rule in this fix — issue FR3 flagged that as `Could`, and extending it to other commands is out of scope per Out of Scope in `requirements.md`.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/script/runner.rs` (around the `bind` call at lines 128–134) | Before calling `ctx.bind`, if the step's `cmd[0..2] == ["js", "exec"]` **and** the returned `value` is a JSON object containing a `"result"` key, replace the bound value with `value["result"]`. Otherwise keep current behaviour. | Localizes the unwrap to the one command whose envelope diverges from caller expectations; zero change for all other commands. |
| `tests/features/batch-script-execution.feature` | Add `@regression` scenarios proving: (a) `$vars.t.includes('…')` works after a `js exec` scalar bind, and (b) a `js exec` returning an object binds to the object (not the envelope). | Locks in the new contract and guards against future re-wrapping. |
| `tests/bdd.rs` | Embed a new script fixture for the `js exec` bind regression (matching the pattern used by existing `simple.json` / `page-find.json` fixtures at lines 490–566). | Keeps BDD fixtures colocated with tests, per existing convention. |

### Blast Radius

- **Direct impact**: `src/script/runner.rs` (one added conditional), new regression fixture, new Gherkin scenarios.
- **Indirect impact**: Any pre-existing user script that accessed `$vars.<jsExecBind>.result` explicitly. In JavaScript-style expression evaluation, `"some-string".result` evaluates to `undefined` rather than throwing, so most such expressions degrade gracefully rather than fail hard. Scripts that depended on truthiness of `.result` (e.g. `if ($vars.t.result)`) will flip to falsy — acceptable per issue AC4 (“or the script is not broken”), since the correct replacement is `$vars.t` itself.
- **Risk level**: Low.

The change does **not** touch:
- `src/js.rs` (the envelope is still produced — it's just unwrapped by the script runner).
- `agentchrome js exec` stdout format.
- Bind behaviour for `navigate`, `page find`, `page text`, `page screenshot`, `perf record`, etc.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| An existing script relies on `$vars.<jsExecBind>.truncated` or `.type` metadata. | Low | No such script is present in the repo (grep of `tests/` and `docs/`). If one appears later, the envelope is still obtainable by binding from a step that writes its own wrapper; this is an acceptable behavioural change for a bug fix. |
| An existing script reads `$vars.<jsExecBind>.result` explicitly. | Low | `"string".result` evaluates to `undefined` in the expression engine (no throw); regression test AC3 + a manual fixture will confirm the runner does not regress on adjacent bind shapes. |
| Other commands are accidentally unwrapped by an overly broad heuristic. | Low | Fix is gated on `cmd[0..2] == ["js","exec"]` — no heuristic, explicit command match. AC3 regression scenario locks in `page find` array shape. |
| Future additions to `js exec`'s envelope (new metadata fields) leak into `$vars`. | Low | The unwrap discards all fields other than `result` by design; future metadata stays out of `$vars`, matching the scalar-caller-expectation contract. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| **A: Change `js exec`'s wire format to emit the scalar directly** | Drop the envelope and return `result` as the top-level JSON on stdout. | Rejected — breaks every non-script caller of `agentchrome js exec` that currently parses `{result,truncated,type}`. Issue explicitly puts this out of scope. |
| **B: General "single-scalar-result" auto-unwrap rule** | At bind time, if the returned value is an object whose only non-metadata field is `result`, unwrap. | Rejected for this fix — broader than the defect requires, drags in heuristic decisions about which fields count as "metadata", and risks silent behaviour changes for commands like `page screenshot` that also have a `result`-ish structure. Flagged for a future feature issue (issue FR3). |
| **C: Unwrap at bind site, gated on `cmd == ["js","exec"]`** | Minimal targeted change in the script runner. | **Selected** — smallest diff, zero impact on non-script callers, no heuristic. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references (`src/script/runner.rs:128–134`, `src/js.rs:636–640`)
- [x] Fix is minimal — no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `steering/structure.md` — change lives inside `src/script/`)
