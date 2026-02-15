# Design: Cross-Platform Release Pipeline

**Issue**: #2
**Date**: 2026-02-10
**Status**: Draft
**Author**: Claude (spec-driven)

---

## Overview

This feature creates two GitHub Actions workflows: a CI workflow for PR/push validation and a release workflow for cross-platform binary distribution. Since the repository is public, all 5 target platforms can be built natively on GitHub-hosted runners — including Linux ARM64 on the free `ubuntu-24.04-arm` runner — eliminating the need for cross-compilation tools like `cross` or `cargo-zigbuild`.

The design follows patterns established by mature Rust CLI projects (ripgrep, bat, fd): a matrix-based build strategy with per-platform archiving, SHA256 checksums, and automated GitHub Release publishing.

---

## Architecture

### Workflow Diagram

```
                    ┌─────────────────┐
                    │   CI Workflow    │
                    │  (ci.yml)       │
                    └────────┬────────┘
                             │ Triggered by: PR, push to main
                             ▼
                    ┌─────────────────┐
                    │  Single Job:    │
                    │  fmt → clippy   │
                    │  → test → build │
                    │  (ubuntu-latest)│
                    └─────────────────┘


                    ┌──────────────────┐
                    │ Release Workflow  │
                    │ (release.yml)    │
                    └────────┬─────────┘
                             │ Triggered by: tag push v*
                             ▼
                    ┌──────────────────┐
                    │  create-release  │
                    │  (ubuntu-latest) │
                    │  Creates draft   │
                    │  GitHub Release  │
                    └────────┬─────────┘
                             │ needs
                             ▼
              ┌──────────────────────────────┐
              │       build-release          │
              │       (matrix: 5 targets)    │
              ├──────────────────────────────┤
              │ macOS ARM    │ macos-latest   │
              │ macOS Intel  │ macos-13       │
              │ Linux x86_64 │ ubuntu-latest  │
              │ Linux ARM64  │ ubuntu-24.04-arm│
              │ Windows      │ windows-latest │
              └──────────────┬───────────────┘
                             │ uploads artifacts
                             ▼
                    ┌──────────────────┐
                    │ publish-release  │
                    │ (ubuntu-latest)  │
                    │ Downloads all    │
                    │ artifacts, gen   │
                    │ checksums, attach│
                    │ to release       │
                    └──────────────────┘
```

### Data Flow

```
1. Developer pushes a v* tag (e.g., v0.1.0)
2. Release workflow triggers
3. create-release job creates a draft GitHub Release, extracts version from tag
4. build-release matrix runs 5 parallel jobs on native runners
5. Each job: install toolchain → build release binary → strip → archive → upload artifact
6. publish-release job downloads all artifacts, generates SHA256 checksums, uploads to GitHub Release
7. Release is published (no longer draft)
```

---

## CI Workflow Design (`.github/workflows/ci.yml`)

### Trigger

```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
```

### Single Job: `check`

Runs on `ubuntu-latest` with these sequential steps:

1. Checkout code
2. Install Rust toolchain (uses `rust-toolchain.toml` automatically)
3. Cache cargo registry and target directory
4. `cargo fmt --check`
5. `cargo clippy -- -D warnings`
6. `cargo test`
7. `cargo build` (build verification)

**Rationale for single job**: The project is small; splitting into parallel jobs adds overhead (checkout + toolchain install per job) that exceeds any time saved. A single sequential job on ubuntu-latest is simpler and faster for projects of this size.

---

## Release Workflow Design (`.github/workflows/release.yml`)

### Trigger

```yaml
on:
  push:
    tags: ["v*"]
```

### Job 1: `create-release`

- Runs on `ubuntu-latest`
- Extracts version from `${{ github.ref_name }}` (the tag, e.g. `v0.1.0`)
- Creates a draft GitHub Release using `gh release create --draft`
- Outputs: `version` (the tag name)

### Job 2: `build-release` (matrix)

**Build Matrix:**

| Name | Runner | Target | Binary Extension |
|------|--------|--------|-----------------|
| macOS ARM64 | `macos-latest` | `aarch64-apple-darwin` | (none) |
| macOS x86_64 | `macos-13` | `x86_64-apple-darwin` | (none) |
| Linux x86_64 | `ubuntu-latest` | `x86_64-unknown-linux-gnu` | (none) |
| Linux ARM64 | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | (none) |
| Windows x86_64 | `windows-latest` | `x86_64-pc-windows-msvc` | `.exe` |

**`fail-fast: false`** — so a failure on one platform does not cancel others.

**Steps per matrix entry:**

1. Checkout code
2. Install Rust toolchain with target via `dtolnay/rust-toolchain`
3. Build: `cargo build --release --target ${{ matrix.target }}`
4. Strip binary (Unix only): `strip` to reduce size
5. Create archive directory: `chrome-cli-{version}-{target}/`
6. Copy binary + README.md + LICENSE files into archive directory
7. Archive: `tar czf` for Unix, `7z a` (zip) for Windows
8. Upload archive as GitHub Actions artifact

### Job 3: `publish-release`

- Runs on `ubuntu-latest`, needs `build-release`
- Downloads all artifacts from build-release
- Generates SHA256 checksums: `shasum -a 256 <file> > <file>.sha256`
- Uploads all archives + checksums to the draft release via `gh release upload`
- Publishes the release (removes draft status) via `gh release edit --draft=false`
- Generates release notes via `--generate-notes` flag on the release

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: cross for ARM64** | Use `cross` Docker-based cross-compilation | Battle-tested; works on private repos | Docker overhead; slower; unnecessary for public repos | Rejected — native ARM runner is free and faster |
| **B: cargo-zigbuild** | Use Zig-based cross-compilation | No Docker; lightweight | Less proven; issues with some deps; unnecessary here | Rejected — native runners available |
| **C: Native runners** | Use platform-specific GitHub-hosted runners | Fastest; simplest; no cross-compilation tooling | Requires public repo for free ARM runners | **Selected** |
| **D: cargo-dist** | Use `axodotdev/cargo-dist` for release automation | All-in-one; handles many details | Opinionated; less control; additional dependency | Rejected — manual approach matches established Rust project patterns |
| **E: softprops/action-gh-release** | Third-party action for release creation | Popular; feature-rich | Third-party dependency; less control | Rejected — `gh` CLI is simpler and already available |
| **F: gh CLI for releases** | Use GitHub's native CLI | No third-party deps; full control; always available on runners | More shell scripting | **Selected** |
| **G: taiki-e/upload-rust-binary-action** | All-in-one build+archive+upload | Simple config | Opaque; less control over archiving | Rejected — manual approach is more transparent |

---

## Action Version Pinning Strategy

All actions will be pinned by tag with a comment indicating the version. For this project, tag-based pinning (`@v4`) is acceptable for first-party actions since:
- The project is public and low-risk
- Dependabot will be configured to watch for updates

| Action | Pin Style | Rationale |
|--------|-----------|-----------|
| `actions/checkout` | `@v4` | First-party; widely trusted |
| `actions/upload-artifact` | `@v4` | First-party |
| `actions/download-artifact` | `@v4` | First-party |
| `dtolnay/rust-toolchain` | `@stable` | Well-known; auto-selects stable toolchain |
| `Swatinem/rust-cache` | `@v2` | Popular Rust caching action; more effective than manual cache |

---

## Security Considerations

- [x] **Action pinning**: All actions pinned by tag; Dependabot configured for updates
- [x] **Permissions**: Workflows use minimal `permissions:` — `contents: write` only on release workflow
- [x] **No secrets in logs**: Binary builds do not require secrets; `GITHUB_TOKEN` is the only secret used (auto-provided)
- [x] **No code signing**: Out of scope; binaries are unsigned

---

## Performance Considerations

- [x] **Cargo caching**: `Swatinem/rust-cache` caches the cargo registry and target directory to speed up builds
- [x] **Parallel matrix**: All 5 targets build concurrently
- [x] **Binary stripping**: `strip` on Unix targets reduces binary size
- [x] **fail-fast disabled**: One failure does not cancel other platform builds

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CI Workflow | Workflow validation | YAML syntax via `actionlint` (optional) |
| CI Workflow | Functional | Verify fmt/clippy/test/build pass on PRs |
| Release Workflow | Functional | Verify all 5 binaries built, archived, checksummed, and published |
| Workflow files | BDD | Gherkin scenarios validating workflow file structure and configuration |

**Note**: Full end-to-end testing of GitHub Actions workflows requires actually pushing tags and running the workflows on GitHub. The BDD tests will validate the workflow file contents (YAML structure, correct targets, correct runners, etc.) rather than executing the workflows.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| `ubuntu-24.04-arm` runner unavailable or slow | Low | Med | Fall back to `cross` for ARM64 |
| macOS runner image changes break builds | Low | Med | Pin runner version if needed (e.g., `macos-13` not `macos-latest` for Intel) |
| Rust toolchain mismatch between CI and release | Low | Low | Both workflows use `rust-toolchain.toml` auto-detection |
| Large binary size | Med | Low | `strip` + release profile optimizations |

---

## File Structure

```
.github/
├── workflows/
│   ├── ci.yml          # CI: fmt, clippy, test, build
│   └── release.yml     # Release: build matrix, archive, checksums, publish
└── dependabot.yml      # Auto-update action versions
```

---

## Validation Checklist

- [x] Architecture follows established Rust project patterns (ripgrep, bat, fd)
- [x] All workflow configurations documented
- [x] Build matrix covers all 5 required targets
- [x] Runner selection matches target architecture
- [x] Archive and checksum strategy defined
- [x] Security considerations addressed
- [x] Performance impact analyzed
- [x] Testing strategy defined
- [x] Alternatives considered and documented
- [x] Risks identified with mitigations
