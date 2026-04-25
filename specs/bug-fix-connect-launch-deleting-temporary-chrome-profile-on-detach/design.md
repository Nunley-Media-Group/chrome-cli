# Root Cause Analysis: Fix connect --launch deleting temporary Chrome profile on detach

**Issue**: #265
**Date**: 2026-04-24
**Status**: Investigating
**Author**: Codex
**Related Spec**: `specs/feature-chrome-instance-discovery-and-launch/`

---

## Root Cause

The launch path had two implicit lifetime assumptions that no longer hold reliably after the toolchain/dependency refresh.

First, `launch_chrome()` creates a temporary user data directory when `LaunchConfig::user_data_dir` is `None`. The directory is owned by `ChromeProcess.temp_dir`, whose `Drop` implementation removes the directory.

After Chrome becomes ready, the `connect --launch` path calls `ChromeProcess::detach()` so Chrome should continue running after the `agentchrome connect` process exits. `detach()` correctly clears `self.child` to avoid killing the Chrome process, but it also assigns `self.temp_dir = None`. That assignment immediately drops the `TempDir` owner and deletes Chrome's profile directory while the detached Chrome process still needs it.

Second, the spawned Chrome process remains in the launcher process group. In noninteractive callers that clean up the command process group after `agentchrome connect` exits, the detached Chrome is still treated as part of the completed command and is terminated before the next AgentChrome invocation can reuse the session.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/chrome/launcher.rs` | 54-70 | `ChromeProcess::detach()` detaches process ownership and currently drops owned resources. |
| `src/chrome/launcher.rs` | 156-190 | `launch_chrome()` builds and spawns Chrome in the launcher's process group. |
| `src/main.rs` | 631-643 | `execute_launch()` calls `launch_chrome()`, then `process.detach()` before saving the session. |

### Triggering Conditions

- User runs `agentchrome connect --launch` or `agentchrome connect --launch --headless`.
- AgentChrome creates an internal temporary profile directory.
- Chrome becomes ready and the launch handle is detached.
- The temp directory owner is dropped during detach, deleting the profile under the running browser.
- The caller's process-group cleanup can still terminate the detached Chrome process.

---

## Fix Strategy

### Approach

Change `ChromeProcess::detach()` so it intentionally leaks the owned `Child` handle and `TempDir` owner when the Chrome process is detached. This matches the existing method comment: after detach, the caller no longer owns the Chrome process lifetime, and the profile directory must outlive the AgentChrome process that launched it.

Configure the spawned Chrome process as independent from the launcher process group: on Unix, call `setsid()` in `pre_exec`; on Windows, use `CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`. This keeps the browser alive when a noninteractive caller cleans up the completed `agentchrome connect` process group.

The existing cleanup behavior remains for non-detached handles because `ChromeProcess::drop()` still calls `kill()`, and `TempDir::drop()` still removes the directory when the owner is not forgotten.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/chrome/launcher.rs` | In `ChromeProcess::detach()`, take `child` and `temp_dir` and `std::mem::forget()` them instead of dropping them. | Prevents cleanup of resources required by detached Chrome. |
| `src/chrome/launcher.rs` | Add platform-specific process-session detachment before spawning Chrome. | Prevents external process-group cleanup from terminating detached Chrome. |
| `src/chrome/launcher.rs` | Add a unit regression test for detach preserving the temp profile. | Fails against the current bug and protects the lifetime contract. |

### Blast Radius

- **Direct impact**: `ChromeProcess::detach()` and Chrome process spawn configuration in `src/chrome/launcher.rs`.
- **Indirect impact**: all successful `connect --launch` invocations keep their temporary profile directory for the lifetime of the launched Chrome process.
- **Risk level**: Low. The change aligns behavior with the documented detach contract and does not alter CDP, session serialization, launch arguments, or CLI output.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Temporary profile directories remain after detached Chrome exits | Medium | This already follows the detach contract; future cleanup can be a separate feature with explicit ownership. |
| Startup failure stops cleaning temporary profile directories | Low | The fix only changes `detach()` after successful launch readiness. Failure paths still drop `ChromeProcess` normally. |
| `disconnect` no longer kills launched Chrome | Very Low | The fix does not change stored PID or disconnect behavior; manual smoke verified `connect --disconnect` can kill the stored PID. |
| Windows process flags alter visible launch behavior | Low | Flags only detach the browser process group; stdout/stderr remain unchanged from existing behavior. |

---

## Validation Checklist

- [x] Root cause is identified with specific code references.
- [x] Fix is minimal and scoped to launch lifetime.
- [x] Blast radius is assessed.
- [x] Regression risks are documented with mitigations.
- [x] Fix follows existing project patterns.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #265 | 2026-04-24 | Initial defect design |
