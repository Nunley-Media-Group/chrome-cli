# Tasks: Cargo Workspace Setup

**Issue**: #1
**Date**: 2026-02-10
**Status**: Planning
**Author**: Claude (spec-driven)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 7 | [ ] |
| Integration | 1 | [ ] |
| Testing | 1 | [ ] |
| **Total** | **9** | |

---

## Task Format

Each task follows this structure:

```
### T[NNN]: [Task Title]

**File(s)**: `{layer}/path/to/file`
**Type**: Create | Modify | Delete
**Depends**: T[NNN], T[NNN] (or None)
**Acceptance**:
- [ ] [Verifiable criterion 1]
- [ ] [Verifiable criterion 2]

**Notes**: [Optional implementation hints]
```

Map `{layer}/` placeholders to actual project paths using `structure.md`.

---

## Phase 1: Setup

### T001: Create .gitignore

**File(s)**: `.gitignore`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] Excludes `target/`
- [ ] Excludes editor temps (`*.swp`, `*~`, `.idea/`, `.vscode/`)
- [ ] Excludes OS files (`.DS_Store`, `Thumbs.db`)

### T002: Create rust-toolchain.toml

**File(s)**: `rust-toolchain.toml`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] Pins `channel = "1.93.0"` (current pinned stable toolchain)

### T003: Create rustfmt.toml

**File(s)**: `rustfmt.toml`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] Sets `edition = "2024"`

### T004: Create license files

**File(s)**: `LICENSE-MIT`, `LICENSE-APACHE`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `LICENSE-MIT` contains valid MIT license text with copyright holder
- [ ] `LICENSE-APACHE` contains full Apache License 2.0 text

### T005: Create Cargo.toml

**File(s)**: `Cargo.toml`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `[workspace]` section with `resolver = "3"`
- [ ] `[package]` section with name, version `0.1.0`, edition `2024`
- [ ] Metadata: authors, license, repository, description, keywords, categories
- [ ] `[lints.clippy]` section with `all = deny` and `pedantic = warn`

### T006: Create src/main.rs

**File(s)**: `src/main.rs`
**Type**: Create
**Depends**: T005
**Acceptance**:
- [ ] Uses `env!("CARGO_PKG_NAME")` and `env!("CARGO_PKG_VERSION")`
- [ ] Prints `agentchrome 0.1.0` format to stdout
- [ ] Compiles without warnings

### T007: Create README.md

**File(s)**: `README.md`
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] Contains project name as heading
- [ ] Contains one-line description
- [ ] Contains "under construction" notice
- [ ] Mentions license

---

## Phase 2: Backend Implementation

### T003: [Data access layer]

**File(s)**: `{data-layer}/repositories/...` or `{data-layer}/data/...`
**Type**: Create
**Depends**: T001, T002
**Acceptance**:
- [ ] All CRUD operations implemented
- [ ] Uses parameterized queries (SQL injection safe)
- [ ] Error handling for data access failures
- [ ] Unit tests pass

### T004: [Business logic layer]

**File(s)**: `{business-layer}/services/...`
**Type**: Create
**Depends**: T003
**Acceptance**:
- [ ] Business logic implemented per design
- [ ] Input validation
- [ ] Error handling with appropriate error types
- [ ] Unit tests pass

### T005: [Request handler / Controller]

**File(s)**: `{entry-layer}/controllers/...` or `{entry-layer}/handlers/...`
**Type**: Create
**Depends**: T004
**Acceptance**:
- [ ] All endpoints/handlers implemented per API spec
- [ ] Request validation
- [ ] Proper response codes/formats
- [ ] Response format matches spec

### T006: [Route registration / Endpoint wiring]

**File(s)**: `{entry-layer}/routes/...`
**Type**: Create or Modify
**Depends**: T005
**Acceptance**:
- [ ] Routes/endpoints registered with correct paths
- [ ] Auth/middleware applied where needed
- [ ] Endpoints accessible and responding

---

## Phase 3: Frontend Implementation

### T007: [Client-side model]

**File(s)**: `{presentation-layer}/models/...`
**Type**: Create
**Depends**: T002
**Acceptance**:
- [ ] Model matches API response schema
- [ ] Serialization/deserialization works
- [ ] Immutable with update method (if applicable)
- [ ] Unit tests for serialization

### T008: [Client-side service / API client]

**File(s)**: `{presentation-layer}/services/...`
**Type**: Create
**Depends**: T007
**Acceptance**:
- [ ] All API calls implemented
- [ ] Error handling with typed exceptions
- [ ] Uses project's HTTP client pattern
- [ ] Unit tests pass

### T009: [State management]

**File(s)**: `{presentation-layer}/state/...` or `{presentation-layer}/providers/...`
**Type**: Create
**Depends**: T008
**Acceptance**:
- [ ] State class defined (immutable if applicable)
- [ ] Loading/error states handled
- [ ] State transitions match design spec
- [ ] Unit tests for state transitions

### T010: [UI components]

**File(s)**: `{presentation-layer}/components/...` or `{presentation-layer}/widgets/...`
**Type**: Create
**Depends**: T009
**Acceptance**:
- [ ] Components match design specs
- [ ] Uses project's design tokens (no hardcoded values)
- [ ] Loading/error/empty states
- [ ] Component tests pass

### T011: [Screen / Page]

**File(s)**: `{presentation-layer}/screens/...` or `{presentation-layer}/pages/...`
**Type**: Create
**Depends**: T010
**Acceptance**:
- [ ] Screen layout matches design
- [ ] State management integration working
- [ ] Navigation implemented

---

## Phase 2: Integration

### T008: Verify build, lint, format, and test

**File(s)**: (none — verification only)
**Type**: Verify
**Depends**: T001, T002, T003, T004, T005, T006, T007
**Acceptance**:
- [ ] `cargo build` succeeds with zero warnings
- [ ] `cargo clippy -- -D warnings` passes with zero warnings
- [ ] `cargo fmt --check` passes with no formatting issues
- [ ] `cargo test` passes

---

## Phase 3: Testing (BDD)

### T009: Create BDD feature file

**File(s)**: `specs/1-cargo-workspace-setup/feature.gherkin`
**Type**: Create
**Depends**: T008
**Acceptance**:
- [ ] All 10 acceptance criteria from requirements.md are scenarios
- [ ] Uses Given/When/Then format
- [ ] Valid Gherkin syntax

---

## Dependency Graph

```
T001 (gitignore)     ──┐
T002 (toolchain)     ──┤
T003 (rustfmt)       ──┤
T004 (licenses)      ──┼──▶ T008 (verify) ──▶ T009 (BDD feature)
T005 (Cargo.toml) ──┬──┤
                    │  │
T006 (main.rs) ◀───┘  │
T007 (README)       ───┘
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths are specific
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
