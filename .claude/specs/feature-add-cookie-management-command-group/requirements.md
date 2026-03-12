# Requirements: Cookie Management Command Group

**Issues**: #164
**Date**: 2026-03-11
**Status**: Draft
**Author**: Claude (SDLC)

---

## User Story

**As an** AI agent or automation engineer testing authenticated workflows
**I want** commands to read, set, and clear browser cookies
**So that** I can manage authentication state, test session handling, and set up preconditions without navigating through login flows

---

## Background

Cookie management is a fundamental browser automation capability. Common use cases include skipping login flows by setting auth cookies directly, testing session expiry by clearing session cookies, inspecting auth state by reading cookies, preventing session bleed between test runs, and extracting cookies for API call forwarding.

Currently, the only workaround is `js exec "document.cookie"`, which cannot access HttpOnly cookies (which most auth cookies are). The CDP `Network.getCookies` / `Network.setCookie` / `Network.deleteCookies` methods provide full access but are not exposed through agentchrome. The `Network` domain is already enabled for `network list` and `network intercept` commands, so the infrastructure for CDP network domain communication is in place.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: List all cookies for the current page

**Given** a connected Chrome session on a page that has set cookies
**When** I run `agentchrome cookie list`
**Then** the command returns a JSON array of all cookies (including HttpOnly) with name, value, domain, path, expires, httpOnly, secure, and sameSite fields
**And** the exit code is 0

**Example**:
- Given: Chrome connected and navigated to a page with cookies set (e.g., `document.cookie = "test=value"`)
- When: `agentchrome cookie list`
- Then: JSON array on stdout containing at least `{"name":"test","value":"value",...}`

### AC2: Set a cookie

**Given** a connected Chrome session
**When** I run `agentchrome cookie set "session_id" "abc123" --domain "example.com"`
**Then** the cookie is set in the browser
**And** a subsequent `cookie list` invocation includes the cookie with the specified name, value, and domain
**And** the exit code is 0

**Example**:
- Given: Chrome connected
- When: `agentchrome cookie set "session_id" "abc123" --domain "example.com"`
- Then: stdout JSON confirms success; a subsequent `agentchrome cookie list` shows the cookie

### AC3: Delete a specific cookie

**Given** a connected Chrome session with a cookie named "session_id"
**When** I run `agentchrome cookie delete "session_id"`
**Then** the cookie is removed from the browser
**And** a subsequent `cookie list` invocation no longer includes it
**And** the exit code is 0

**Example**:
- Given: A cookie "session_id" exists (set via `cookie set`)
- When: `agentchrome cookie delete "session_id"`
- Then: stdout JSON confirms deletion; `cookie list` no longer shows "session_id"

### AC4: Clear all cookies

**Given** a connected Chrome session with multiple cookies
**When** I run `agentchrome cookie clear`
**Then** all cookies are removed
**And** a subsequent `cookie list` invocation returns an empty array
**And** the exit code is 0

**Example**:
- Given: Multiple cookies exist
- When: `agentchrome cookie clear`
- Then: stdout JSON confirms clearing; `cookie list` returns `[]`

### AC5: List cookies filtered by domain

**Given** a connected Chrome session with cookies from multiple domains
**When** I run `agentchrome cookie list --domain "example.com"`
**Then** only cookies matching the specified domain are returned
**And** cookies from other domains are excluded

**Example**:
- Given: Cookies exist for "example.com" and "other.com"
- When: `agentchrome cookie list --domain "example.com"`
- Then: JSON array contains only cookies with domain "example.com"

### AC6: Set a cookie with optional flags

**Given** a connected Chrome session
**When** I run `agentchrome cookie set "secure_token" "xyz" --domain "example.com" --secure --http-only --same-site "Strict" --path "/api" --expires 1735689600`
**Then** the cookie is set with the specified secure, httpOnly, sameSite, path, and expires attributes
**And** a subsequent `cookie list` confirms all attributes are correctly applied

### AC7: Delete a cookie scoped by domain

**Given** a connected Chrome session with cookies named "token" on both "a.example.com" and "b.example.com"
**When** I run `agentchrome cookie delete "token" --domain "a.example.com"`
**Then** only the cookie on "a.example.com" is deleted
**And** the cookie on "b.example.com" remains

### AC8: Cookie list returns empty array when no cookies exist

**Given** a connected Chrome session on a page with no cookies
**When** I run `agentchrome cookie list`
**Then** the command returns an empty JSON array `[]`
**And** the exit code is 0

### AC9: JSON output on stdout, errors on stderr

**Given** any cookie subcommand
**When** the command executes
**Then** structured JSON is written to stdout on success
**And** structured JSON errors are written to stderr on failure
**And** exit codes follow the project convention (0=success, 2=connection error)

### AC10: Cross-invocation state persistence

**Given** a connected Chrome session
**When** I set a cookie via `agentchrome cookie set "persist" "yes" --domain "example.com"` in one invocation
**And** I run `agentchrome cookie list` in a separate subsequent invocation
**Then** the cookie "persist" appears in the list output
**And** this confirms cookies survive across CLI process boundaries

### Generated Gherkin Preview

```gherkin
Feature: Cookie Management
  As an AI agent or automation engineer
  I want commands to read, set, and clear browser cookies
  So that I can manage authentication state and test session handling

  Scenario: List all cookies for the current page
    Given a connected Chrome session on a page with cookies
    When I run "agentchrome cookie list"
    Then the output is a JSON array of cookie objects with standard fields
    And the exit code is 0

  Scenario: Set a cookie
    Given a connected Chrome session
    When I run "agentchrome cookie set 'session_id' 'abc123' --domain 'example.com'"
    Then the output confirms the cookie was set
    And a subsequent "cookie list" includes the cookie

  Scenario: Delete a specific cookie
    Given a connected Chrome session with a cookie named "session_id"
    When I run "agentchrome cookie delete 'session_id'"
    Then the output confirms deletion
    And a subsequent "cookie list" does not include the cookie

  Scenario: Clear all cookies
    Given a connected Chrome session with multiple cookies
    When I run "agentchrome cookie clear"
    Then the output confirms clearing
    And a subsequent "cookie list" returns an empty array

  Scenario: List cookies filtered by domain
    Given a connected Chrome session with cookies from multiple domains
    When I run "agentchrome cookie list --domain 'example.com'"
    Then only cookies matching "example.com" are returned

  Scenario: Set a cookie with optional flags
    Given a connected Chrome session
    When I run "agentchrome cookie set" with --secure --http-only --same-site --path --expires flags
    Then the cookie is set with all specified attributes

  Scenario: Delete a cookie scoped by domain
    Given cookies named "token" on "a.example.com" and "b.example.com"
    When I run "agentchrome cookie delete 'token' --domain 'a.example.com'"
    Then only the "a.example.com" cookie is deleted

  Scenario: Empty cookie list
    Given a connected Chrome session on a page with no cookies
    When I run "agentchrome cookie list"
    Then the output is an empty JSON array

  Scenario: JSON output format compliance
    Given any cookie subcommand
    When the command executes
    Then success output is JSON on stdout
    And error output is JSON on stderr

  Scenario: Cross-invocation state persistence
    Given a cookie set in one CLI invocation
    When I run "cookie list" in a separate invocation
    Then the previously set cookie is visible
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Add `cookie list` subcommand using `Network.getCookies` | Must | Returns all cookies for the current page URL |
| FR2 | Add `cookie set <name> <value>` subcommand using `Network.setCookie` | Must | Positional args for name and value |
| FR3 | Add `cookie delete <name>` subcommand using `Network.deleteCookies` | Must | Deletes by name, optionally scoped by domain |
| FR4 | Add `cookie clear` subcommand to delete all cookies | Must | Lists all cookies, then deletes each one |
| FR5 | `cookie set` supports `--domain`, `--path`, `--secure`, `--http-only`, `--same-site`, `--expires` flags | Should | All optional, domain is strongly recommended |
| FR6 | `cookie list` supports `--domain` filter flag | Should | Client-side filter on CDP response |
| FR7 | All subcommands produce structured JSON output on stdout | Must | Consistent with project output contract |
| FR8 | `cookie list` supports `--all` flag to get all cookies (not scoped to current URL) via `Network.getAllCookies` | Could | Useful for debugging cross-domain cookie state |
| FR9 | `cookie delete` supports `--domain` flag to scope deletion | Should | Required for deleting domain-specific cookies |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | All cookie commands complete within the global timeout (default timeout applies) |
| **Security** | Cookies with sensitive values are output as-is (no redaction); the user controls what they do with the output |
| **Reliability** | Graceful error on no connection; empty array on no cookies (not an error) |
| **Platforms** | macOS, Linux, Windows — platform-independent (all CDP-based) |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **CLI syntax** | `agentchrome cookie <list\|set\|delete\|clear> [args] [flags]` |
| **Output format** | JSON on stdout; supports `--pretty` global flag |
| **Error states** | Connection errors → JSON on stderr with exit code 2 |
| **Empty states** | No cookies → empty JSON array `[]` (not an error) |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `name` (cookie set) | String | Non-empty | Yes |
| `value` (cookie set) | String | Any string (including empty) | Yes |
| `name` (cookie delete) | String | Non-empty | Yes |
| `--domain` | String | Valid domain format | No |
| `--path` | String | Valid path starting with `/` | No |
| `--secure` | Boolean flag | N/A | No |
| `--http-only` | Boolean flag | N/A | No |
| `--same-site` | String enum | `Strict`, `Lax`, `None` | No |
| `--expires` | f64 (Unix timestamp) | Positive number | No |
| `--all` (cookie list) | Boolean flag | N/A | No |

### Output Data (cookie list)

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Cookie name |
| `value` | String | Cookie value |
| `domain` | String | Cookie domain |
| `path` | String | Cookie path |
| `expires` | f64 | Expiry as Unix timestamp (-1 for session cookies) |
| `httpOnly` | bool | Whether cookie is HttpOnly |
| `secure` | bool | Whether cookie requires HTTPS |
| `sameSite` | String | SameSite attribute (`Strict`, `Lax`, `None`) |
| `size` | u64 | Cookie size in bytes |

### Output Data (cookie set)

| Field | Type | Description |
|-------|------|-------------|
| `success` | bool | Whether the cookie was set successfully |
| `name` | String | Name of the cookie that was set |
| `domain` | String | Domain the cookie was set on |

### Output Data (cookie delete / clear)

| Field | Type | Description |
|-------|------|-------------|
| `deleted` | u64 | Number of cookies deleted |

---

## Dependencies

### Internal Dependencies
- [x] CDP Client (`src/cdp/client.rs`) — already supports sending arbitrary CDP commands
- [x] Network domain enabled — already used by `network.rs`
- [x] Session/connection infrastructure — shared with all commands

### External Dependencies
- [x] Chrome DevTools Protocol — `Network.getCookies`, `Network.setCookie`, `Network.deleteCookies`

### Blocked By
- None

---

## Out of Scope

- Cookie import/export in Netscape or JSON format (could be a follow-up)
- Automatic cookie persistence across agentchrome sessions
- Cookie monitoring/watching (like `console follow` but for cookie changes)
- Cookie jar file I/O

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| All ACs pass | 100% | BDD test scenarios pass |
| HttpOnly cookie access | Works | `cookie list` returns HttpOnly cookies that `document.cookie` cannot |
| Cross-invocation persistence | Works | Cookie set in invocation A visible in invocation B |

---

## Open Questions

- None (CDP cookie methods are well-documented and stable)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #164 | 2026-03-11 | Initial feature spec |

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
