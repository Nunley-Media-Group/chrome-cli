# Verification Report: Active-Tool Stale-Skill Notice Scope

**Date**: 2026-04-27
**Issue**: #255
**Reviewer**: Codex
**Scope**: Verify active-tool scoped stale-skill notices against the amended skill-command spec

---

## Executive Summary

| Category | Score (1-5) |
|----------|-------------|
| Spec Compliance | 5 |
| Architecture (SOLID) | 4 |
| Security | 5 |
| Performance | 5 |
| Testability | 5 |
| Error Handling | 5 |
| **Overall** | 4.8 |

**Status**: Pass
**Total Issues**: 1 found, 1 fixed, 0 remaining

---

## Acceptance Criteria Verification

| AC | Description | Status | Evidence |
|----|-------------|--------|----------|
| AC31 | Active Claude Code session suppresses unrelated stale Cursor notices when Claude Code skill is current | Pass | `src/skill.rs::detect_active_tool`; `src/skill_check.rs::stale_tools_for_notice`; BDD scenario in `tests/features/skill-staleness.feature` |
| AC32 | Active stale skill emits exactly one notice naming only the active tool | Pass | `src/skill_check.rs::format_notice`; BDD scenario asserts `claude-code` is present and `cursor` is absent |
| AC33 | No active tool preserves registry-wide all-tools stale fallback | Pass | `stale_tools_for_notice(None, ...)` unit test; BDD and manual temp-home smoke aggregate `claude-code, cursor` |
| AC34 | Scoped stale notice keeps installed version, binary version, and update guidance | Pass | Existing notice formatter reused; BDD scenario verifies version details and `agentchrome skill update` guidance |

---

## Task Completion

| Task | Description | Status | Notes |
|------|-------------|--------|-------|
| T035 | Add active runtime tool detection helper | Complete | `detect_active_tool()` uses env and parent-process signals only, leaving config-dir detection to install/update discovery |
| T036 | Scope stale-skill notice to the active tool | Complete | `emit_stale_notice_if_any()` routes the detected active tool into the structured stale inventory scan |
| T037 | Add focused unit coverage for scoped stale notices | Complete | Unit tests cover active env, parent process, env priority, config-dir non-detection, active-current suppression, active-stale notice, and no-active fallback |
| T038 | Add BDD coverage for active-tool stale-notice behavior | Complete | Four scenarios cover AC31-AC34 in `tests/features/skill-staleness.feature` with temp homes and temp env signals |
| T039 | Verify active-tool stale-notice workflow | Complete | Format, build, unit, clippy, full BDD, and manual temp-home smoke all passed |

---

## Architecture Assessment

| Area | Score (1-5) | Notes |
|------|-------------|-------|
| SOLID Principles | 4 | The change reuses the existing registry and stale inventory helpers; `src/skill.rs` remains broad but follows the established command-module pattern. |
| Security | 5 | No new network, shell, credential, or privilege surface. File reads remain bounded to resolved skill paths. |
| Performance | 5 | The scan remains bounded by the supported-tool registry and only changes filtering behavior after inventory collection. |
| Testability | 5 | Active-tool selection and stale-notice filtering are covered by unit tests, BDD scenarios, and temp-home smoke tests. |
| Error Handling | 5 | Missing, unreadable, and unversioned active skill files still degrade to no notice rather than breaking the command path. |

---

## Test Coverage

- BDD scenarios: 4/4 issue #255 acceptance criteria covered and passing.
- Step definitions: Implemented in `tests/bdd.rs`.
- Unit tests: `src/skill.rs` covers active-tool detection; `src/skill_check.rs` covers scoped stale filtering.
- Test execution: `cargo test --test bdd` passed.

---

## Steering Doc Verification Gates

| Gate | Status | Evidence |
|------|--------|----------|
| Debug Build | Pass | `cargo build 2>&1` exited 0 |
| Unit Tests | Pass | `cargo test --lib 2>&1` exited 0; 255 tests passed |
| Clippy | Pass | `cargo clippy --all-targets 2>&1` exited 0 |
| Format Check | Pass | `cargo fmt --check 2>&1` exited 0 |
| Feature Exercise | Pass | Manual temp-home smoke proved active Claude Code current suppresses stale Cursor notice, and no-active fallback aggregates `claude-code, cursor` |

**Gate Summary**: 5/5 gates passed, 0 failed, 0 incomplete

---

## Fixes Applied

| Severity | Category | Location | Original Issue | Fix Applied | Routing |
|----------|----------|----------|----------------|-------------|---------|
| Medium | Testing | `tests/bdd.rs` | Generic BDD CLI runs inherited repo-local relative Cursor skill state, so stale notices contaminated unrelated stderr JSON assertions | Added `AGENTCHROME_NO_SKILL_CHECK=1` to the generic CLI runner while keeping dedicated staleness scenarios unsuppressed | direct |

## Remaining Issues

None.

---

## Recommendation

**Ready for PR**

Issue #255 acceptance criteria pass after the BDD isolation fix. No remaining verification findings.
