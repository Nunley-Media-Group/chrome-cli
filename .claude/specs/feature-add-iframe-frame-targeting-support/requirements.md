# Requirements: Iframe/Frame Targeting Support

**Issues**: #189
**Date**: 2026-04-15
**Status**: Draft
**Author**: Rich Nunley

---

## User Story

**As a** browser automation engineer working with enterprise web applications
**I want** to target specific iframe contexts when running AgentChrome commands
**So that** I can automate applications like Salesforce, ServiceNow, Dynamics, and SCORM/LMS platforms that embed content in iframes

---

## Background

Enterprise web applications heavily use iframes to embed content — Salesforce Lightning embeds classic admin pages in iframes, SCORM courses run inside nested iframes in LMS platforms, and ServiceNow/Dynamics 365 use iframes throughout their UIs. AgentChrome currently has no way to access iframe content: `page snapshot` shows the iframe element but cannot see inside it, `js exec` runs only in the main frame, `interact click-at` dispatches events at the page level where overlays inside iframes intercept them, and `dom select` cannot query across frame boundaries. This is the single highest-leverage enhancement for enterprise automation use cases and has been identified as the top priority by user feedback.

Frame ID tracking already exists in `src/navigate.rs` for `Page.frameNavigated` events, but no CDP frame attachment (`Target.attachToTarget` for OOPIFs, `Page.createIsolatedWorld`, `Runtime.evaluate` with `executionContextId`) is used. The CDP client communicates only with the main frame session.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: List frames on a page

**Given** a page with one or more iframes loaded in the browser
**When** `page frames` is run
**Then** JSON output on stdout lists all frames with fields: `index` (integer, 0-based), `id` (CDP frame ID string), `url` (frame URL), `name` (frame name attribute or empty string), `securityOrigin` (origin string), `unreachable` (boolean, true for cross-origin frames that failed to load), `width` (integer pixels), `height` (integer pixels), and `depth` (integer nesting level, 0 for main frame)

**Example**:
- Given: A page at `file:///test.html` containing `<iframe src="child.html">` which itself contains `<iframe src="grandchild.html">`
- When: `agentchrome page frames`
- Then: JSON array with 3 entries at depths 0, 1, and 2 respectively, each with the fields above

### AC2: Main frame listed at index 0

**Given** a page with or without iframes
**When** `page frames` is run
**Then** the main/top-level frame appears at index 0 with `depth: 0`
**And** iframe children follow in document order starting at index 1

### AC3: Target a specific frame with page commands

**Given** a `--frame <index>` argument on a page command (`page snapshot`, `page find`, `page screenshot`, `page text`, `page element`, `page wait`)
**When** the command is executed
**Then** execution targets the specified iframe context rather than the main frame
**And** the output reflects only the content within that frame

**Example**:
- Given: A page with an iframe at index 1 containing a button labeled "Submit"
- When: `agentchrome page snapshot --frame 1`
- Then: The accessibility tree snapshot contains the "Submit" button from the iframe, not the parent page content

### AC4: Target a specific frame with js exec

**Given** a `--frame <index>` argument on `js exec`
**When** `agentchrome js exec --frame 1 "document.title"` is executed
**Then** the JavaScript runs in the iframe's execution context
**And** `document.title` returns the iframe's document title, not the parent page's title

### AC5: Target a specific frame with dom commands

**Given** a `--frame <index>` argument on a dom command (`dom select`, `dom get-text`, `dom get-html`, `dom get-attribute`, `dom set-attribute`, `dom set-text`, `dom remove`, `dom get-style`, `dom set-style`, `dom parent`, `dom children`, `dom siblings`)
**When** the command is executed
**Then** DOM operations target elements within the specified iframe's document

### AC6: Target a specific frame with interact commands

**Given** a `--frame <index>` argument on an interact command (`interact click`, `interact click-at`, `interact hover`, `interact drag`, `interact type`, `interact key`, `interact scroll`)
**When** the command is executed
**Then** input dispatch targets the specified iframe context
**And** coordinates are relative to the iframe's viewport, not the parent page

### AC7: Target a specific frame with form commands

**Given** a `--frame <index>` argument on a form command (`form fill`, `form fill-many`, `form clear`, `form submit`)
**When** the command is executed
**Then** form operations target elements within the specified iframe's document

### AC8: Cross-origin iframe access

**Given** a cross-origin iframe on the page (e.g., iframe `src` on a different origin than the parent)
**When** frame targeting is used with `--frame <index>`
**Then** AgentChrome attaches to the frame via CDP and provides DOM, JS, and interaction access within that frame
**And** the behavior is functionally identical to same-origin frame targeting from the user's perspective

### AC9: Frame-scoped UIDs are consistent across commands

**Given** a `page snapshot --frame 1` that assigns UID `s3` to a button inside the iframe
**When** a subsequent command references that UID (e.g., `interact click --frame 1 s3` or `form fill --frame 1 s3 "value"`)
**Then** the UID resolves to the same element within the iframe context
**And** the command operates on the correct element

### AC10: No --frame flag defaults to main frame

**Given** a command executed without the `--frame` argument
**When** the command runs
**Then** behavior is identical to the current implementation (main frame context)
**And** no regression is introduced for existing workflows

### AC11: --frame 0 targets the main frame

**Given** a command executed with `--frame 0`
**When** the command runs
**Then** the command targets the main/top-level frame
**And** the result is identical to running the command without `--frame`

### AC12: Invalid frame index error

**Given** a `--frame` argument with an index that does not correspond to any frame on the page
**When** any command is run
**Then** a JSON error is returned on stderr: `{"error": "Frame index N not found. Use 'page frames' to list available frames.", "code": 3}`
**And** the process exits with code 3 (TargetError)

### AC13: Frame removed during command execution

**Given** a `--frame <index>` targeting a frame that is removed from the DOM after the command starts but before it completes
**When** the command encounters the missing frame
**Then** a JSON error is returned on stderr with a descriptive message indicating the frame is no longer available
**And** the process exits with code 3 (TargetError)

### AC14: Documentation and examples updated

**Given** the new frame targeting feature
**When** `agentchrome examples page` is run
**Then** frame targeting examples are included in the output (e.g., `page frames`, `page snapshot --frame 1`)
**And** when `agentchrome examples interact` is run, frame-scoped interaction examples are included
**And** help text for all affected commands documents the `--frame` parameter

### AC15: Automatic frame detection with --frame auto

**Given** a command with `--frame auto` and a UID or selector argument (e.g., `interact click --frame auto s5`)
**When** the target element is not found in the main frame
**Then** AgentChrome searches all child frames in document order for the matching UID or selector
**And** executes the command in the first frame where the target is found
**And** the JSON output includes a `"frame"` field indicating which frame index was used

**Example**:
- Given: A page where UID `s5` exists only inside an iframe at index 2
- When: `agentchrome interact click --frame auto s5`
- Then: The click targets the element in frame 2, and output includes `"frame": 2`

### AC16: Automatic frame detection with no match

**Given** a command with `--frame auto` and a UID or selector that does not exist in any frame
**When** the search exhausts all frames
**Then** a JSON error is returned on stderr: `{"error": "Element not found in any frame. Use 'page frames' to list available frames.", "code": 3}`
**And** the process exits with code 3 (TargetError)

### AC17: Nested iframe path syntax

**Given** a `--frame` argument with a slash-separated path (e.g., `--frame 1/0`)
**When** the command is executed
**Then** AgentChrome resolves the path by traversing frame children: first to child index 1 of the main frame, then to child index 0 of that frame
**And** the command executes in the resolved nested frame context

**Example**:
- Given: Main frame has 2 iframes (indices 1, 2). The iframe at index 1 has 1 child iframe.
- When: `agentchrome page snapshot --frame 1/0`
- Then: The snapshot targets the first child iframe of the iframe at flat index 1

### AC18: Invalid nested frame path error

**Given** a `--frame` argument with a path where any segment does not resolve (e.g., `--frame 1/5` when frame 1 has only 2 children)
**When** the command is run
**Then** a JSON error is returned on stderr indicating which path segment failed to resolve
**And** the process exits with code 3 (TargetError)

### AC19: List workers on a page

**Given** a page with one or more Service Workers or Web Workers registered
**When** `page workers` is run
**Then** JSON output on stdout lists all workers with fields: `index` (integer, 0-based), `id` (CDP target ID string), `type` (one of `"service_worker"`, `"shared_worker"`, `"worker"`), `url` (worker script URL), and `status` (worker lifecycle state)

### AC20: Target a worker with js exec

**Given** a `--worker <index>` argument on `js exec`
**When** `agentchrome js exec --worker 0 "self.registration.scope"` is executed
**Then** the JavaScript runs in the worker's execution context
**And** the result reflects the worker's scope, not the page context

### AC21: Invalid worker index error

**Given** a `--worker` argument with an index that does not correspond to any worker
**When** `js exec` is run
**Then** a JSON error is returned on stderr: `{"error": "Worker index N not found. Use 'page workers' to list available workers.", "code": 3}`
**And** the process exits with code 3 (TargetError)

### AC22: Frame-scoped network monitoring

**Given** a `--frame <index>` argument on `network list`
**When** `agentchrome network list --frame 1` is executed
**Then** only network requests initiated by the specified frame are included in the output
**And** requests from other frames are excluded

### AC23: Frame-scoped network interception

**Given** a `--frame <index>` argument on `network intercept`
**When** `agentchrome network intercept --frame 1 --url-pattern "*.js"` is executed
**Then** only requests from the specified frame matching the pattern are intercepted
**And** requests from other frames pass through unmodified

### AC24: Legacy frameset support

**Given** a page using `<frameset>` and `<frame>` elements (legacy HTML)
**When** `page frames` is run
**Then** each `<frame>` element appears in the frame list with the same metadata fields as iframes
**And** `--frame <index>` targeting works identically for `<frame>` elements as it does for `<iframe>` elements

### AC25: Shadow DOM traversal in snapshots

**Given** a page containing elements with shadow DOM roots (e.g., custom web components)
**When** `page snapshot --pierce-shadow` is run
**Then** the accessibility tree includes nodes from within shadow DOM roots
**And** UIDs are assigned to interactive elements inside shadow roots just like regular DOM elements

### AC26: Shadow DOM traversal in dom commands

**Given** a `--pierce-shadow` flag on a dom command (e.g., `dom select --pierce-shadow "#shadow-button"`)
**When** the command is executed
**Then** CSS selectors pierce shadow DOM boundaries to match elements inside shadow roots

### AC27: Shadow DOM traversal in interact and form commands

**Given** a UID assigned to an element inside a shadow root via `page snapshot --pierce-shadow`
**When** an interact or form command references that UID (e.g., `interact click <uid>`)
**Then** the command operates on the shadow DOM element
**And** no `--pierce-shadow` flag is needed on the interact/form command since the UID already identifies the element

### AC28: Combined frame and shadow DOM targeting

**Given** an iframe containing a web component with a shadow DOM root
**When** `page snapshot --frame 1 --pierce-shadow` is run
**Then** the snapshot includes shadow DOM content within the targeted iframe
**And** UIDs assigned to shadow DOM elements inside the iframe work with subsequent `--frame 1` commands

### Generated Gherkin Preview

```gherkin
Feature: Iframe/Frame Targeting Support
  As a browser automation engineer working with enterprise web applications
  I want to target specific iframe contexts when running AgentChrome commands
  So that I can automate applications that embed content in iframes

  Scenario: List frames on a page
    Given a page with nested iframes is loaded
    When "page frames" is run
    Then JSON output lists all frames with index, id, url, name, securityOrigin, unreachable, width, height, and depth

  Scenario: Main frame listed at index 0
    Given a page with iframes is loaded
    When "page frames" is run
    Then the main frame appears at index 0 with depth 0

  Scenario: Target a specific frame with page snapshot
    Given a page with an iframe containing unique content
    When "page snapshot --frame 1" is run
    Then the snapshot contains only the iframe's accessibility tree

  Scenario: Target a specific frame with js exec
    Given a page with an iframe that has a different document title
    When "js exec --frame 1 'document.title'" is run
    Then the result is the iframe's document title

  Scenario: Target a specific frame with dom commands
    Given a page with an iframe containing a specific element
    When "dom select --frame 1 '#element'" is run
    Then the element from the iframe is returned

  Scenario: Target a specific frame with interact commands
    Given a page with an iframe containing an interactive element
    When "interact click --frame 1 <uid>" is run
    Then the click targets the element inside the iframe

  Scenario: Target a specific frame with form commands
    Given a page with an iframe containing a form
    When "form fill --frame 1 <uid> 'value'" is run
    Then the form field inside the iframe is filled

  Scenario: Cross-origin iframe access
    Given a page with a cross-origin iframe
    When frame targeting is used with "--frame <index>"
    Then CDP provides DOM, JS, and interaction access within the cross-origin frame

  Scenario: Frame-scoped UIDs are consistent across commands
    Given a snapshot assigned UID s3 to a button in frame 1
    When "interact click --frame 1 s3" is run
    Then the click targets the same button

  Scenario: No --frame flag defaults to main frame
    Given a page with iframes
    When a command is run without --frame
    Then the main frame is targeted

  Scenario: --frame 0 targets the main frame
    Given a page with iframes
    When a command is run with "--frame 0"
    Then the main frame is targeted

  Scenario: Invalid frame index error
    Given a "--frame 99" argument on a page with fewer frames
    When the command is run
    Then a JSON error with code 3 is returned on stderr

  Scenario: Frame removed during command execution
    Given a "--frame 1" targeting a frame that is removed
    When the command encounters the missing frame
    Then a JSON error with code 3 is returned on stderr

  Scenario: Documentation and examples updated
    Given the frame targeting feature is implemented
    When "examples page" is run
    Then frame targeting examples are included in the output

  Scenario: Automatic frame detection with --frame auto
    Given a page where UID s5 exists only inside an iframe at index 2
    When "interact click --frame auto s5" is run
    Then the click targets the element in frame 2
    And the output includes a "frame" field indicating frame index 2

  Scenario: Automatic frame detection with no match
    Given a page where UID s99 does not exist in any frame
    When "interact click --frame auto s99" is run
    Then a JSON error with code 3 is returned on stderr

  Scenario: Nested iframe path syntax
    Given a page with nested iframes
    When "page snapshot --frame 1/0" is run
    Then the snapshot targets the first child of the iframe at index 1

  Scenario: Invalid nested frame path error
    Given "--frame 1/5" where frame 1 has fewer than 6 children
    When the command is run
    Then a JSON error with code 3 is returned indicating which path segment failed

  Scenario: List workers on a page
    Given a page with a registered Service Worker
    When "page workers" is run
    Then JSON output lists workers with index, id, type, url, and status

  Scenario: Target a worker with js exec
    Given a page with a Service Worker at index 0
    When "js exec --worker 0 'self.registration.scope'" is run
    Then the result reflects the worker's scope

  Scenario: Invalid worker index error
    Given "--worker 99" on a page with no workers
    When "js exec" is run
    Then a JSON error with code 3 is returned on stderr

  Scenario: Frame-scoped network monitoring
    Given a page with an iframe making network requests
    When "network list --frame 1" is run
    Then only requests from frame 1 are included

  Scenario: Frame-scoped network interception
    Given a page with an iframe
    When "network intercept --frame 1 --url-pattern '*.js'" is run
    Then only requests from frame 1 matching the pattern are intercepted

  Scenario: Legacy frameset support
    Given a page using frameset and frame elements
    When "page frames" is run
    Then each frame element appears in the frame list
    And "--frame <index>" targeting works for frame elements

  Scenario: Shadow DOM traversal in snapshots
    Given a page with web components using shadow DOM
    When "page snapshot --pierce-shadow" is run
    Then the accessibility tree includes nodes from shadow roots

  Scenario: Shadow DOM traversal in dom commands
    Given a shadow DOM element with id "shadow-button"
    When "dom select --pierce-shadow '#shadow-button'" is run
    Then the element inside the shadow root is returned

  Scenario: Shadow DOM in interact and form via UID
    Given a UID assigned to a shadow DOM element via pierce-shadow snapshot
    When "interact click <uid>" is run
    Then the shadow DOM element is clicked without needing --pierce-shadow

  Scenario: Combined frame and shadow DOM targeting
    Given an iframe containing a web component with shadow DOM
    When "page snapshot --frame 1 --pierce-shadow" is run
    Then shadow DOM content within the iframe is included in the snapshot
```

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | New `page frames` subcommand listing all frames with metadata (index, id, url, name, securityOrigin, unreachable, width, height, depth) | Must | Returns JSON array on stdout |
| FR2 | `--frame <index>` global-style parameter on all page, dom, interact, form, and js commands | Must | 0-based index matching `page frames` output |
| FR3 | CDP frame attachment for cross-origin iframes (out-of-process iframes) via `Target.attachToTarget` with `flatten: true` or equivalent | Must | Must handle both same-origin and cross-origin frames |
| FR4 | Frame-scoped accessibility tree snapshots (`page snapshot --frame`) returning only the subtree within the target frame | Must | UIDs assigned within frame scope must be usable by subsequent frame-targeted commands |
| FR5 | Frame-scoped JavaScript execution (`js exec --frame`) running code in the frame's execution context | Must | Uses `Runtime.evaluate` with the frame's execution context ID |
| FR6 | Frame-scoped input dispatch for interact commands, with coordinates relative to the iframe viewport | Must | Must handle coordinate translation for `click-at`, `hover`, `drag` |
| FR7 | Frame-scoped DOM operations for all dom subcommands | Must | CSS selectors resolve within the frame's document |
| FR8 | Frame-scoped form operations for all form subcommands | Must | UID resolution within frame context |
| FR9 | JSON error on stderr with exit code 3 for invalid frame index | Must | Message includes suggestion to use `page frames` |
| FR10 | Help documentation and built-in examples updated for frame targeting | Must | `examples page`, `examples interact`, and `--help` text |
| FR11 | Frame enumeration uses CDP `Page.getFrameTree` to build the frame hierarchy | Must | Depth-first document order for consistent indexing |
| FR12 | Snapshot state persistence includes frame context so UIDs are resolvable in subsequent frame-targeted commands | Must | Per retrospective: cross-invocation state must be observable |
| FR13 | `--frame auto` mode that searches all frames for the target UID or selector and executes in the first matching frame | Must | Output includes `"frame"` field indicating which frame was used |
| FR14 | `--frame` accepts slash-separated path syntax (e.g., `1/0`) for nested iframe targeting by parent-child traversal | Must | Each segment indexes into the children of the resolved frame |
| FR15 | New `page workers` subcommand listing all Service Workers, Shared Workers, and Web Workers with metadata (index, id, type, url, status) | Must | Returns JSON array on stdout |
| FR16 | `--worker <index>` parameter on `js exec` for executing JavaScript in a worker's execution context | Must | 0-based index matching `page workers` output |
| FR17 | `--frame <index>` parameter on `network list` to filter network requests by originating frame | Must | Frame ID matched against request's `frameId` field |
| FR18 | `--frame <index>` parameter on `network intercept` to scope interception rules to a specific frame | Must | Only intercepts requests from the specified frame |
| FR19 | `page frames` and `--frame` support legacy `<frame>` / `<frameset>` elements identically to `<iframe>` elements | Must | CDP `Page.getFrameTree` already enumerates frameset frames |
| FR20 | `--pierce-shadow` flag on `page snapshot` to include shadow DOM content in the accessibility tree | Must | UIDs assigned to shadow DOM elements are usable by subsequent commands |
| FR21 | `--pierce-shadow` flag on all dom subcommands to pierce shadow DOM boundaries for CSS selector resolution | Must | Selectors match elements inside open shadow roots |
| FR22 | Interact and form commands operate on shadow DOM elements via UID without requiring `--pierce-shadow` (the UID from a pierce-shadow snapshot is sufficient) | Must | UID resolution uses backendDOMNodeId which is shadow-DOM-agnostic |
| FR23 | `--frame` and `--pierce-shadow` flags can be combined to target shadow DOM content within an iframe | Must | Flags are orthogonal and composable |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | `page frames` completes in < 200ms. Adding `--frame` to an existing command adds < 100ms overhead for frame resolution |
| **Reliability** | Frame attachment gracefully handles frames that become unavailable (navigation, removal) with descriptive errors |
| **Platforms** | macOS, Linux, Windows — no platform-specific behavior for frame targeting |
| **Backwards Compatibility** | All existing commands without `--frame` behave identically to current implementation. No breaking changes. |

---

## UI/UX Requirements

| Element | Requirement |
|---------|-------------|
| **CLI Argument** | `--frame` accepts an integer, slash-separated path, or `auto`. `--worker` accepts an integer. `--pierce-shadow` is a boolean flag. |
| **Error States** | Invalid frame/worker index and unresolvable path produce structured JSON error on stderr with exit code 3 and actionable message |
| **Discovery** | `page frames` and `page workers` provide the information needed to select the correct `--frame` / `--worker` values |
| **Help Text** | Each affected command's `--help` includes descriptions of `--frame`, `--worker`, and/or `--pierce-shadow` as applicable |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--frame` | String | Non-negative integer, slash-separated path (e.g., `1/0`), or literal `auto`. Must resolve to a valid frame. | No (defaults to main frame) |
| `--worker` | Non-negative integer (u32) | Must be a valid index from `page workers` output. Only valid on `js exec`. | No |
| `--pierce-shadow` | Boolean flag | No value required. Valid on `page snapshot` and all `dom` subcommands. | No (defaults to false) |

### Output Data — `page frames`

| Field | Type | Description |
|-------|------|-------------|
| `index` | integer | 0-based position in document-order traversal. 0 is always the main frame. |
| `id` | string | CDP frame ID (opaque identifier) |
| `url` | string | The frame's current URL. Empty string if not yet navigated. |
| `name` | string | The `name` attribute of the iframe element, or empty string if absent |
| `securityOrigin` | string | The frame's security origin (e.g., `https://example.com`) |
| `unreachable` | boolean | `true` if the frame's security origin is marked unreachable by CDP (failed load). `false` otherwise. |
| `width` | integer | Frame viewport width in CSS pixels. `0` if dimensions cannot be determined. |
| `height` | integer | Frame viewport height in CSS pixels. `0` if dimensions cannot be determined. |
| `depth` | integer | Nesting level. `0` for the main frame, `1` for direct child iframes, `2` for grandchild iframes, etc. |

### Output Data — `page workers`

| Field | Type | Description |
|-------|------|-------------|
| `index` | integer | 0-based position in enumeration order. |
| `id` | string | CDP target ID for the worker. |
| `type` | string | One of `"service_worker"`, `"shared_worker"`, or `"worker"` (dedicated web worker). |
| `url` | string | The worker script URL. |
| `status` | string | Worker lifecycle state (e.g., `"activated"`, `"installing"`, `"redundant"` for service workers; `"running"` for web/shared workers). |

---

## Dependencies

### Internal Dependencies
- [x] CDP client WebSocket communication (`src/cdp/client.rs`) — already supports session-scoped commands
- [x] Frame ID tracking in navigation events (`src/navigate.rs`) — partial, needs extension
- [x] Accessibility tree snapshot (`src/snapshot.rs`) — needs frame-scoping
- [x] Session/connection management (`src/connection.rs`) — needs frame resolution step

### External Dependencies
- [x] Chrome DevTools Protocol — `Page.getFrameTree`, `Target.attachToTarget`, `Runtime.evaluate` with execution context, `Page.createIsolatedWorld`, `Target.getTargets` (for workers), `DOM.describeNode` (for shadow DOM)

### Blocked By
- None — all prerequisites exist in the codebase

---

## Out of Scope

- Frame-level cookie isolation (cookies are managed at the browser/profile level, not per-frame)
- Closed shadow DOM piercing (only open shadow roots are accessible via CDP)
- Worker-level DOM access (workers have no DOM; only JS execution is supported via `--worker`)

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Frame resolution overhead | < 100ms added to command execution | Benchmark `page snapshot` vs `page snapshot --frame 0` |
| `page frames` latency | < 200ms | Benchmark on a page with 10 iframes |
| `--frame auto` search latency | < 500ms on a page with 10 frames | Benchmark with UID in last frame |
| `page workers` latency | < 200ms | Benchmark on a page with 3 workers |
| Command coverage — `--frame` | 100% of page/dom/interact/form/js/network commands support `--frame` | Verify via `--help` output audit |
| Command coverage — `--pierce-shadow` | 100% of page snapshot and dom commands support `--pierce-shadow` | Verify via `--help` output audit |

---

## Open Questions

- None — the CDP APIs for frame targeting are well-documented and the implementation path is clear

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #189 | 2026-04-15 | Initial feature spec |

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements (CDP methods mentioned only in background/FR notes)
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified (AC10-AC13)
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented (or resolved)
