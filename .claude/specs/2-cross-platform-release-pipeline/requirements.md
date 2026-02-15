# Requirements: Cross-Platform Release Pipeline

**Issue**: #2
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## User Story

**As a** developer contributing to chrome-cli
**I want** automated CI/CD pipelines that build, test, lint, and release cross-platform binaries
**So that** every PR is validated and releases produce optimized standalone binaries for all supported platforms

---

## Background

chrome-cli must be distributed as a standalone binary with no runtime dependencies. The project needs two GitHub Actions workflows: a CI workflow that validates every PR and push to main (format, lint, test, build), and a release workflow triggered by version tags that builds optimized binaries for 5 target platforms, archives them, generates checksums, and publishes them as GitHub Releases.

---

## Acceptance Criteria

### AC1: CI workflow runs on PRs and pushes to main

**Given** the repository has `.github/workflows/ci.yml`
**When** a PR is opened or a push to `main` occurs
**Then** the CI workflow runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, and a build verification on Ubuntu latest
**And** all steps pass with zero warnings or errors

### AC2: Release workflow triggered by version tags

**Given** the repository has `.github/workflows/release.yml`
**When** a tag matching `v*` is pushed (e.g., `v0.1.0`)
**Then** the release workflow is triggered
**And** it builds optimized release binaries for all 5 target platforms

### AC3: Release builds all 5 target platforms

**Given** the release workflow has been triggered
**When** the build matrix completes
**Then** binaries exist for:
  - `aarch64-apple-darwin` (macOS ARM / Apple Silicon)
  - `x86_64-apple-darwin` (macOS Intel)
  - `x86_64-pc-windows-msvc` (Windows)
  - `x86_64-unknown-linux-gnu` (Linux x86_64)
  - `aarch64-unknown-linux-gnu` (Linux ARM64)

### AC4: macOS builds use correct runners

**Given** the release workflow builds macOS targets
**When** the `aarch64-apple-darwin` target builds
**Then** it runs on `macos-latest` (ARM runner)
**And** when the `x86_64-apple-darwin` target builds, it runs on `macos-13` (Intel runner)

### AC5: Linux ARM64 uses cross-compilation

**Given** the release workflow builds the `aarch64-unknown-linux-gnu` target
**When** the build step executes
**Then** it uses cross-compilation tooling (e.g., `cross` or `cargo-zigbuild`)
**And** the resulting binary is a valid aarch64 Linux ELF binary

### AC6: Windows build uses Windows runner

**Given** the release workflow builds the `x86_64-pc-windows-msvc` target
**When** the build step executes
**Then** it runs on `windows-latest`
**And** the resulting binary is a valid Windows PE executable

### AC7: Archives use correct format per platform

**Given** release binaries have been built
**When** they are archived
**Then** Unix targets (macOS, Linux) are packaged as `.tar.gz`
**And** Windows targets are packaged as `.zip`
**And** the archive naming follows: `chrome-cli-{version}-{target}.{ext}`

### AC8: SHA256 checksums generated

**Given** release archives have been created
**When** the checksum step runs
**Then** a SHA256 checksum file is generated for each archive
**And** the checksums are verifiable against the actual archive contents

### AC9: GitHub Release created with all artifacts

**Given** all archives and checksums have been generated
**When** the publish step runs
**Then** a GitHub Release is created for the tag
**And** all archive files and checksum files are attached as release assets
**And** release notes are auto-generated from git log or conventional commits

### AC10: Binary naming convention followed

**Given** a release binary is built for a target
**When** it is archived
**Then** the archive name matches the pattern `chrome-cli-{version}-{target}.{ext}`

**Example**:
- Given: target is `x86_64-unknown-linux-gnu`, version is `v0.1.0`
- When: archived
- Then: filename is `chrome-cli-v0.1.0-x86_64-unknown-linux-gnu.tar.gz`

### AC11: Binaries are statically linked where possible

**Given** a release binary is built
**When** the build completes
**Then** the binary has minimal or no shared library dependencies
**And** specifically, OpenSSL is either statically linked or replaced by `rustls`

### Generated Gherkin Preview

```gherkin
Feature: Cross-platform release pipeline
  As a developer contributing to chrome-cli
  I want automated CI/CD pipelines
  So that every PR is validated and releases produce optimized standalone binaries

  Scenario: CI workflow validates PRs
    Given the repository has ".github/workflows/ci.yml"
    When a PR is opened or a push to main occurs
    Then cargo fmt, clippy, test, and build all pass

  Scenario: Release workflow triggered by version tag
    Given the repository has ".github/workflows/release.yml"
    When a tag matching "v*" is pushed
    Then the release workflow builds binaries for all 5 targets

  Scenario: Archives use correct format per platform
    Given release binaries have been built
    When they are archived
    Then Unix targets use tar.gz and Windows targets use zip

  Scenario: GitHub Release created with artifacts
    Given all archives and checksums are generated
    When the publish step runs
    Then a GitHub Release is created with all artifacts attached
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | CI workflow running fmt, clippy, test, build on PRs and main pushes | Must | `.github/workflows/ci.yml` |
| FR2 | Release workflow triggered by `v*` tags | Must | `.github/workflows/release.yml` |
| FR3 | Build matrix for 5 target platforms | Must | See AC3 for target list |
| FR4 | Correct runner selection per platform | Must | macOS-latest for ARM, macos-13 for Intel |
| FR5 | Cross-compilation for Linux ARM64 | Must | `cross` or `cargo-zigbuild` |
| FR6 | Platform-appropriate archive format (tar.gz / zip) | Must | |
| FR7 | SHA256 checksums for all archives | Must | |
| FR8 | GitHub Release creation with all artifacts | Must | Auto-generated release notes |
| FR9 | Binary naming convention `chrome-cli-{version}-{target}.{ext}` | Must | |
| FR10 | Static linking where possible | Should | Use `rustls` over OpenSSL |
| FR11 | macOS universal/fat binaries | Won't (this release) | Nice-to-have per issue notes |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Reliability** | CI must not have flaky tests; release must be reproducible |
| **Performance** | CI should complete in < 10 minutes for fast PR feedback |
| **Security** | Workflows use pinned action versions; no secrets leaked in logs |
| **Portability** | Release binaries run on target platforms with no runtime dependencies |

---

## Dependencies

### Internal Dependencies
- Issue #1 (Cargo workspace setup) — must be merged first

### External Dependencies
- GitHub Actions runners (ubuntu-latest, macos-latest, macos-13, windows-latest)
- Cross-compilation tooling for Linux ARM64

### Blocked By
- Nothing (issue #1 is already merged)

---

## Out of Scope

- macOS universal (fat) binaries
- Code signing or notarization
- Automated version bumping
- Publishing to crates.io
- Homebrew formula or other package manager distribution
- Smoke testing release binaries on each platform
- Docker image builds

---

## Open Questions

- None — the issue is well-specified with clear acceptance criteria

---

## Validation Checklist

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
