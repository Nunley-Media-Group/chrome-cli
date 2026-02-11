# Tasks: Cross-Platform Release Pipeline

**Issue**: #2
**Date**: 2026-02-10
**Status**: Planning
**Author**: Claude (spec-driven)

---

## Summary

| Phase | Tasks | Status |
|-------|-------|--------|
| Setup | 1 | [ ] |
| CI Workflow | 1 | [ ] |
| Release Workflow | 3 | [ ] |
| Integration | 1 | [ ] |
| Testing | 1 | [ ] |
| **Total** | **7** | |

---

## Phase 1: Setup

### T001: Create .github directory structure

**File(s)**: `.github/workflows/` (directory creation)
**Type**: Create
**Depends**: None
**Acceptance**:
- [ ] `.github/workflows/` directory exists
- [ ] Directory is tracked by git

**Notes**: Simple directory scaffolding needed before workflow files can be created.

---

## Phase 2: CI Workflow

### T002: Create CI workflow

**File(s)**: `.github/workflows/ci.yml`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Workflow triggers on `push` to `main` and `pull_request` to `main`
- [ ] Single `check` job runs on `ubuntu-latest`
- [ ] Steps execute in order: checkout → toolchain install → cache → fmt check → clippy → test → build
- [ ] Uses `actions/checkout@v4`
- [ ] Uses `dtolnay/rust-toolchain@stable` (auto-detects `rust-toolchain.toml`)
- [ ] Uses `Swatinem/rust-cache@v2` for cargo caching
- [ ] Runs `cargo fmt --check`
- [ ] Runs `cargo clippy -- -D warnings`
- [ ] Runs `cargo test`
- [ ] Runs `cargo build`
- [ ] Valid YAML syntax

---

## Phase 3: Release Workflow

### T003: Create release workflow — create-release job

**File(s)**: `.github/workflows/release.yml`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Workflow triggers on `push` of tags matching `v*`
- [ ] Workflow-level `permissions: contents: write` is set
- [ ] `create-release` job runs on `ubuntu-latest`
- [ ] Extracts version from `github.ref_name`
- [ ] Creates a draft GitHub Release using `gh release create --draft`
- [ ] Outputs `version` for downstream jobs
- [ ] Valid YAML syntax

### T004: Create release workflow — build-release matrix job

**File(s)**: `.github/workflows/release.yml`
**Type**: Modify (append to file from T003)
**Depends**: T003
**Acceptance**:
- [ ] `build-release` job has `needs: create-release`
- [ ] `fail-fast: false` is set on the matrix strategy
- [ ] Matrix includes all 5 targets with correct runners:
  - `aarch64-apple-darwin` on `macos-latest`
  - `x86_64-apple-darwin` on `macos-13`
  - `x86_64-unknown-linux-gnu` on `ubuntu-latest`
  - `aarch64-unknown-linux-gnu` on `ubuntu-24.04-arm`
  - `x86_64-pc-windows-msvc` on `windows-latest`
- [ ] Steps: checkout → toolchain install (with target) → cache → build release → strip (Unix) → create archive dir → copy binary + README + LICENSE files → archive (tar.gz Unix / zip Windows) → upload artifact
- [ ] Binary name is `chrome-cli` (or `chrome-cli.exe` on Windows)
- [ ] Archive naming: `chrome-cli-{version}-{target}.tar.gz` (Unix) or `.zip` (Windows)
- [ ] Strip step skipped on Windows
- [ ] Uses `actions/upload-artifact@v4` to upload archive

### T005: Create release workflow — publish-release job

**File(s)**: `.github/workflows/release.yml`
**Type**: Modify (append to file from T004)
**Depends**: T004
**Acceptance**:
- [ ] `publish-release` job has `needs: build-release`
- [ ] Runs on `ubuntu-latest`
- [ ] Downloads all artifacts using `actions/download-artifact@v4`
- [ ] Generates SHA256 checksums for each archive: `shasum -a 256 <file> > <file>.sha256`
- [ ] Uploads all archives and checksums to the draft release via `gh release upload`
- [ ] Publishes the release (removes draft) via `gh release edit --draft=false`
- [ ] Uses `--generate-notes` for auto-generated release notes

---

## Phase 4: Integration

### T006: Create Dependabot configuration for GitHub Actions

**File(s)**: `.github/dependabot.yml`
**Type**: Create
**Depends**: T001
**Acceptance**:
- [ ] Configures `github-actions` package ecosystem
- [ ] Sets directory to `/`
- [ ] Sets schedule interval to `weekly`
- [ ] Valid YAML syntax

---

## Phase 5: Testing

### T007: Create BDD feature file for release pipeline

**File(s)**: `.claude/specs/cross-platform-release-pipeline/feature.gherkin`
**Type**: Create
**Depends**: T002, T005
**Acceptance**:
- [ ] Every acceptance criterion from requirements.md has a corresponding scenario
- [ ] Uses Given/When/Then format
- [ ] Includes scenarios for CI workflow, release workflow, build matrix, archiving, checksums, and GitHub Release
- [ ] Valid Gherkin syntax
- [ ] Scenarios are declarative (validate file contents/structure, not workflow execution)

---

## Dependency Graph

```
T001 ──┬──▶ T002
       │
       ├──▶ T003 ──▶ T004 ──▶ T005
       │
       └──▶ T006

T002 + T005 ──▶ T007
```

---

## Validation Checklist

- [x] Each task has single responsibility
- [x] Dependencies are correctly mapped
- [x] Tasks can be completed independently (given dependencies)
- [x] Acceptance criteria are verifiable
- [x] File paths reference actual project structure
- [x] Test tasks are included
- [x] No circular dependencies
- [x] Tasks are in logical execution order
