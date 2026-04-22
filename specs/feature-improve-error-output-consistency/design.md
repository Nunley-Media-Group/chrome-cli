# Design: Improve Error Output Consistency on All Failure Paths

**Issues**: #197
**Date**: 2026-04-21
**Status**: Draft
**Author**: Claude (spec-driven)

---

## Overview

This is a cross-cutting quality feature that tightens the existing structured-error contract rather than building new functionality. The error infrastructure already in `src/error.rs` (the `AppError` / `ExitCode` / `custom_json` triad) and the top-level dispatch in `src/main.rs` are sound — the defect is that individual command modules have a handful of paths that return errors without routing them through `AppError::print_json_stderr`, or that raise no error at all and merely exit via an unwrapped `Result`.

The work is therefore: (1) audit the three modules called out in the issue (`form.rs`, `interact.rs`, `page.rs` including the page-wait/snapshot/screenshot paths), (2) convert any non-`AppError` leaf into a typed `AppError` constructor, (3) add a first-class `form_fill_not_fillable` constructor that carries element-type context in `custom_json`, (4) add a `--uid` misuse detector to the clap-error branch in `main.rs`, and (5) extend the top-level clap `after_long_help` to document the error contract.

Because the existing `AppError` path guarantees exactly-once emission (`main.rs` line 82–86 invokes `print_json_stderr` once and exits), meeting AC6 requires only removing any leftover `eprintln!`, `anyhow::anyhow!`, or direct-stderr writes in the audited modules.

---

## Architecture

### Component Diagram

Reference `steering/structure.md` for the full layer diagram. Only two layers are affected:

```
┌──────────────────────────────────────────────────────────────┐
│  CLI dispatch (src/main.rs)                                   │
│  ─ clap error branch: reformats parse errors → AppError       │
│  ─ runtime error branch: calls AppError::print_json_stderr    │
│    (+ NEW: --uid misuse detector invoked from clap branch)    │
└───────────────────────────────┬──────────────────────────────┘
                                │
┌───────────────────────────────▼──────────────────────────────┐
│  Command modules (src/form.rs, interact.rs, page.rs, …)       │
│  ─ All leaf failures construct typed AppError                 │
│  ─ NEW: AppError::form_fill_not_fillable(target, tag, role)   │
└───────────────────────────────┬──────────────────────────────┘
                                │
┌───────────────────────────────▼──────────────────────────────┐
│  Error core (src/error.rs)                                    │
│  ─ AppError { message, code, custom_json }                    │
│  ─ print_json_stderr() — already exactly-once                 │
└──────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. User invokes `agentchrome form fill sN "v"`
2. clap parses successfully → run() dispatches to form::execute_fill
3. form.rs resolves sN via snapshot → inspects tag / ARIA role
4. If role ∉ {textbox, searchbox, editable combobox, textarea, input[type=text|email|...]}:
   → construct AppError::form_fill_not_fillable(sN, tag, role)
     which calls a private structured-loss-style helper that builds
     custom_json = {error, code:1, kind:"not_fillable", element_type:{tag,role}, suggested_alternatives:[…]}
5. run() returns Err(app_err); main() calls print_json_stderr (once) and exits with app_err.code
```

For the `--uid s6` case:

```
1. clap parsing fails because `--uid` is not defined on `interact click`
2. clap ErrorKind::UnknownArgument triggers the existing main.rs error branch
3. NEW: inspect the original argv; if it contains `--uid` or `--selector` as a flag where
   the target subcommand takes a positional, append a "Did you mean: agentchrome … s6" hint
4. Emit the AppError with the augmented message exactly once
```

---

## API / Interface Changes

### New `AppError` constructor

```rust
impl AppError {
    /// Emitted when `form fill` targets an element whose tag/role is not fillable.
    /// Carries element-type context in `custom_json` for AI-agent consumption.
    #[must_use]
    pub fn form_fill_not_fillable(
        target: &str,
        tag: &str,
        role: Option<&str>,
    ) -> Self {
        let role_str = role.unwrap_or("");
        let alternatives = suggest_alternatives(tag, role);
        let message = format!(
            "Element '{target}' (tag={tag}{role_sep}{role_str}) is not fillable. \
             Use {} instead.",
            alternatives.join(" or "),
            role_sep = if role_str.is_empty() { "" } else { ", role=" },
        );
        let custom = serde_json::json!({
            "error": message,
            "code": ExitCode::GeneralError as u8,
            "kind": "not_fillable",
            "element_type": { "tag": tag, "role": role },
            "suggested_alternatives": alternatives,
        });
        Self { message, code: ExitCode::GeneralError, custom_json: Some(custom.to_string()) }
    }
}

fn suggest_alternatives(tag: &str, role: Option<&str>) -> Vec<&'static str> {
    match (tag, role) {
        ("button", _) | (_, Some("button")) => vec!["'agentchrome interact click'"],
        ("a", _) | (_, Some("link")) => vec!["'agentchrome interact click'"],
        ("canvas", _) => vec!["'agentchrome js exec'"],
        _ => vec!["'agentchrome interact click'", "'agentchrome js exec'"],
    }
}
```

### New clap-error hint helper (in `main.rs`)

```rust
/// If argv contains `--uid <val>` or `--selector <val>` targeting a subcommand
/// that takes those values positionally, return a "Did you mean: …" suffix.
fn syntax_hint(argv: &[String]) -> Option<String> { … }
```

Invoked from the existing `Err(e) = Cli::try_parse()` branch, appended to `clean` when non-empty.

### Existing error JSON schema (unchanged)

```json
{ "error": "string", "code": 1 }
```

Extended shape (only when `custom_json` is populated):

```json
{
  "error": "Element 'sN' (tag=div) is not fillable. Use 'agentchrome interact click' instead.",
  "code": 1,
  "kind": "not_fillable",
  "element_type": { "tag": "div", "role": null },
  "suggested_alternatives": ["'agentchrome interact click'"]
}
```

---

## Audit Findings

The audit enumerated every `Result`-returning entry point and leaf `Err(...)` construction in the three flagged modules (`src/form.rs`, `src/interact.rs`, and the `src/page/` submodule tree — `page/wait.rs`, `page/screenshot.rs`, `page/snapshot.rs`, `page/find.rs`, `page/analyze.rs`, `page/element.rs`, `page/text.rs`, `page/hittest.rs`, `page/coords.rs`, `page/mod.rs`). Every leaf already constructs an `AppError` directly (via a named constructor or a struct literal) or propagates one via `?` from an upstream typed error. No path uses `anyhow::anyhow!`, `anyhow::bail!`, or bare-string `Err("...".into())`. `eprintln!` calls in these modules appear only on warning paths (persistence-write failures, non-fatal hit-test diagnostics) and never replace an error return.

The only silent-failure path is the one called out in the issue: `fill_element` and `clear_element` in `src/form.rs` fall through to a JavaScript setter branch for any element that isn't a text input and isn't a combobox, which lets `el.value = value` succeed silently on `<div>`, `<canvas>`, `<button>`, etc.

| File | Path (function / line) | Current behaviour | Fix |
|------|------------------------|-------------------|-----|
| `src/form.rs` | `fill_element` — else branch after `is_text_input` / combobox checks | Runs `FILL_JS` on any element; silently succeeds on non-fillable tags (div, canvas, button, contenteditable, etc.) | Classify: `<select>` / `<input type=checkbox\|radio>` remain routed to `FILL_JS`; all other tags/roles return `AppError::form_fill_not_fillable(target, tag, role)` |
| `src/form.rs` | `clear_element` — else branch after `is_text_input` check | Same silent `CLEAR_JS` fallback as fill | Same classification; non-`<select>` / non-checkbox/radio elements return `form_fill_not_fillable` with a "clear" alternative suggestion |
| `src/interact.rs` | All paths | Already route through `AppError` (see `element_not_found`, `element_zero_size`, `uid_not_found`, `stale_uid`, `invalid_key`, `duplicate_modifier`, `interaction_failed`, `snapshot_failed`, `no_snapshot_state`) | No fix required |
| `src/page/wait.rs` | Timeout / JS-eval branches | Already route through `AppError::wait_timeout` and `AppError::js_eval_error` | No fix required |
| `src/page/screenshot.rs` | Failure branches | Already route through `AppError::invalid_clip`, `AppError::element_not_found`, `AppError::screenshot_failed` | No fix required |
| `src/page/snapshot.rs`, `src/page/find.rs`, `src/page/analyze.rs`, `src/page/element.rs`, `src/page/text.rs`, `src/page/hittest.rs` | All paths | Propagate typed errors via `?`; leaves construct `AppError` directly | No fix required |

---

## Database / Storage Changes

None — no persistence.

---

## State Management

None — errors are stateless.

---

## UI Components

None — CLI only.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Add `ExitCode::UnsupportedElement` (code 6)** | New exit code for AC2 | Fine-grained sigint for agents | Out-of-scope (issue #197 explicitly forbids new exit codes); breaks existing consumers that treat 1–5 as an exhaustive set | Rejected |
| **B: Per-module error types (`FormError`, `InteractError`) that `impl Into<AppError>`** | Stronger typing inside modules | Idiomatic Rust, easier to exhaustively match | Large refactor; out-of-scope; risk of regression on well-tested paths | Rejected |
| **C: Keep the `{error, code}` shape and widen it only via `custom_json`** | **Selected** | Zero-breakage for existing consumers; structured context available for agents that opt in; minimises diff | `custom_json` is opt-in so callers must remember to set it | **Selected** |
| **D: Clap-global `after_parse` hook for --uid suggestion** | Use a clap-derive callback | Centralised in clap | clap 4 doesn't expose a post-parse global hook; would require custom argv pre-scan anyway | Rejected in favour of the pre-scan we already need |

---

## Security Considerations

- [x] **Authentication**: N/A — CLI tool, no auth
- [x] **Input Validation**: Element-type context (tag, role) is derived from CDP — already sanitised by `serde_json::json!`
- [x] **Data Sanitization**: `serde_json::to_string` handles escaping; no manual string interpolation into JSON
- [x] **Sensitive Data**: Existing `AppError` messages occasionally include file paths (e.g., `file_write_failed`); no new leakage introduced — scope of this feature does not change existing message content beyond the three audited modules

---

## Performance Considerations

- [x] **Error-path overhead**: Formatting overhead < 1ms; happens once per failed invocation
- [x] **Success-path impact**: Zero — no changes to happy paths
- [x] **Audit cost**: One-time implementation cost, not a runtime cost

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| `AppError::form_fill_not_fillable` | Unit (`src/error.rs` tests module) | Message shape, `custom_json` schema, suggested-alternatives logic |
| `syntax_hint(argv)` | Unit (inline in `main.rs` tests or new `src/cli/hints.rs`) | `--uid` detection, false-positive guard on commands that legitimately take `--uid` |
| `form fill` non-fillable paths | BDD (`tests/features/improve-error-output-consistency.feature`) | AC1, AC2, AC7 |
| `interact click --uid` syntax hint | BDD | AC3 |
| Audit coverage | Doc-style test: a unit test iterates known error-emitting call sites in `form.rs`/`interact.rs`/`page.rs` and asserts each reaches `AppError` | AC4 |
| Help-text error-contract description | BDD (assert `agentchrome --help` output contains exit-code meanings) | AC5 |
| Exactly-once emission | BDD: invoke a failing command, assert stderr has exactly one line that parses as JSON | AC6 |

### Verification Gates (per `steering/tech.md`)

- Debug Build / Unit Tests / Clippy / Format Check: all pass
- Feature Exercise Gate: `tests/fixtures/improve-error-output-consistency.html` containing non-fillable elements (`<div>`, `<canvas>`, `<button>`, `role="combobox"` without editable input) plus a fillable `<input>` control for a positive-control AC

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Audit misses a silent path in a module outside the three flagged files | Medium | Low | Add a generic BDD sweep: invoke a sample of commands with bad args; assert every non-zero exit produces JSON on stderr |
| `custom_json` introduces shape drift for consumers that parse the stable `{error, code}` form | Low | Medium | AC7 enforces that `custom_json` always includes the two stable fields |
| `--uid` detection produces false positives on commands that genuinely accept `--uid` | Low | Low | Gate the hint on clap's `ErrorKind::UnknownArgument` (i.e., only suggest when the flag was actually unrecognised) |
| Form-fill element-type detection uses an unreliable signal (tag vs role) | Medium | Medium | Check both HTML tag (authoritative for native inputs) and ARIA role (authoritative for custom widgets); surface whichever is non-empty |

---

## Open Questions

- [ ] Should the audit sweep extend to `dialog.rs`, `network.rs`, `perf.rs` since they have historically had silent-path bugs (#96, #99, #134)? **Proposed answer**: no — scope of #197 is the three flagged modules; a follow-up issue can cover the rest after this lands.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #197 | 2026-04-21 | Initial feature spec |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (per `structure.md`)
- [x] All API/interface changes documented with schemas
- [x] Database/storage changes planned with migrations — N/A
- [x] State management approach — N/A
- [x] UI components and hierarchy — N/A
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives considered and documented
- [x] Risks identified with mitigations
