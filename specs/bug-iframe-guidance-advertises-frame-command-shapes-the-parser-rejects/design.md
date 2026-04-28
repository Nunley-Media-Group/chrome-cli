# Root Cause Analysis: Iframe guidance advertises --frame command shapes the parser rejects

**Issue**: #286
**Date**: 2026-04-28
**Status**: Draft
**Author**: Codex (write-spec)

---

## Root Cause

The public frame-targeting implementation is mostly command-group scoped. `PageArgs`, `JsArgs`, `InteractArgs`, `FormArgs`, `MediaArgs`, and `DomArgs` each define `frame` on the group argument struct before dispatching to their subcommands. That makes commands such as `agentchrome page --frame 1 snapshot` and `agentchrome dom --frame 1 select body` the accepted parser shape. `network list` is the known exception: `NetworkListArgs` owns its own `frame` flag, so `agentchrome network list --frame 1` is the accepted shape for network filtering.

Several static guidance sources drifted away from that parser contract. The iframe strategy data advertises `agentchrome page snapshot --frame N`; the generic examples data advertises page snapshot, hittest, and analyze examples with `--frame` after the page subcommand; clap long-help for page hittest, page coords, and page analyze does the same; and diagnose pattern suggestions include at least one `page snapshot --frame N` hint. Generated man pages inherit those stale strings from clap metadata and `examples_data` enrichment, so the stale guidance is replicated across runtime help, `agentchrome examples`, and committed `man/*.1` files.

The defect is not in the core frame-targeting execution path. The live issue confirms group-scoped page targeting succeeds, and existing feature tests already cover frame routing. The root cause is missing parser-backed validation for advertised command strings: current tests assert that guidance text exists, but they do not prove the text is a command shape clap accepts.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/cli/mod.rs` | 1164-1170, 1620-1626, 1901-1907, 2313-2319, 2824-2830, 3344-3352 | Command-group `--frame` parser definitions for page, js, media, interact, form, and dom. |
| `src/cli/mod.rs` | 1359-1365, 1395-1402 | Page help examples that currently place `--frame` after subcommands. |
| `src/examples/strategies.rs` | 59-95, 153-199 | Strategy guide capabilities, workarounds, and recommended sequences that include stale page frame examples. |
| `src/examples_data.rs` | 169-225, 341-344, 382-385, 491-498, 579-582, 739-742 | Built-in command examples consumed by `agentchrome examples` and man-page enrichment. |
| `src/diagnose/detectors.rs` | 252-258 | Iframe and overlay suggestion constants that mention frame targeting without concrete accepted command shapes. |
| `src/diagnose/patterns.rs` | 107-146 | Storyline and SCORM pattern suggestions, including `page snapshot --frame N`. |
| `src/man_enrichment.rs` | 54-74 | Appends `examples_data` entries into generated man pages, spreading stale examples into `man/*.1`. |
| `tests/features/examples-strategies.feature` | 36-87 | Current strategy-guide coverage checks output shape and fields but not parser acceptance of advertised commands. |
| `tests/features/diagnose.feature` | 29-51 | Current diagnose coverage checks discoverability but not parser acceptance of suggestions. |

### Triggering Conditions

- A user or agent follows iframe guidance from `examples strategies iframes`, `diagnose --current`, help, or man pages.
- The guidance string places `--frame` after a page subcommand whose args do not own that flag.
- The user runs the advertised command against the current parser.
- Clap rejects the flag before AgentChrome can execute the intended frame-targeted operation.

---

## Fix Strategy

### Approach

Use the current parser as the canonical public API and update guidance to match it. The minimal fix is to correct static command strings and help examples so every advertised command is parseable today. For `page`, `dom`, `js`, `interact`, `form`, and `media`, examples should place `--frame` immediately after the command group. For `network list`, examples should keep `--frame` on `network list` because that flag belongs to `NetworkListArgs`.

Add regression coverage that extracts or enumerates advertised frame commands and validates them with the clap command definition. Parser validation should normalize placeholder tokens (`N`, `<uid>`, `<selector>`, `<script>`) into concrete sample values before parsing. This keeps the test deterministic and avoids launching Chrome when only parser acceptance is being tested. Chrome-backed smoke coverage still verifies that the accepted frame commands work against a real iframe page.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/examples/strategies.rs` | Rewrite iframe and SCORM strategy capability, workaround, and recommended sequence strings to accepted command shapes, for example `agentchrome page --frame N snapshot`. | Fixes the failing strategy guide surface from the issue. |
| `src/examples_data.rs` | Rewrite page frame examples to `agentchrome page --frame 1 <subcommand>` while preserving already accepted dom/js/interact/form/media/network examples. | Fixes built-in examples and the man-page examples derived from them. |
| `src/cli/mod.rs` | Update page command `after_long_help` examples for frame targeting to group-scoped placement. | Fixes `--help` and generated clap man content at the source. |
| `src/diagnose/detectors.rs` | Replace abstract iframe suggestions with concrete accepted commands such as `agentchrome page --frame N snapshot` and `agentchrome interact --frame N click-at X Y`. | Prevents diagnose guidance from implying rejected page/interact shapes. |
| `src/diagnose/patterns.rs` | Rewrite Storyline and SCORM pattern suggestions to accepted command shapes. | Fixes pattern-specific diagnose output. |
| `tests/features/286-iframe-guidance-advertises-frame-command-shapes-the-parser-rejects.feature` | Add regression scenarios for strategy guide parser validation, diagnose suggestion parser validation, cross-surface consistency, and preserved accepted frame commands. | Provides issue-scoped BDD coverage. |
| `tests/bdd.rs` | Add parser-validation steps that normalize advertised command placeholders and call `agentchrome::command().try_get_matches_from(...)`. | Validates parser acceptance without requiring a browser for every static guidance string. |
| `man/*.1` | Regenerate via `cargo xtask man` after source guidance changes. | Keeps committed man pages in sync with clap and examples sources. |

### Blast Radius

- **Direct impact**: Static help/example/suggestion text and BDD parser-validation steps.
- **Indirect impact**: Generated man pages, examples JSON/plain output, diagnose challenge and pattern output, any snapshots that assert exact guidance text.
- **Risk level**: Low - the preferred fix does not change frame-targeting internals or parser behavior. The main risk is missing one stale guidance string, mitigated by a path-audit test.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| A stale `page <subcommand> --frame` string remains in help, examples, or man pages | Medium | Add parser-backed BDD coverage over advertised frame commands and search all guidance surfaces for rejected placements during verification. |
| Tests over-normalize placeholders and parse a command the real user-facing string would not support | Low | Keep placeholder normalization narrow and visible in `tests/bdd.rs`; use realistic values such as `1`, `s3`, `css:button`, and `document.title`. |
| Updating examples changes progressive-disclosure output shape | Low | Modify only `cmd` string values; keep existing JSON fields and listing/detail behavior unchanged. |
| Generated man pages drift from source changes | Medium | Regenerate with `cargo xtask man` and include man-page diffs in the delivery commit. |
| A parser broadening attempt accidentally creates duplicate or conflicting `--frame` semantics | Low | Prefer guidance correction over parser expansion; if parser expansion is chosen, require explicit tests for both placements and conflict behavior. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Correct guidance to match the current parser | Keep parser behavior unchanged and update static guidance to accepted command shapes. | Selected - smallest fix, preserves existing public behavior, and directly addresses the reported drift. |
| Broaden the parser to accept both group-scoped and subcommand-scoped `--frame` | Add duplicate frame args to affected subcommands or custom normalization. | Larger public API change with higher conflict risk; unnecessary because the current accepted group-scoped API already works. |
| Move every command to subcommand-scoped `--frame` | Redesign parser ownership so examples such as `page snapshot --frame 1` become canonical. | High blast radius across page/js/dom/interact/form/media command groups and existing scripts; out of scope for a guidance drift bug. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal - focused on guidance/parser contract alignment
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #286 | 2026-04-28 | Initial defect report |
