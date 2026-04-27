# Root Cause Analysis: Detect Codex runtime env vars for active skill-notice scoping

**Issue**: #278
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (AI-assisted)

---

## Root Cause

The defect is in the split between installed-target detection and active-runtime detection. Codex install targeting already supports a passive `~/.codex` directory and non-empty `CODEX_HOME`, but stale-skill notice scoping intentionally uses active-runtime detection only. That distinction is correct: installed files and passive config directories should not imply the current command is running inside that tool.

The active-runtime path is too narrow for Codex. `detect_active_tool_with` classifies Codex as active only when `CODEX_HOME` is present and non-empty. Real Codex sessions can expose `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, and `CODEX_THREAD_ID` without `CODEX_HOME`. In that state, active-tool detection returns `None`. `src/skill_check.rs::stale_tools_for_notice` then follows its intended no-active fallback and scans every installed tool, which produces noisy stale notices for inactive tools such as `claude-code`.

The stale-notice scoping logic is already shaped correctly. When `stale_tools_for_notice` receives an active tool, it checks only that tool's inventory row and emits either a single-tool notice or no notice. The minimal fix is therefore to make Codex active-runtime detection recognize the actual Codex runtime signals, while preserving the current passive config-directory behavior for install targeting and the all-tools fallback for plain terminals.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/skill.rs` | 230-238 | `detect_tool()` recognizes Codex only through non-empty `CODEX_HOME` before falling back to parent process and config directories. |
| `src/skill.rs` | 281-311 | `detect_active_tool_with()` recognizes active Codex only through non-empty `CODEX_HOME`; this is the direct input to stale-notice scoping. |
| `src/skill.rs` | 338-346 | `tool_detected_with()` keeps Codex install targeting tied to `CODEX_HOME` or `~/.codex`; this passive path must remain distinct from active-runtime detection. |
| `src/skill_check.rs` | 159-180 | `stale_tools_for_notice()` scopes to the active tool when present, otherwise falls back to every stale installed skill. |
| `src/skill.rs` | 1435-1465 | Existing unit tests cover `CODEX_HOME`, passive directory exclusion from active detection, and empty `CODEX_HOME`, but not Codex runtime env vars. |
| `tests/bdd.rs` | 5328-5350, 5474-5495, 6160-6182 | Existing BDD helpers can set runtime env vars, plant stale/current skills in temp homes, and assert notice cardinality. |

### Triggering Conditions

- A command runs inside a Codex session that exposes `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, or `CODEX_THREAD_ID`.
- `CODEX_HOME` is absent or empty, so the current active Codex check fails.
- At least one unrelated supported tool has a stale AgentChrome skill installed.
- The command succeeds, so `emit_stale_notice_if_any` reaches notice selection and formats the registry-wide fallback result.

---

## Fix Strategy

### Approach

Add a small Codex-runtime signal helper in `src/skill.rs` and use it where AgentChrome decides whether the current runtime is Codex. The helper should return true for non-empty `CODEX_HOME` and for the observed runtime keys `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, and `CODEX_THREAD_ID`. Empty values should not activate Codex, matching existing `CODEX_HOME` behavior.

Preserve the existing priority order by keeping the Codex check in the same location after Gemini and before parent-process checks. Preserve passive install targeting by leaving `tool_detected_with`'s `~/.codex` directory behavior intact and by keeping `detect_active_tool_with(&[], None)` as `None`, even when a temp home contains `.codex`. The stale-notice code should not need semantic changes beyond any test-visible helper exposure.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/skill.rs` | Add a helper such as `env_has_codex_runtime_signal(env)` that recognizes non-empty `CODEX_HOME`, `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, or `CODEX_THREAD_ID`. | Centralizes Codex runtime identity and avoids repeating the allowlist. |
| `src/skill.rs` | Use the helper in `detect_active_tool_with()` and the std-env equivalent in `detect_tool()`. | Fixes active Codex classification for real Codex sessions and keeps bare `skill install` auto-detection aligned with active runtime identity. |
| `src/skill.rs` | Update the Codex registry detection description if needed to name runtime env vars in addition to `CODEX_HOME` / `~/.codex`. | Keeps `agentchrome skill list` discovery truthful. |
| `src/skill.rs` tests | Add unit tests for `CODEX_CI`, `CODEX_MANAGED_BY_NPM`, and `CODEX_THREAD_ID`; add a priority test showing `CLAUDE_CODE` still wins over a Codex runtime signal. | Guards the exact defect and the key priority no-regression behavior. |
| `tests/features/278-detect-codex-runtime-env-vars-for-active-skill-notice-scoping.feature` | Add `@regression` scenarios for AC1-AC3. | Captures the user-visible failure and related scoping guarantees. |
| `tests/bdd.rs` | Extend `SkillWorld` Given/Then steps for Codex runtime env vars without `CODEX_HOME`, current Codex skill + inactive stale skill, and stale Codex + inactive stale skill assertions. | Exercises the real CLI command path with temp homes and isolated env. |

### Blast Radius

- **Direct impact**: `src/skill.rs` active tool detection for Codex and bare `skill install` auto-detection inside Codex sessions.
- **Indirect impact**: `src/skill_check.rs::emit_stale_notice_if_any` will now receive `Some(codex)` for more real Codex sessions and therefore scope notices to the Codex inventory row.
- **Risk level**: Low. The change is an additive allowlist of Codex runtime signals and does not alter output schemas, file-writing semantics, or the stale-notice formatter.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| A future unrelated `CODEX_*` env var accidentally marks Codex active. | Low | Use an explicit allowlist of observed runtime keys instead of broad prefix matching. |
| Higher-priority tool signals stop winning when both tool environments are present. | Low | Preserve detection order and add a unit test with `CLAUDE_CODE` plus a Codex runtime signal. |
| Passive `.codex` directory existence starts suppressing all-tools notices in plain terminals. | Low | Keep config-directory detection out of `detect_active_tool_with`; preserve the existing passive-directory unit test. |
| BDD temp-home fixtures accidentally rely on real user `CODEX_HOME`. | Medium | Use `env_clear()`, explicitly omit `CODEX_HOME`, and plant skills only inside test temp homes. |
| Notice wording changes unexpectedly while fixing scoping. | Low | Assert notice cardinality and included/excluded tool names without reformatting `format_notice`. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Match any `CODEX_*` environment variable. | Treat the whole Codex namespace as active runtime identity. | Too broad; future or user-defined variables could suppress legitimate all-tools fallback notices. |
| Treat `.codex` directory existence as active Codex for stale notices. | Reuse passive install-target detection for active scoping. | Rejected by existing design: passive config directories indicate install targets, not the currently active runtime. |
| Change `stale_tools_for_notice` to hide inactive tools by default. | Avoid dependence on active-tool detection. | Breaks the specified plain-terminal all-tools fallback and weakens the actionable stale-notice workflow. |
| Suppress stale notices entirely inside Codex. | Remove the noisy symptom. | Loses useful stale Codex skill notices and conflicts with the existing staleness feature. |

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #278 | 2026-04-27 | Initial defect design |

---

## Validation Checklist

- [x] Root cause is identified with specific code references
- [x] Fix is minimal - no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)
