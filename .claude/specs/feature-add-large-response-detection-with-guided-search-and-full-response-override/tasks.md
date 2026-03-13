# Tasks: Large Response Detection with Guided Search and Full-Response Override

**Issues**: #168
**Date**: 2026-03-12
**Status**: Planning
**Author**: AI (nmg-sdlc)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 5 | [ ] |
| Integration | 2 | [ ] |
| Testing | 5 | [ ] |
| **Total** | **15** | |

---

## Phase 1: Setup

### T001: Create output.rs Module with Large-Response Gate

**File(s)**: `src/output.rs`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `DEFAULT_THRESHOLD` constant is `16_384` (16 KB)
- [ ] `LargeResponseGuidance` struct serializes to `{"large_response": true, "size_bytes": N, "command": "...", "summary": {...}, "guidance": "..."}`
- [ ] `emit()` function accepts `&impl Serialize`, `&OutputFormat`, command name `&str`, and summary closure `FnOnce(&T) -> serde_json::Value`
- [ ] `emit()` serializes value once (respecting `--pretty`), checks byte length against threshold, and prints JSON or guidance
- [ ] `emit()` bypasses threshold check when `full_response` is `true`
- [ ] `format_human_size()` returns "X bytes", "X KB", or "X.Y MB" as appropriate
- [ ] `build_guidance_text()` generates guidance string with human-readable size, summary, `--search` example, `--full-response` example, and when-to-use-full-response reasons
- [ ] `command_specific_guidance()` returns summary sentence, search example, and full-response reasons per command name

**Notes**: The `emit()` function replaces per-module `print_output()` calls. It must handle both compact and pretty-printed JSON. The summary closure is only called when threshold is exceeded (lazy evaluation).

### T002: Extend OutputFormat with Global Flags

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `OutputFormat` struct has `full_response: bool` field with `#[arg(long, global = true)]`
- [ ] `OutputFormat` struct has `large_response_threshold: Option<usize>` field with `#[arg(long, global = true)]`
- [ ] `--json`, `--pretty`, `--plain` remain mutually exclusive (named argument group)
- [ ] `--full-response` and `--large-response-threshold` are independent of the format group
- [ ] `agentchrome --help` shows all new flags with clear descriptions
- [ ] `--large-response-threshold 0` is rejected (validation: value must be > 0)

**Notes**: The `#[group(multiple = false)]` annotation currently covers all `OutputFormat` fields. Restructure so format flags use a named conflict group while the new flags are ungrouped. Use `#[arg(conflicts_with_all = ["json", "pretty"])]` on `plain` etc., or use `#[group(id = "format", multiple = false)]` on the three format fields only.

### T003: Extend Config with large_response_threshold

**File(s)**: `src/config.rs`, `src/main.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `OutputConfig` has `large_response_threshold: Option<usize>` field
- [ ] `ResolvedOutput` has `large_response_threshold: usize` field (resolved with `DEFAULT_THRESHOLD`)
- [ ] `resolve_config()` populates `large_response_threshold` from config or default
- [ ] `apply_config_defaults()` in `main.rs` merges: CLI flag > config file > default
- [ ] Config file key `[output] large_response_threshold = 8192` is parsed correctly
- [ ] TOML deserialization handles missing key gracefully (uses default)

**Notes**: Follow the existing pattern for config merging (see `apply_config_defaults()` in `main.rs`). The CLI flag (`Option<usize>`) takes precedence when `Some`, otherwise falls back to config file value.

---

## Phase 2: Backend Implementation

### T004: Add --search to page snapshot and Implement Tree Filtering

**File(s)**: `src/cli/mod.rs`, `src/snapshot.rs`, `src/page/snapshot.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `PageSnapshotArgs` has `search: Option<String>` field with `#[arg(long)]`
- [ ] `filter_tree()` in `snapshot.rs` accepts `&SnapshotNode` and `&str` query, returns `Option<SnapshotNode>`
- [ ] Filter matches case-insensitively on node `name` and `role`
- [ ] Matching nodes retain ancestor chain (non-matching branches pruned)
- [ ] `execute_snapshot()` applies search filter before output
- [ ] `execute_snapshot()` calls `output::emit()` with snapshot summary closure
- [ ] Snapshot summary closure returns `{"total_nodes": N, "top_roles": ["role1", ...]}`
- [ ] `--search` result is printed directly via `output::emit()` (bypasses guidance gate per AC13 — search results always pass through regardless of size)
- [ ] `--search` works with `--plain` mode (filters tree text, then prints)

**Notes**: The snapshot has custom serialization (adding `truncated`/`total_nodes` to JSON value). Build the `serde_json::Value` first, then pass to `emit()`. For search, filter the `SnapshotNode` tree before serialization. When `--search` is used, call `output::emit()` but set `full_response = true` conceptually (or call a variant that skips the gate). Simplest approach: when search is present, serialize and print directly without going through the gate.

### T005: Add --search to page text and Integrate with output::emit()

**File(s)**: `src/cli/mod.rs`, `src/page/text.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `PageTextArgs` has `search: Option<String>` field with `#[arg(long)]`
- [ ] Text filtering splits text into paragraphs (double-newline separated), retains paragraphs containing query (case-insensitive), rejoins with double newlines
- [ ] Filtered result uses normal `PageTextResult` schema (`text`, `url`, `title`)
- [ ] `execute_text()` calls `output::emit()` with text summary closure
- [ ] Text summary closure returns `{"character_count": N, "line_count": N}`
- [ ] `--search` bypasses the large-response gate
- [ ] `--search` works with `--plain` mode

### T006: Add --search to js exec and Integrate with output::emit()

**File(s)**: `src/cli/mod.rs`, `src/js.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `JsExecArgs` has `search: Option<String>` field with `#[arg(long)]`
- [ ] JSON filtering: objects retain only key-value pairs where key or serialized value contains query; arrays retain only elements where serialized element contains query; strings returned only if containing query
- [ ] Filtered result uses normal `JsExecResult` schema
- [ ] `execute_exec()` calls `output::emit()` with js summary closure
- [ ] JS summary closure returns `{"result_type": "object|array|string|...", "size_bytes": N}`
- [ ] `--search` bypasses the large-response gate
- [ ] `--search` works with `--plain` mode
- [ ] Existing `--max-size` truncation still applies before search filtering

### T007: Add --search to network list and Integrate with output::emit()

**File(s)**: `src/cli/mod.rs`, `src/network.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `NetworkListArgs` has `search: Option<String>` field with `#[arg(long)]`
- [ ] Search filters requests where URL or method contains query (case-insensitive)
- [ ] Search is applied after existing `--url`, `--method`, `--type`, `--status` filters
- [ ] Filtered result uses normal network list schema (array of `NetworkRequestSummary`)
- [ ] `execute_list()` calls `output::emit()` with network list summary closure
- [ ] Network list summary closure returns `{"request_count": N, "methods": [...], "domains": [...]}`
- [ ] `--search` bypasses the large-response gate
- [ ] `--search` works with `--plain` mode

### T008: Add --search to network get and Integrate with output::emit()

**File(s)**: `src/cli/mod.rs`, `src/network.rs`
**Type**: Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `NetworkGetArgs` has `search: Option<String>` field with `#[arg(long)]`
- [ ] Search filters: response body retained only if it contains query; headers filtered to those with name or value containing query
- [ ] Filtered result uses normal `NetworkRequestDetail` schema
- [ ] `execute_get()` calls `output::emit()` with network get summary closure
- [ ] Network get summary closure returns `{"url": "...", "status": N, "content_type": "...", "body_size_bytes": N}`
- [ ] `--search` bypasses the large-response gate
- [ ] `--search` works with `--plain` mode

---

## Phase 3: Integration

### T009: Register output Module in lib.rs

**File(s)**: `src/lib.rs`
**Type**: Modify
**Depends**: T001
**Acceptance**:
- [ ] `pub mod output;` added to `src/lib.rs`
- [ ] Module is accessible as `agentchrome::output` from tests and other modules
- [ ] `cargo build` compiles without errors

### T010: Wire Config Merging for large_response_threshold

**File(s)**: `src/main.rs`
**Type**: Modify
**Depends**: T002, T003
**Acceptance**:
- [ ] `apply_config_defaults()` merges `large_response_threshold`: CLI `Option<usize>` > config file `Option<usize>` > (default handled by `output::emit()`)
- [ ] `full_response` bool is passed through unchanged (no config file equivalent)
- [ ] Full merge chain verified: `agentchrome page snapshot` with config `large_response_threshold = 8192` uses 8192; with CLI `--large-response-threshold 32768` uses 32768

---

## Phase 4: Testing

### T011: Create BDD Feature File

**File(s)**: `tests/features/large-response-detection.feature`
**Type**: Create
**Depends**: T004, T005, T006, T007, T008
**Acceptance**:
- [ ] All 17 acceptance criteria from requirements.md have corresponding scenarios
- [ ] Valid Gherkin syntax
- [ ] Scenarios are independent (no shared mutable state)
- [ ] Uses concrete examples with realistic data
- [ ] Includes error/edge case scenarios

### T012: Implement BDD Step Definitions

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T011
**Acceptance**:
- [ ] All scenarios have step definitions
- [ ] Steps follow existing cucumber-rs patterns in `tests/bdd.rs`
- [ ] `cargo test --test bdd` passes for all new scenarios
- [ ] Steps use CLI invocation pattern (build binary, run command, check stdout/stderr/exit code)

### T013: Unit Tests for output.rs

**File(s)**: `src/output.rs` (inline `#[cfg(test)]` module)
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Test `emit()` with value below threshold → prints JSON unchanged
- [ ] Test `emit()` with value above threshold → prints guidance object
- [ ] Test `emit()` with `full_response = true` → prints full JSON regardless of size
- [ ] Test `emit()` with custom threshold → guidance triggers at custom threshold
- [ ] Test `format_human_size()` for bytes, KB, MB ranges
- [ ] Test `LargeResponseGuidance` serialization schema (field names, types, order)
- [ ] Test `build_guidance_text()` includes search example, full-response example, and when-to-use reasons
- [ ] All tests pass with `cargo test --lib`

### T014: Manual Smoke Test Against Real Chrome

**File(s)**: (no file changes — execution only)
**Type**: Verify
**Depends**: T004, T005, T006, T007, T008, T009, T010
**Acceptance**:
- [ ] Build debug binary: `cargo build`
- [ ] Connect to headless Chrome: `./target/debug/agentchrome connect --launch --headless`
- [ ] Navigate to https://www.saucedemo.com/
- [ ] `page snapshot` on SauceDemo returns guidance object (if tree > 16 KB) or full output (if under)
- [ ] `page snapshot --search "login"` returns only login-related nodes
- [ ] `page snapshot --full-response` returns full tree
- [ ] `page text` returns expected output (guidance or full, depending on size)
- [ ] `page text --search "Username"` returns matching text
- [ ] `js exec "JSON.stringify(performance.getEntries())"` exercises JS path
- [ ] `network list` after page load shows captured requests
- [ ] `--large-response-threshold 100` triggers guidance on small responses
- [ ] `--plain` mode returns full text without guidance object
- [ ] Disconnect: `./target/debug/agentchrome connect disconnect`
- [ ] Kill orphaned Chrome: `pkill -f 'chrome.*--remote-debugging' || true`

### T015: Verify No Regressions

**File(s)**: (no file changes — execution only)
**Type**: Verify
**Depends**: T011, T012, T013, T014
**Acceptance**:
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo test --test bdd` passes (all BDD tests)
- [ ] `cargo clippy` passes with no new warnings
- [ ] `cargo fmt --check` passes
- [ ] Existing BDD feature files still pass
- [ ] Below-threshold responses produce identical JSON output to pre-feature behavior

---

## Dependency Graph

```
T001 (output.rs) ──┬──▶ T004 (snapshot search + emit) ──┐
                   │                                      │
T002 (CLI flags)  ─┼──▶ T005 (text search + emit) ──────┤
                   │                                      │
T003 (config)     ─┼──▶ T006 (js search + emit) ────────┤
                   │                                      │
                   ├──▶ T007 (network list search) ──────┤
                   │                                      │
                   └──▶ T008 (network get search) ───────┤
                                                          │
T009 (lib.rs) ◀── T001                                   │
                                                          │
T010 (config wire) ◀── T002, T003                        │
                                                          │
T011 (BDD feature) ◀── T004-T008 ──▶ T012 (step defs)   │
                                                          │
T013 (unit tests) ◀── T001                               │
                                                          │
T014 (smoke test) ◀── T004-T010 ────────────────────────┘
                                                          │
T015 (regressions) ◀── T011, T012, T013, T014           │
```

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #168 | 2026-03-12 | Initial feature spec |

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] Test tasks are included for each layer
- [x] No circular dependencies
- [x] Tasks are in logical execution order
