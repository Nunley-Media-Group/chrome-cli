# Requirements: Cargo Workspace Setup

**Issue**: #1
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## User Story

**As a** developer contributing to chrome-cli
**I want** a properly configured Rust project with workspace setup, linting, and project metadata
**So that** I can build, test, and lint the project with zero warnings from day one

---

## Background

chrome-cli is a standalone CLI tool that provides browser automation and inspection capabilities via the Chrome DevTools Protocol (CDP) over WebSocket. This is the foundational setup issue — everything else depends on having a clean, CI-ready Rust project structure. The project uses edition 2024 and resolver 3, starting as a single crate with the option to split into a workspace later.

---

## Acceptance Criteria

### AC1: Cargo.toml with workspace configuration and project metadata

**Given** a freshly cloned repository
**When** I inspect `Cargo.toml`
**Then** it contains workspace configuration with `resolver = "3"`
**And** it contains project metadata: name (`chrome-cli`), version (`0.1.0`), authors, license (`MIT OR Apache-2.0`), repository URL, description, keywords, categories
**And** it uses `edition = "2024"`

### AC2: Dual license files present

**Given** a freshly cloned repository
**When** I inspect the root directory
**Then** a `LICENSE-MIT` file exists with valid MIT license text
**And** a `LICENSE-APACHE` file exists with valid Apache 2.0 license text

### AC3: Rust-appropriate .gitignore

**Given** a freshly cloned repository
**When** I inspect `.gitignore`
**Then** it excludes `target/`, `*.swp`, `.DS_Store`, and other common Rust/editor artifacts

### AC4: Formatting configuration

**Given** a freshly cloned repository
**When** I inspect `rustfmt.toml`
**Then** it contains project formatting rules
**And** `cargo fmt --check` passes with no formatting issues

### AC5: Clippy linting configuration

**Given** a freshly cloned repository
**When** I inspect `Cargo.toml` or `clippy.toml`
**Then** Clippy is configured to deny warnings in CI mode
**And** `cargo clippy -- -D warnings` passes with zero warnings

### AC6: Rust toolchain pinned

**Given** a freshly cloned repository
**When** I inspect `rust-toolchain.toml`
**Then** it pins a stable Rust version that supports edition 2024

### AC7: Minimal main.rs compiles and prints version

**Given** the project has been set up
**When** I run `cargo run -- --version`
**Then** it prints the program name and version (e.g., `chrome-cli 0.1.0`)
**And** the process exits with code 0

### AC8: Scaffold README.md

**Given** a freshly cloned repository
**When** I inspect `README.md`
**Then** it contains the project name (`chrome-cli`)
**And** it contains a one-line description
**And** it contains an "under construction" notice

### AC9: Build and clippy pass with zero warnings

**Given** the project has been set up
**When** I run `cargo build` and `cargo clippy -- -D warnings`
**Then** both commands complete successfully with zero warnings

### AC10: Tests pass

**Given** the project has been set up
**When** I run `cargo test`
**Then** the test suite passes (even if no tests exist yet)

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Cargo.toml with complete workspace and metadata configuration | Must | edition 2024, resolver 3 |
| FR2 | Dual MIT/Apache-2.0 license files | Must | Standard Rust ecosystem practice |
| FR3 | .gitignore for Rust projects | Must | target/, editor temps, OS files |
| FR4 | rustfmt.toml with formatting rules | Must | Consistent code style |
| FR5 | Clippy configuration denying warnings | Must | CI-ready linting |
| FR6 | rust-toolchain.toml pinning stable Rust | Must | Reproducible builds |
| FR7 | Minimal src/main.rs that compiles and prints version | Must | Verifiable build |
| FR8 | Scaffold README.md | Must | Project identification |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Build** | `cargo build`, `cargo clippy`, `cargo test` all pass with zero warnings/errors |
| **Reproducibility** | Pinned toolchain ensures consistent builds across machines |
| **Standards** | Follows Rust ecosystem conventions for project layout and licensing |

---

## Dependencies

### Internal Dependencies
- None (this is the first issue)

### External Dependencies
- Rust toolchain (stable, supporting edition 2024)
- Cargo package manager

### Blocked By
- Nothing

---

## Out of Scope

- Multi-crate workspace structure (start single-crate, split later)
- CI/CD pipeline (covered by issue #2)
- CLI argument parsing beyond `--version` (covered by issue #3)
- Any CDP/WebSocket functionality
- Dependency selection beyond what's needed for a minimal binary

---

## Open Questions

- None — requirements are straightforward for project scaffolding

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
