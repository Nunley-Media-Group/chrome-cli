# Requirements: Large Response Detection with Guided Search and Full-Response Override

**Issues**: #168
**Date**: 2026-03-12
**Status**: Draft
**Author**: AI (nmg-sdlc)

---

## User Story

**As an** AI agent consuming agentchrome output
**I want** a machine-readable guidance response when output exceeds a size threshold, plus a `--search` flag to filter results and a `--full-response` override when I need complete data
**So that** I can avoid unnecessary context consumption while still accessing full output when required

---

## Background

AI agents (Claude Code, MCP clients) have finite context windows. When agentchrome commands return large responses — accessibility trees (1–10 MB), full page text, JS execution results, or network listings — the agent must consume the entire JSON output before processing it. This wastes context budget and reduces effectiveness.

Currently, commands have piecemeal truncation: `page snapshot` hard-caps at 10,000 nodes with `"truncated": true` metadata, `network get` truncates response bodies at 10 KB inline, `js exec` has an optional `--max-size` flag, and `page screenshot` warns on stderr for large base64 output. There is no unified cross-command threshold detection, no structured guidance object, no `--search` filtering flag, and no `--full-response` override.

The solution is a three-part behavior: (1) when serialized output exceeds a configurable threshold (default 16 KB), return a structured guidance object instead of raw data; (2) add a per-command `--search <query>` flag so the agent can retrieve only matching content; (3) add a global `--full-response` flag so the agent can explicitly opt into the full output.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Large Response Detection Returns Guidance Object

**Given** a command that would produce serialized JSON output exceeding the 16 KB default threshold
**When** the command is executed without `--search` or `--full-response`
**Then** stdout contains a JSON guidance object (not the raw data) with fields `large_response: true`, `size_bytes`, `command`, `summary`, and `guidance`
**And** the exit code is 0

**Example**:
- Given: `page snapshot` on a complex page producing a 524 KB accessibility tree
- When: `agentchrome page snapshot`
- Then: stdout is `{"large_response":true,"size_bytes":536576,"command":"page snapshot","summary":{"total_nodes":8500,"top_roles":["main","navigation","complementary"]},"guidance":"Response is 524 KB (above 16 KB threshold). ..."}`

### AC2: Search Flag Filters and Returns Matching Content

**Given** a command that supports `--search` and output exceeding 16 KB
**When** the agent runs with `--search "login button"`
**Then** stdout contains only the matching nodes/content following the normal output schema for that command (not the guidance object)
**And** the exit code is 0

**Example**:
- Given: `page snapshot` on a page with an accessibility tree exceeding 16 KB
- When: `agentchrome page snapshot --search "login"`
- Then: stdout contains only accessibility tree nodes whose text, role, or name matches "login", serialized in the normal snapshot JSON schema

### AC3: Full-Response Override Returns Complete Data

**Given** a command that would trigger the large-response guidance
**When** the agent runs with `--full-response`
**Then** stdout contains the complete, untruncated response as if no threshold existed
**And** the exit code is 0

**Example**:
- Given: `page snapshot` producing 524 KB output
- When: `agentchrome page snapshot --full-response`
- Then: stdout contains the full 524 KB accessibility tree JSON

### AC4: Guidance Object Contains Actionable Instructions

**Given** the guidance object is returned for a large response
**When** an agent reads the `guidance` field
**Then** the guidance string includes: the response size in human-readable form, a structural summary, `--search` usage with a command-specific example, `--full-response` usage, and 2–3 concrete examples of when `--full-response` is appropriate

**Example**:
- Given: guidance returned for `page snapshot`
- When: reading `guidance` field
- Then: text contains `"Response is 524 KB (above 16 KB threshold). Summary: accessibility tree with 8,500 nodes (top roles: main, navigation, complementary). Options: (1) Use --search \"<query>\" to retrieve matching nodes only. Example: page snapshot --search \"login\". (2) Use --full-response to retrieve the complete tree. Use --full-response when: you need to inspect all interactive elements, --search doesn't narrow results sufficiently, or you are performing a comprehensive page audit."`

### AC5: Below-Threshold Responses Are Unaffected

**Given** a command that produces serialized JSON output under 16 KB
**When** the command is executed (without `--search` or `--full-response`)
**Then** stdout contains the full response as today — no guidance object, no behavioral change
**And** the output schema is identical to pre-feature behavior

**Example**:
- Given: `page text` on a simple page producing 2 KB of text
- When: `agentchrome page text`
- Then: stdout is `{"text":"...","url":"...","title":"..."}` as before

### AC6: Threshold Is Configurable via CLI Flag

**Given** a user who wants a different threshold
**When** they run with `--large-response-threshold 32768`
**Then** the guidance behavior activates at 32,768 bytes instead of the default 16,384

**Example**:
- Given: a command producing 20 KB output
- When: `agentchrome page snapshot --large-response-threshold 32768`
- Then: output is the full response (20 KB < 32 KB threshold)

### AC7: Threshold Is Configurable via Config File

**Given** a config file with `large_response_threshold = 8192`
**When** the user runs a command producing 10 KB output without any CLI threshold flag
**Then** the guidance behavior activates because 10 KB exceeds the configured 8 KB threshold
**And** the CLI flag `--large-response-threshold` overrides the config file value when both are present

### AC8: Guidance Object Schema Is Consistent Across Commands

**Given** any command that triggers the large-response behavior
**When** the guidance object is returned
**Then** it always includes these fields in this order: `large_response` (bool, always `true`), `size_bytes` (integer), `command` (string — the full subcommand path, e.g., `"page snapshot"`, `"network list"`), `summary` (object — command-specific structural metadata), and `guidance` (string)
**And** no other top-level fields are present

### AC9: Page Snapshot Search Filters by Text, Role, and Name

**Given** `page snapshot` on a page with a large accessibility tree
**When** the agent runs `page snapshot --search "login"`
**Then** the returned nodes include any node whose `name`, `role`, or visible `text` contains "login" (case-insensitive substring match)
**And** ancestor nodes are included to preserve tree context (but non-matching branches are pruned)

### AC10: Page Text Search Filters by Content

**Given** `page text` on a page with large text content
**When** the agent runs `page text --search "error"`
**Then** the output contains only text sections/paragraphs containing the query "error"
**And** the output schema remains `{"text":"...","url":"...","title":"..."}`

### AC11: JS Exec Search Filters JSON Keys and Values

**Given** `js exec` returning a large JSON result
**When** the agent runs `js exec "..." --search "email"`
**Then** the output contains only the JSON keys/values matching "email"
**And** the output schema follows the normal `JsExecResult` structure

### AC12: Network Commands Search Filters by URL Pattern or Method

**Given** `network list` with many captured requests
**When** the agent runs `network list --search "api/v2"`
**Then** only requests whose URL contains "api/v2" are returned
**And** the output schema follows the normal network list structure

**Given** `network get` with a large response body
**When** the agent runs `network get <id> --search "token"`
**Then** only response content containing "token" is returned

### AC13: Search Flag Bypasses Large-Response Gate

**Given** a command that would normally trigger the large-response guidance
**When** the agent runs with `--search <query>`
**Then** the matched content is returned directly (never the guidance object), even if the matched content also exceeds the threshold

### AC14: Full-Response Flag Is Compatible with Output Format Flags

**Given** a command that would trigger the large-response guidance
**When** the agent runs with `--full-response --pretty`
**Then** stdout contains the complete response pretty-printed
**And** `--full-response` works with `--json`, `--pretty`, and `--plain`

### AC15: Existing Per-Command Truncation Remains as Second Layer

**Given** `page snapshot` on a page with 15,000 nodes and `--full-response`
**When** the command is executed with `--full-response`
**Then** the existing `MAX_NODES` (10,000) truncation still applies (the snapshot has `truncated: true`)
**And** the large-response gate operates on the serialized output of the already-truncated result

### AC16: Summary Field Is Command-Specific

**Given** different commands triggering the large-response guidance
**When** the guidance object is returned
**Then** the `summary` field contains command-specific metadata:
- `page snapshot`: `{"total_nodes": N, "top_roles": ["role1", "role2", ...]}`
- `page text`: `{"character_count": N, "line_count": N}`
- `js exec`: `{"result_type": "object|array|string", "size_bytes": N}`
- `network list`: `{"request_count": N, "methods": ["GET", "POST", ...], "domains": ["example.com", ...]}`
- `network get`: `{"url": "...", "status": N, "content_type": "...", "body_size_bytes": N}`

### AC17: Plain-Text Mode Is Not Affected

**Given** a command executed with `--plain` that would exceed the threshold in JSON mode
**When** the command runs with `--plain`
**Then** the full plain-text output is returned as today — no guidance object
**And** `--search` and `--full-response` are still usable with `--plain`

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | All commands check serialized JSON output size before printing; if above threshold, emit guidance object instead | Must | Applies after per-command truncation (MAX_NODES, etc.) |
| FR2 | Default threshold is 16,384 bytes (16 KB); overridable via `--large-response-threshold <bytes>` global flag | Must | |
| FR3 | Threshold is configurable via config file key `large_response_threshold` | Must | CLI flag overrides config file |
| FR4 | Guidance object includes: `large_response` (bool), `size_bytes` (integer), `command` (string), `summary` (object), `guidance` (string) | Must | No other top-level fields |
| FR5 | `guidance` field includes `--search` example, `--full-response` example, and 2–3 concrete examples of when `--full-response` is appropriate | Must | Examples are command-specific |
| FR6 | `--full-response` global flag bypasses the threshold and returns complete raw output | Must | Compatible with `--json`, `--pretty`, `--plain` |
| FR7 | `page snapshot` supports `--search <query>` to filter accessibility tree nodes by text/role/name (case-insensitive substring) | Must | Ancestor nodes preserved for tree context |
| FR8 | `page text` supports `--search <query>` to return only text sections containing the query | Must | |
| FR9 | `js exec` supports `--search <query>` to filter JSON result keys/values matching the query | Should | |
| FR10 | `network list` supports `--search <query>` to filter by URL pattern or method | Should | |
| FR11 | `network get` supports `--search <query>` to filter response content | Should | |
| FR12 | `--search` flag bypasses the large-response gate (always returns matched content, never the guidance object) | Must | |
| FR13 | Summary field is command-specific: node count + top roles for snapshot; character count + line count for page text; result type + size for js exec; request count + methods + domains for network list; url + status + content type + body size for network get | Must | |
| FR14 | Existing per-command truncation constants (`MAX_NODES`, `MAX_INLINE_BODY_SIZE`) remain as a second protection layer | Should | Large-response gate operates on already-truncated output |
| FR15 | `--plain` mode is exempt from the guidance object behavior; full plain-text output is returned | Must | `--search` and `--full-response` still apply in plain mode |
| FR16 | No new global flag name (`--search`, `--full-response`, `--large-response-threshold`) collides with existing global flags or per-command flags | Must | `--search` is per-command; `--full-response` and `--large-response-threshold` are global |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Threshold check adds < 1ms overhead to any command; serialization happens once (not duplicated for size check + output) |
| **Compatibility** | Existing scripts relying on current output schemas continue to work for below-threshold responses |
| **Platforms** | macOS, Linux, Windows — all platforms supported identically |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **CLI help text** | `--full-response`, `--large-response-threshold`, and `--search` appear in help output with clear descriptions |
| **Error states** | `--search` on a command that doesn't support it produces a clap validation error (JSON on stderr) |
| **Guidance readability** | Guidance string uses human-readable sizes (e.g., "524 KB" not "536576 bytes") and complete example commands |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--large-response-threshold` | usize (bytes) | Must be > 0 | No (default 16384) |
| `--full-response` | bool flag | N/A | No (default false) |
| `--search` | String | Non-empty | No (per-command) |

### Output Data (Guidance Object)

| Field | Type | Description |
|-------|------|-------------|
| `large_response` | bool | Always `true` |
| `size_bytes` | u64 | Serialized JSON size in bytes |
| `command` | String | Full subcommand path (e.g., `"page snapshot"`) |
| `summary` | Object | Command-specific structural metadata (see FR13) |
| `guidance` | String | Human-readable instructions with `--search` and `--full-response` examples |

---

## Dependencies

### Internal Dependencies
- [x] `OutputFormat` struct in `src/cli/mod.rs` — extended with `--full-response` and `--large-response-threshold`
- [x] `snapshot.rs` — already has `MAX_NODES` truncation and node count metadata
- [x] `network.rs` — already has `MAX_INLINE_BODY_SIZE` truncation
- [x] `js.rs` — already has `--max-size` and `apply_truncation()`
- [x] `config.rs` — extended with `large_response_threshold` key

### External Dependencies
- None

### Blocked By
- None

---

## Out of Scope

- Streaming/pagination of results (no cursor-based "next page" tokens)
- Server-side response caching for subsequent search queries
- `page screenshot` large-response handling (handled by existing `--file` flag)
- `perf record` trace files (written to disk, not stdout)
- Regex support in `--search` (substring match only for v1)
- `console read` / `console follow` large-response detection (streaming commands)
- `connect`, `tabs`, `emulate`, `dialog` commands (typically small output)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Context savings | 90%+ reduction in token consumption for large page snapshots when using `--search` | Compare output sizes: full snapshot vs. searched subset |
| Agent adoption | Agents correctly interpret guidance object and use `--search` or `--full-response` on next invocation | Manual testing with Claude Code |
| Zero regressions | Below-threshold responses produce identical output to pre-feature behavior | BDD tests comparing output schemas |

---

## Open Questions

- [x] Should `--search` support regex? — **No, substring match only for v1 (per Out of Scope)**
- [x] Should `--plain` mode support the guidance object? — **No, plain mode is exempt (per AC17)**

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #168 | 2026-03-12 | Initial feature spec |

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
