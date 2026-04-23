# Requirements: Large Response Detection with Guided Search and Full-Response Override

**Issues**: #168, #177, #220
**Date**: 2026-03-13
**Status**: Draft
**Author**: AI (nmg-sdlc)

---

## User Story

**As an** AI agent consuming agentchrome output
**I want** large command outputs automatically written to the OS temp directory with the file path returned on stdout
**So that** I can access complete data without consuming excessive context budget or requiring re-invocation with special flags

---

## Background

AI agents (Claude Code, MCP clients) have finite context windows. When agentchrome commands return large responses — accessibility trees (1–10 MB), full page text, JS execution results, or network listings — the agent must consume the entire JSON output before processing it. This wastes context budget and reduces effectiveness.

Currently, commands have piecemeal truncation: `page snapshot` hard-caps at 10,000 nodes with `"truncated": true` metadata, `network get` truncates response bodies at 10 KB inline, `js exec` has an optional `--max-size` flag, and `page screenshot` warns on stderr for large base64 output. There is no unified cross-command threshold detection, no structured guidance object, no `--search` filtering flag, and no `--full-response` override.

The original solution (#168) introduced a three-part behavior: (1) a structured guidance object when output exceeded the threshold, (2) per-command `--search` flags, and (3) a global `--full-response` flag. This required a two-step agent pattern (detect large response → re-invoke with flags), adding latency and complexity.

Issue #177 simplifies this: when output exceeds the threshold, automatically write the full JSON to a UUID-named temp file and return the file path on stdout. The agent reads the file directly — no re-invocation, no special flags. The `--search` and `--full-response` flags are removed.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: ~~Large Response Detection Returns Guidance Object~~ [SUPERSEDED by AC18]

~~**Given** a command that would produce serialized JSON output exceeding the 16 KB default threshold~~
~~**When** the command is executed without `--search` or `--full-response`~~
~~**Then** stdout contains a JSON guidance object (not the raw data) with fields `large_response: true`, `size_bytes`, `command`, `summary`, and `guidance`~~
~~**And** the exit code is 0~~

> **Superseded by #177**: The guidance object is replaced by temp file output (see AC18).

### AC2: ~~Search Flag Filters and Returns Matching Content~~ [SUPERSEDED — removed by AC24]

~~**Given** a command that supports `--search` and output exceeding 16 KB~~
~~**When** the agent runs with `--search "login button"`~~
~~**Then** stdout contains only the matching nodes/content following the normal output schema for that command (not the guidance object)~~
~~**And** the exit code is 0~~

> **Superseded by #177**: The `--search` flag is removed entirely (see AC24).

### AC3: ~~Full-Response Override Returns Complete Data~~ [SUPERSEDED — removed by AC24]

~~**Given** a command that would trigger the large-response guidance~~
~~**When** the agent runs with `--full-response`~~
~~**Then** stdout contains the complete, untruncated response as if no threshold existed~~
~~**And** the exit code is 0~~

> **Superseded by #177**: The `--full-response` flag is removed entirely (see AC24).

### AC4: ~~Guidance Object Contains Actionable Instructions~~ [SUPERSEDED by AC18]

~~**Given** the guidance object is returned for a large response~~
~~**When** an agent reads the `guidance` field~~
~~**Then** the guidance string includes: the response size in human-readable form, a structural summary, `--search` usage with a command-specific example, `--full-response` usage, and 2–3 concrete examples of when `--full-response` is appropriate~~

> **Superseded by #177**: The guidance object is replaced by a temp file output object (see AC18, AC21).

### AC5: Below-Threshold Responses Are Unaffected

**Given** a command that produces serialized JSON output under 16 KB
**When** the command is executed
**Then** stdout contains the full response as today — no temp file is written, no behavioral change
**And** the output schema is identical to pre-feature behavior

**Example**:
- Given: `page text` on a simple page producing 2 KB of text
- When: `agentchrome page text`
- Then: stdout is `{"text":"...","url":"...","title":"..."}` as before

### AC6: Threshold Is Configurable via CLI Flag

**Given** a user who wants a different threshold
**When** they run with `--large-response-threshold 32768`
**Then** the temp file behavior activates at 32,768 bytes instead of the default 16,384

**Example**:
- Given: a command producing 20 KB output
- When: `agentchrome page snapshot --large-response-threshold 32768`
- Then: output is the full response (20 KB < 32 KB threshold)

### AC7: Threshold Is Configurable via Config File

**Given** a config file with `large_response_threshold = 8192`
**When** the user runs a command producing 10 KB output without any CLI threshold flag
**Then** a temp file is written (10 KB > 8 KB)
**And** the CLI flag `--large-response-threshold` overrides the config file value when both are present

### AC8: ~~Guidance Object Schema Is Consistent Across Commands~~ [SUPERSEDED by AC21]

~~**Given** any command that triggers the large-response behavior~~
~~**When** the guidance object is returned~~
~~**Then** it always includes these fields in this order: `large_response` (bool, always `true`), `size_bytes` (integer), `command` (string — the full subcommand path, e.g., `"page snapshot"`, `"network list"`), `summary` (object — command-specific structural metadata), and `guidance` (string)~~
~~**And** no other top-level fields are present~~

> **Superseded by #177**: The output object schema has changed (see AC21).

### AC9: ~~Page Snapshot Search Filters by Text, Role, and Name~~ [SUPERSEDED — removed by AC24]

~~**Given** `page snapshot` on a page with a large accessibility tree~~
~~**When** the agent runs `page snapshot --search "login"`~~
~~**Then** the returned nodes include any node whose `name`, `role`, or visible `text` contains "login" (case-insensitive substring match)~~
~~**And** ancestor nodes are included to preserve tree context (but non-matching branches are pruned)~~

> **Superseded by #177**: The `--search` flag is removed entirely (see AC24).

### AC10: ~~Page Text Search Filters by Content~~ [SUPERSEDED — removed by AC24]

~~**Given** `page text` on a page with large text content~~
~~**When** the agent runs `page text --search "error"`~~
~~**Then** the output contains only text sections/paragraphs containing the query "error"~~
~~**And** the output schema remains `{"text":"...","url":"...","title":"..."}`~~

> **Superseded by #177**: The `--search` flag is removed entirely (see AC24).

### AC11: ~~JS Exec Search Filters JSON Keys and Values~~ [SUPERSEDED — removed by AC24]

~~**Given** `js exec` returning a large JSON result~~
~~**When** the agent runs `js exec "..." --search "email"`~~
~~**Then** the output contains only the JSON keys/values matching "email"~~
~~**And** the output schema follows the normal `JsExecResult` structure~~

> **Superseded by #177**: The `--search` flag is removed entirely (see AC24).

### AC12: ~~Network Commands Search Filters by URL Pattern or Method~~ [SUPERSEDED — removed by AC24]

~~**Given** `network list` with many captured requests~~
~~**When** the agent runs `network list --search "api/v2"`~~
~~**Then** only requests whose URL contains "api/v2" are returned~~
~~**And** the output schema follows the normal network list structure~~

> **Superseded by #177**: The `--search` flag is removed entirely (see AC24).

### AC13: ~~Search Flag Bypasses Large-Response Gate~~ [SUPERSEDED — removed by AC24]

~~**Given** a command that would normally trigger the large-response guidance~~
~~**When** the agent runs with `--search <query>`~~
~~**Then** the matched content is returned directly (never the guidance object), even if the matched content also exceeds the threshold~~

> **Superseded by #177**: The `--search` flag is removed entirely (see AC24).

### AC14: ~~Full-Response Flag Is Compatible with Output Format Flags~~ [SUPERSEDED — removed by AC24]

~~**Given** a command that would trigger the large-response guidance~~
~~**When** the agent runs with `--full-response --pretty`~~
~~**Then** stdout contains the complete response pretty-printed~~
~~**And** `--full-response` works with `--json`, `--pretty`, and `--plain`~~

> **Superseded by #177**: The `--full-response` flag is removed entirely (see AC24).

### AC15: Existing Per-Command Truncation Remains as Second Layer

**Given** `page snapshot` on a page with 15,000 nodes
**When** the command is executed
**Then** the existing `MAX_NODES` (10,000) truncation still applies (the snapshot has `truncated: true`)
**And** the large-response gate operates on the serialized output of the already-truncated result

### AC16: ~~Summary Field Is Command-Specific~~ [SUPERSEDED by AC23]

~~**Given** different commands triggering the large-response guidance~~
~~**When** the guidance object is returned~~
~~**Then** the `summary` field contains command-specific metadata:~~
~~- `page snapshot`: `{"total_nodes": N, "top_roles": ["role1", "role2", ...]}`~~
~~- `page text`: `{"character_count": N, "line_count": N}`~~
~~- `js exec`: `{"result_type": "object|array|string", "size_bytes": N}`~~
~~- `network list`: `{"request_count": N, "methods": ["GET", "POST", ...], "domains": ["example.com", ...]}`~~
~~- `network get`: `{"url": "...", "status": N, "content_type": "...", "body_size_bytes": N}`~~

> **Superseded by #177**: Summary field is retained but in a new output object (see AC23).

### AC17: ~~Plain-Text Mode Is Not Affected~~ [SUPERSEDED by AC22]

~~**Given** a command executed with `--plain` that would exceed the threshold in JSON mode~~
~~**When** the command runs with `--plain`~~
~~**Then** the full plain-text output is returned as today — no guidance object~~
~~**And** `--search` and `--full-response` are still usable with `--plain`~~

> **Superseded by #177**: Plain mode now also writes to temp file when above threshold (see AC22).

---

### AC18: Large Output Written to Temp File

**Given** a command producing JSON exceeding the threshold (default 16 KB)
**When** the command is executed
**Then** the full JSON is written to a UUID-named file in the OS temp directory (`{os_temp_dir}/agentchrome-{uuid}.json`)
**And** stdout contains `{"output_file": "<path>", "size_bytes": N, "command": "<cmd>", "summary": {...}}`
**And** exit code is 0

**Example**:
- Given: `page snapshot` on a complex page producing a 524 KB accessibility tree
- When: `agentchrome page snapshot`
- Then: stdout is `{"output_file":"/tmp/agentchrome-a1b2c3d4.json","size_bytes":536576,"command":"page snapshot","summary":{"total_nodes":8500,"top_roles":["main","navigation","complementary"]}}`
- And: `/tmp/agentchrome-a1b2c3d4.json` contains the full 524 KB accessibility tree JSON

### AC19: Temp File Contains Complete Unmodified Output

**Given** a command that triggers temp file output
**When** the agent reads the file at the returned `output_file` path
**Then** the file contains the full, unmodified JSON matching the normal command output schema
**And** the file is readable by the current user immediately after the command exits

### AC20: Temp Files Use UUID-Based Names to Prevent Collisions

**Given** two concurrent agentchrome commands both producing large output
**When** both execute simultaneously
**Then** each writes to a distinct UUID-based file path
**And** both paths are valid and readable
**And** neither file is corrupted by the other

### AC21: Output Object Schema Is Consistent Across Commands

**Given** any command triggering temp file output
**When** the output object is returned on stdout
**Then** it always has exactly these top-level fields: `output_file` (string — absolute file path), `size_bytes` (integer — byte count of the written file), `command` (string — full subcommand path, e.g., `"page snapshot"`), `summary` (object — command-specific metadata)
**And** no other top-level fields are present

> **Note (#220)**: Compound interaction+snapshot results use an extension of this shape — see the harden-progressive-disclosure spec. When an `--include-snapshot` result exceeds the threshold, only the `snapshot` field is offloaded to a temp file and replaced with a `TempFileOutput` object inline; the interaction confirmation fields remain at the top level.

### AC22: Plain Mode Also Writes to Temp File

**Given** a command run with `--plain` producing output exceeding the threshold
**When** the command executes
**Then** the full plain-text output is written to a temp file (`{os_temp_dir}/agentchrome-{uuid}.txt`)
**And** stdout contains the file path as plain text (not JSON)

### AC23: Summary Field Is Command-Specific

**Given** different commands triggering temp file output
**When** the output object is returned
**Then** the `summary` field contains command-specific metadata:
- `page snapshot`: `{"total_nodes": N, "top_roles": ["role1", ...]}`
- `page text`: `{"character_count": N, "line_count": N}`
- `js exec`: `{"result_type": "object|array|string", "size_bytes": N}`
- `network list`: `{"request_count": N, "methods": [...], "domains": [...]}`
- `network get`: `{"url": "...", "status": N, "content_type": "...", "body_size_bytes": N}`

### AC24: --search and --full-response Flags Are Removed

**Given** the previous mechanism with `--search` and `--full-response`
**When** a user passes either flag
**Then** clap returns a validation error (unknown argument)
**And** neither flag appears in help output

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | ~~All commands check serialized JSON output size before printing; if above threshold, emit guidance object instead~~ [SUPERSEDED by FR17] | ~~Must~~ | |
| FR2 | Default threshold is 16,384 bytes (16 KB); overridable via `--large-response-threshold <bytes>` global flag | Must | |
| FR3 | Threshold is configurable via config file key `large_response_threshold` | Must | CLI flag overrides config file |
| FR4 | ~~Guidance object includes: `large_response` (bool), `size_bytes` (integer), `command` (string), `summary` (object), `guidance` (string)~~ [SUPERSEDED by FR18] | ~~Must~~ | |
| FR5 | ~~`guidance` field includes `--search` example, `--full-response` example, and 2–3 concrete examples of when `--full-response` is appropriate~~ [SUPERSEDED — removed by FR23] | ~~Must~~ | |
| FR6 | ~~`--full-response` global flag bypasses the threshold and returns complete raw output~~ [SUPERSEDED — removed by FR24] | ~~Must~~ | |
| FR7 | ~~`page snapshot` supports `--search <query>` to filter accessibility tree nodes by text/role/name (case-insensitive substring)~~ [SUPERSEDED — removed by FR23] | ~~Must~~ | |
| FR8 | ~~`page text` supports `--search <query>` to return only text sections containing the query~~ [SUPERSEDED — removed by FR23] | ~~Must~~ | |
| FR9 | ~~`js exec` supports `--search <query>` to filter JSON result keys/values matching the query~~ [SUPERSEDED — removed by FR23] | ~~Should~~ | |
| FR10 | ~~`network list` supports `--search <query>` to filter by URL pattern or method~~ [SUPERSEDED — removed by FR23] | ~~Should~~ | |
| FR11 | ~~`network get` supports `--search <query>` to filter response content~~ [SUPERSEDED — removed by FR23] | ~~Should~~ | |
| FR12 | ~~`--search` flag bypasses the large-response gate (always returns matched content, never the guidance object)~~ [SUPERSEDED — removed by FR23] | ~~Must~~ | |
| FR13 | ~~Summary field is command-specific: node count + top roles for snapshot; character count + line count for page text; result type + size for js exec; request count + methods + domains for network list; url + status + content type + body size for network get~~ [SUPERSEDED by FR21] | ~~Must~~ | |
| FR14 | Existing per-command truncation constants (`MAX_NODES`, `MAX_INLINE_BODY_SIZE`) remain as a second protection layer | Should | Large-response gate operates on already-truncated output |
| FR15 | ~~`--plain` mode is exempt from the guidance object behavior; full plain-text output is returned~~ [SUPERSEDED by FR22] | ~~Must~~ | |
| FR16 | ~~No new global flag name (`--search`, `--full-response`, `--large-response-threshold`) collides with existing global flags or per-command flags~~ [SUPERSEDED — simplified] | ~~Must~~ | `--search` and `--full-response` are removed |
| FR17 | Write full JSON to `{os_temp_dir}/agentchrome-{uuid}.json` when serialized output exceeds threshold; print temp file output object to stdout | Must | Replaces guidance object behavior |
| FR18 | Stdout object fields: `output_file` (string), `size_bytes` (integer), `command` (string), `summary` (object) — no other top-level fields | Must | |
| FR19 | UUID-based file names prevent collisions in parallel workflows | Must | |
| FR20 | Below-threshold responses unchanged — full JSON inline, no file written | Must | Same as FR1 below-threshold path |
| FR21 | Summary field is command-specific (same metadata as original FR13: node count + top roles for snapshot, character count + line count for page text, result type + size for js exec, request count + methods + domains for network list, url + status + content type + body size for network get) | Must | |
| FR22 | `--plain` mode writes to temp file when above threshold; stdout returns path as plain text (file extension `.txt`) | Must | Replaces plain-mode exemption |
| FR23 | Remove per-command `--search` flags from all commands (`page snapshot`, `page text`, `js exec`, `network list`, `network get`) | Must | |
| FR24 | Remove global `--full-response` flag | Must | |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Threshold check adds < 1ms overhead to any command; serialization happens once (not duplicated for size check + output); temp file write adds < 10ms for typical payloads |
| **Compatibility** | Existing scripts relying on current output schemas continue to work for below-threshold responses; scripts using `--search` or `--full-response` will receive clap validation errors |
| **Platforms** | macOS, Linux, Windows — all platforms use `std::env::temp_dir()` for cross-platform temp directory resolution |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **CLI help text** | `--large-response-threshold` appears in help output with clear description; `--search` and `--full-response` do NOT appear |
| **Error states** | Passing `--search` or `--full-response` produces a clap validation error (unknown argument, JSON on stderr) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--large-response-threshold` | usize (bytes) | Must be > 0 | No (default 16384) |

### Output Data (Temp File Output Object)

| Field | Type | Description |
|-------|------|-------------|
| `output_file` | String | Absolute path to the temp file containing full output |
| `size_bytes` | u64 | Size of the written file in bytes |
| `command` | String | Full subcommand path (e.g., `"page snapshot"`) |
| `summary` | Object | Command-specific structural metadata (see FR21) |

---

## Dependencies

### Internal Dependencies
- [x] `OutputFormat` struct in `src/cli/mod.rs` — `full_response` field to be removed; `--search` per-command args to be removed
- [x] `src/output.rs` — `LargeResponseGuidance` struct replaced by `TempFileOutput`; `emit()` refactored to write to temp file; `emit_searched()` removed
- [x] `snapshot.rs` — already has `MAX_NODES` truncation and node count metadata; `--search` arg removed
- [x] `network.rs` — already has `MAX_INLINE_BODY_SIZE` truncation; `--search` arg removed
- [x] `js.rs` — already has `--max-size` and `apply_truncation()`; `--search` arg removed
- [x] `page.rs` — `--search` arg removed from `page text`
- [x] `config.rs` — `large_response_threshold` key remains supported

### External Dependencies
- `uuid` crate with `v4` feature (or equivalent random hex generation)

### Blocked By
- None

---

## Out of Scope

- Automatic temp file cleanup (OS/agent handles lifecycle)
- File compression of temp output
- Changing the default threshold value
- Streaming/chunked output
- Non-JSON formats (screenshots, traces already use `--file`)
- Regex support in search (search feature removed entirely)
- Server-side response caching
- `page screenshot` large-response handling (handled by existing `--file` flag)
- `perf record` trace files (written to disk, not stdout)
- `console read` / `console follow` large-response detection (streaming commands)
- `connect`, `tabs`, `emulate`, `dialog` commands (typically small output)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Context savings | 90%+ reduction in token consumption for large responses — agent reads only the small output object, fetches full data from file only when needed | Compare stdout sizes: full inline output vs. temp file output object |
| Agent simplicity | Single-step access to full data (read file) instead of two-step (detect guidance → re-invoke with flag) | Agent workflow step count |
| Zero regressions | Below-threshold responses produce identical output to pre-feature behavior | BDD tests comparing output schemas |

---

## Open Questions

- [x] Should `--search` support regex? — **Moot: `--search` removed entirely (#177)**
- [x] Should `--plain` mode support the guidance object? — **Moot: plain mode now uses temp file (#177)**
- [x] Should temp files use UUID or random hex? — **UUID v4 preferred for standard collision resistance**

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #168 | 2026-03-12 | Initial feature spec |
| #177 | 2026-03-13 | Replace guidance object with temp file output; remove `--search` and `--full-response` flags; plain mode now writes to temp file; ACs 1–4, 8–14, 16–17 superseded; FRs 1, 4–13, 15–16 superseded; new ACs 18–24 and FRs 17–24 added |
| #220 | 2026-04-22 | Extended temp-file gating + compound schema (see feature-harden-progressive-disclosure-enrich-skill-md-extend-temp-file-gating-notify-on-stale-skill/) |

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
