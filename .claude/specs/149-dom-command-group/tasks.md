# Tasks: DOM Command Group

**Issue**: #149
**Date**: 2026-02-19
**Status**: Planning
**Author**: Claude (writing-specs)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 3 | [ ] |
| Backend | 6 | [ ] |
| Integration | 3 | [ ] |
| Testing | 3 | [ ] |
| **Total** | **15** | |

---

## Phase 1: Setup

### T001: Define DomArgs, DomCommand enum, and all subcommand arg structs in CLI layer

**File(s)**: `src/cli/mod.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `DomArgs` struct with `#[command(subcommand)] pub command: DomCommand`
- [ ] `DomCommand` enum with 13 variants: Select, GetAttribute, GetText, GetHtml, SetAttribute, SetText, Remove, GetStyle, SetStyle, Parent, Children, Siblings, Tree
- [ ] `DomSelectArgs` with positional `selector: String` and `--xpath` flag
- [ ] `DomGetAttributeArgs` with positional `node_id: String` and `attribute: String`
- [ ] `DomSetAttributeArgs` with positional `node_id: String`, `attribute: String`, `value: String`
- [ ] `DomGetStyleArgs` with positional `node_id: String` and optional `property: Option<String>`
- [ ] `DomSetStyleArgs` with positional `node_id: String` and `style: String`
- [ ] `DomSetTextArgs` with positional `node_id: String` and `text: String`
- [ ] `DomNodeIdArgs` (shared) with positional `node_id: String` — used by GetText, GetHtml, Remove, Parent, Children, Siblings
- [ ] `DomTreeArgs` with optional `--depth` (u32) and `--root` (String) flags
- [ ] `Command::Dom` variant changed from bare `Dom` to `Dom(DomArgs)`
- [ ] Each variant has descriptive help text (`long_about`, `after_long_help` with examples)
- [ ] `cargo check` passes

**Notes**: Follow the `PageArgs`/`PageCommand` pattern. The `Dom` variant's top-level help text should be updated to remove the "not yet implemented" caveat and list all subcommands.

### T002: Add error constructors for DOM-specific errors

**File(s)**: `src/error.rs`
**Type**: Modify
**Depends**: None
**Acceptance**:
- [ ] `AppError::node_not_found(id: &str)` — exit code 3 (TargetError)
- [ ] `AppError::attribute_not_found(name: &str, node_id: &str)` — exit code 1 (GeneralError)
- [ ] `AppError::no_parent()` — exit code 3 (TargetError), message: "Element has no parent (document root)"
- [ ] `cargo check` passes

### T003: Create dom.rs module scaffold with execute_dom dispatcher and helpers

**File(s)**: `src/dom.rs` (create), `src/main.rs` (modify)
**Type**: Create + Modify
**Depends**: T001, T002
**Acceptance**:
- [ ] `src/dom.rs` created with `pub async fn execute_dom(global: &GlobalOpts, args: &DomArgs) -> Result<(), AppError>`
- [ ] `execute_dom` matches on `DomCommand` and dispatches to stub functions that return `Err(AppError::not_implemented("dom <subcommand>"))`
- [ ] `mod dom;` added to `src/main.rs`
- [ ] `Command::Dom(args) => dom::execute_dom(&global, args).await` in main.rs dispatch
- [ ] Helper functions scaffolded: `setup_session`, `resolve_node`, `describe_element`, `get_document_root`
- [ ] Output structs defined: `DomElement`, `AttributeResult`, `TextResult`, `HtmlResult`, `MutationResult`, `StyleResult`, `StylePropertyResult`
- [ ] `cargo check` passes

---

## Phase 2: Backend Implementation

### T004: Implement resolve_node helper and setup_session

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T003
**Acceptance**:
- [ ] `resolve_node(session, target)` handles three formats:
  - Integer string (e.g., `"42"`) → parsed to i64, validated via `DOM.describeNode`
  - UID (e.g., `"s3"`) → read snapshot state → `backendNodeId` → `DOM.describeNode` → `nodeId`
  - CSS selector (e.g., `"css:h1"`) → `DOM.getDocument` → `DOM.querySelector` → `nodeId`
- [ ] Returns `nodeId` (i64) on success, appropriate `AppError` on failure
- [ ] `setup_session` follows the `page.rs` pattern: resolve connection → resolve target → CDP connect → create session → apply emulate state
- [ ] `get_document_root` calls `DOM.getDocument` and returns root `nodeId`
- [ ] `describe_element(session, node_id)` calls `DOM.describeNode` and builds `DomElement` with tag, attributes, textContent
- [ ] Reuses `is_uid` from `src/form.rs` (or reimplements the same check)
- [ ] `cargo check` passes

### T005: Implement dom select (CSS and XPath)

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] CSS selection: `DOM.getDocument` → `DOM.querySelectorAll(nodeId, selector)` → describe each node
- [ ] XPath selection: `DOM.performSearch(query)` → `DOM.getSearchResults(searchId, 0, resultCount)` → describe each → `DOM.discardSearchResults(searchId)`
- [ ] Returns `Vec<DomElement>` as JSON array via `print_output`
- [ ] Empty selector match returns `[]` with exit code 0
- [ ] `cargo test --lib` passes for any unit tests

### T006: Implement dom get-attribute, get-text, get-html

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `get-attribute`: resolve node → `DOM.getAttributes(nodeId)` → find named attribute in flat array → output `AttributeResult`
- [ ] `get-attribute` returns `attribute_not_found` error if the attribute doesn't exist on the node
- [ ] `get-text`: resolve node → `DOM.resolveNode(nodeId)` → `Runtime.callFunctionOn(objectId, "function() { return this.textContent; }")` → output `TextResult`
- [ ] `get-html`: resolve node → `DOM.getOuterHTML(nodeId)` → output `HtmlResult`
- [ ] Invalid nodeId returns exit code 3 (TargetError) for all three commands
- [ ] `cargo check` passes

### T007: Implement dom set-attribute, set-text, remove

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `set-attribute`: resolve node → `DOM.setAttributeValue(nodeId, name, value)` → output `MutationResult`
- [ ] `set-text`: resolve node → `DOM.resolveNode(nodeId)` → `Runtime.callFunctionOn(objectId, "function() { this.textContent = arguments[0]; }", args: [text])` → output `MutationResult`
- [ ] `remove`: resolve node → `DOM.removeNode(nodeId)` → output `MutationResult` with `removed: true`
- [ ] All mutations return `{"success": true, "nodeId": N, ...}` on success
- [ ] Invalid nodeId returns exit code 3 for all three commands
- [ ] `cargo check` passes

### T008: Implement dom get-style and set-style

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `get-style` (all): resolve node → `CSS.getComputedStyleForNode(nodeId)` → convert `computedStyle[]` array of `{name, value}` to object → output `StyleResult`
- [ ] `get-style` (single property): filter computed styles by property name → output `StylePropertyResult`
- [ ] `set-style`: resolve node → `DOM.setAttributeValue(nodeId, "style", css_text)` → output `MutationResult`
- [ ] CSS domain enabled via `managed.ensure_domain("CSS")` before style commands
- [ ] `cargo check` passes

### T009: Implement dom parent, children, siblings, tree

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T004
**Acceptance**:
- [ ] `parent`: resolve node → `DOM.describeNode(nodeId)` → read `parentId` → `describe_element(parentId)` → output single `DomElement`
- [ ] `parent` of root element (parentId 0 or missing) returns `no_parent` error (exit code 3)
- [ ] `children`: resolve node → `DOM.describeNode(nodeId, depth: 1)` → iterate `node.children[]` → filter element nodes (nodeType == 1) → describe each → output `Vec<DomElement>`
- [ ] `siblings`: resolve node → get parent → get parent's children → exclude self → output `Vec<DomElement>`
- [ ] `tree` (default): `DOM.getDocument(depth: -1)` → recursive format as indented plain text with tag, key attributes, truncated textContent
- [ ] `tree --depth N`: limit traversal to N levels, show `...` for truncated subtrees
- [ ] `tree --root <target>`: resolve target via `resolve_node` → use as tree root instead of document
- [ ] `tree` output: plain text by default; if `--json`/`--pretty` flag is set, output as nested JSON
- [ ] `cargo check` passes

---

## Phase 3: Integration

### T010: Update CLI help text and examples

**File(s)**: `src/cli/mod.rs`, `src/examples.rs`
**Type**: Modify
**Depends**: T005, T006, T007, T008, T009
**Acceptance**:
- [ ] `Dom` variant `long_about` updated: describes all subcommands, removes "not yet implemented" caveat
- [ ] `Dom` variant `after_long_help` updated with working examples for select, get-attribute, get-text, tree
- [ ] `src/examples.rs` dom entry updated: description changed from "not yet implemented", examples replaced with real subcommand invocations
- [ ] Existing unit test in `examples.rs` (line 530: `assert!(names.contains(&"dom"))`) still passes
- [ ] `cargo check` passes

### T011: Manual smoke test against headless Chrome

**File(s)**: (no file changes)
**Type**: Verify
**Depends**: T005, T006, T007, T008, T009, T010
**Acceptance**:
- [ ] Build: `cargo build`
- [ ] Connect: `./target/debug/chrome-cli connect --launch --headless`
- [ ] Navigate: `./target/debug/chrome-cli navigate https://example.com`
- [ ] `dom select "h1"` returns JSON array with h1 element
- [ ] `dom select --xpath "//h1"` returns same result
- [ ] `dom get-attribute <nodeId> href` on a link returns the href value
- [ ] `dom get-text <nodeId>` on h1 returns "Example Domain"
- [ ] `dom get-html <nodeId>` on h1 returns outerHTML
- [ ] `dom set-attribute <nodeId> data-test "hello"` succeeds, confirmed by `get-attribute`
- [ ] `dom set-text <nodeId> "changed"` succeeds, confirmed by `get-text`
- [ ] `dom remove <nodeId>` succeeds, confirmed by `dom select` returning empty
- [ ] `dom get-style <nodeId>` returns computed styles
- [ ] `dom get-style <nodeId> display` returns single property
- [ ] `dom set-style <nodeId> "color: red"` succeeds
- [ ] `dom parent <nodeId>` returns parent element
- [ ] `dom children <nodeId>` returns child elements
- [ ] `dom siblings <nodeId>` returns sibling elements
- [ ] `dom tree` displays indented DOM tree
- [ ] `dom tree --depth 2` limits depth
- [ ] `dom select ".nonexistent"` returns `[]`
- [ ] `dom get-attribute 999999 href` returns error with exit code 3
- [ ] SauceDemo smoke test: navigate to https://www.saucedemo.com/, run `page snapshot`, then `dom select "#user-name"`, `dom get-attribute <nodeId> placeholder`
- [ ] Disconnect and kill Chrome: `./target/debug/chrome-cli connect disconnect && pkill -f 'chrome.*--remote-debugging' || true`

### T012: Verify no regressions

**File(s)**: (no file changes)
**Type**: Verify
**Depends**: T010
**Acceptance**:
- [ ] `cargo test --lib` passes (unit tests)
- [ ] `cargo test --test bdd` passes (all existing BDD tests)
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] No side effects in `page find`, `form fill`, `js exec`, or other commands that use DOM/Runtime domains

---

## Phase 4: BDD Testing

### T013: Create Gherkin feature file for dom command group

**File(s)**: `tests/features/149-dom-command-group.feature`
**Type**: Create
**Depends**: T009
**Acceptance**:
- [ ] All 22 acceptance criteria from requirements.md are scenarios
- [ ] Uses Given/When/Then format
- [ ] Background section: `Given Chrome is connected with a page loaded`
- [ ] Scenarios cover: select (CSS, XPath), get-attribute, get-text, get-html, set-attribute, set-text, remove, get-style, set-style, parent, children, siblings, tree, empty results, invalid nodeId, UID targeting, cross-validation, depth limit, root scoping, parent of root error
- [ ] Feature file is valid Gherkin syntax

### T014: Implement BDD step definitions for dom scenarios

**File(s)**: `tests/bdd.rs`
**Type**: Modify
**Depends**: T013
**Acceptance**:
- [ ] Step definitions added for all dom-specific Given/When/Then steps
- [ ] Steps follow existing `WorkflowWorld` pattern in bdd.rs
- [ ] `cargo test --test bdd` compiles and dom scenarios pass (or are appropriately tagged for CI skip if Chrome-dependent)

### T015: Add unit tests for resolve_node and format_tree

**File(s)**: `src/dom.rs`
**Type**: Modify
**Depends**: T009
**Acceptance**:
- [ ] Unit tests for `resolve_node` with integer, UID, CSS selector, and invalid inputs
- [ ] Unit tests for tree formatting with depth limits and various structures
- [ ] Unit tests for DomElement serialization
- [ ] `cargo test --lib` passes

---

## Dependency Graph

```
T001 ──┐
       ├──▶ T003 ──▶ T004 ──┬──▶ T005 ──┐
T002 ──┘                     ├──▶ T006 ──┤
                             ├──▶ T007 ──┤
                             ├──▶ T008 ──├──▶ T010 ──▶ T011 ──▶ T012
                             └──▶ T009 ──┤
                                         ├──▶ T013 ──▶ T014
                                         └──▶ T015
```

---

## Validation Checklist

Before moving to IMPLEMENT phase:

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure (per `structure.md`)
- [x] BDD test tasks included (T013, T014)
- [x] Manual smoke test task included (T011)
- [x] No circular dependencies
- [x] Tasks are in logical execution order
