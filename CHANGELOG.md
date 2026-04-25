# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.51.0] - 2026-04-24

### Added

- Add Codex as a first-class `agentchrome skill` target, including `$CODEX_HOME` and `~/.codex` install path resolution, auto-detection, lifecycle support, stale-skill checks, documentation, and BDD coverage. (#263)

## [1.50.0] - 2026-04-23

### Fixed

- `agentchrome interact click "css:<selector>"` now reliably fires inline `onclick` handlers. The CSS-selector resolution path previously went through `DOM.getDocument` + `DOM.querySelector`, returning a `nodeId` scoped to a document handle that could be stale relative to the live layout; the dispatched mouse event then landed on coordinates that no longer hit the target. Resolution now goes through `Runtime.evaluate(document.querySelector(...))` + `DOM.describeNode { objectId }`, binding the node to the live document and matching the UID path's stable `backendNodeId` semantics. The UID and `click-at` paths are unchanged. (#252)

## [1.49.0] - 2026-04-23

### Fixed

- `config init --config <path>` now honours the supplied path: the template is written to the specified location, the JSON `created` field reflects that path, and the process exits 0. Previously the global `--config` resolution treated the supplied path as a missing config file to read, causing the command to ignore the flag, write to the default XDG path, and exit 1 despite reporting success on stdout. When `--config` is absent the default XDG behaviour is preserved; when the parent directory does not exist the command exits 1 with a clear stderr error. (#249)

## [1.48.0] - 2026-04-23

### Fixed

- `dom tree` now accepts an optional positional `ROOT` argument (e.g., `agentchrome dom tree css:table`), consistent with all other `dom` subcommands. The `--root` flag is retained for backward compatibility; supplying both simultaneously produces a clap conflict error. (#251)

## [1.47.0] - 2026-04-23

### Fixed

- `agentchrome dialog handle --accept` / `--dismiss` now emit a corrected `Did you mean: agentchrome dialog handle accept` (or `dismiss`) hint instead of clap's misleading `-- --accept` tip that led users toward another invalid invocation. The `syntax_hint` helper in `src/main.rs` recognizes these flags only on the `dialog handle` subcommand; `--uid` / `--selector` hint paths are unchanged. (#250)

## [1.46.0] - 2026-04-23

### Fixed

- `script run` now auto-unwraps the `js exec` output envelope at the bind site so `$vars.<name>` holds the scalar (or object/array) `result` directly instead of `{result, truncated, type}`. Expressions like `$vars.t.includes('Internet')` evaluate without `TypeError`, and `$vars.obj.<field>` resolves on returned objects. Bind shape for other commands (`page find`, `page text`, `page screenshot`, `navigate`, …) is unchanged, and `agentchrome js exec` stdout outside scripts still emits the full envelope. (#248)

## [1.45.0] - 2026-04-23

### Added

- Enable `page find` and `page screenshot` inside `script run` batch scripts. The script runner now dispatches both subcommands via `page::run_from_session`, allowing bind-then-use flows (e.g. `page find` → `$vars.match[0].uid` → `interact click`) and inline screenshot capture. The "not yet supported" guard message was updated to reflect the expanded whitelist (`snapshot`, `text`, `find`, `screenshot`). `examples script` samples extended to demonstrate the new patterns. (#247)

## [1.44.0] - 2026-04-23

### Fixed

- `form fill-many` now accepts `target` as the primary JSON key for each entry, matching the field name used throughout the rest of the `form` subcommand family (`form fill <target>`). The legacy `uid` key remains accepted as a silent alias for backward compatibility. The deserialization error message, `--help` `long_about`, inline examples, and `examples_data.rs` strategies now reference `target` consistently. (#246)

## [1.43.0] - 2026-04-22

### Added

- Enrich installed SKILL.md template with YAML frontmatter (`name`, `description`, `version`) and body entries for `diagnose`, `examples strategies`, `--include-snapshot`, and the temp-file response pattern — so agent discovery works out-of-the-box. (#220)
- Extend `output::emit` temp-file gating to `audit`, `dom select/attributes/events`, `page analyze/find`, `console read`, and every `--include-snapshot` interaction code path; each gated command defines a domain-appropriate `summary` shape. (#220)
- Add compound `--include-snapshot` output schema that preserves interaction confirmation fields inline while offloading the large snapshot payload to a temp file. (#220)
- Add skill-staleness check: on every invocation, compare the installed skill's `Version:` marker against `CARGO_PKG_VERSION` and emit a single stderr notice when stale; checks all known tool install locations, aggregating if multiple are stale; suppressible via `AGENTCHROME_NO_SKILL_CHECK=1` or `skill_check_enabled = false`; adds <1 ms overhead. (#220)
- Add `capabilities <command>` detail path gated by `output::emit` when response exceeds threshold. (#220)
- BDD coverage: new `tests/features/skill-staleness.feature` (AC6–AC8) and extended `tests/features/large-response-detection.feature` with scenarios for all newly gated command paths, compound-schema, and SKILL.md frontmatter assertions. (#220)

## [1.42.0] - 2026-04-22

### Changed

- Enrich generated man pages with the structured `capabilities` and `examples` content so `agentchrome man <cmd>` is self-sufficient — the EXAMPLES section now reflects every entry from `agentchrome examples <cmd>`, and a capabilities-manifest entry (purpose, inputs, outputs, exit codes) is rendered in man-page format. Enrichment happens at `cargo xtask man` build time so runtime stays within the <50ms startup target and man output remains deterministic in CI. Examples and capabilities data now live in shared modules (`src/examples_data.rs`, `src/capabilities_cli.rs`) so the `examples`/`capabilities` subcommands and the man pipeline share a single source of truth. (#232)

## [1.41.0] - 2026-04-22

### Added

- Add `agentchrome audit lighthouse --install-prereqs` flag that runs `npm install -g lighthouse` with the flag itself as consent and reports structured JSON on stdout/stderr; probes `npm --version` first and returns an actionable error naming Node.js as the upstream prerequisite when npm is absent. (#231)
- Surface the `lighthouse` npm prerequisite in `audit lighthouse --help` (PREREQUISITES section above EXAMPLES) and in the `audit` group's one-line description on both `agentchrome --help` and `agentchrome audit --help`, so the advertised-but-broken first-run impression is neutralized. (#231)

### Changed

- Extend the `lighthouse` binary-not-found error to point at `--install-prereqs` alongside the existing `npm install -g lighthouse` hint, emitted as a single structured JSON error object per invocation. (#231)

## [1.40.0] - 2026-04-22

### Added

- Normalize flag shapes across related subcommands so first-time users and AI agents hit the canonical form on the first guess: `cookie set --url <URL>` is now accepted as a hidden alias for `--domain` (folded via `url::Url::parse` + `host_str()`); `tabs close --tab <ID>` is accepted as a hidden alias for the positional target list (merged with positional IDs preserving order); and `dom query` is accepted as a hidden alias for `dom select`. Canonical forms, `--help`, `examples`, and the capabilities manifest are unchanged — aliases are `hide = true` / subcommand aliases so there remains one documented shape per command. Passing both `--url` and `--domain` to `cookie set`, or passing a malformed / host-less URL, errors with a message that names `--domain` as the canonical alternative. (#230)

## [1.39.3] - 2026-04-22

### Fixed

- Fix `agentchrome js exec "<expr>" --plain` zero-byte stdout for empty-string results: the `--plain` branch now emits the two-byte JSON literal `""` when the evaluated expression returns `""`, so callers can distinguish "empty-string result" from "no output". Non-empty strings, numbers, booleans, null, undefined, and `--pretty`/default JSON output paths are unchanged. (#229)

## [1.39.2] - 2026-04-22

### Fixed

- Fix `agentchrome console follow --timeout <ms>` default exit code so it returns 0 when the timeout elapses even if `console.error` messages were observed, restoring tail-style monitoring semantics. The previous error-coupled exit-1 behavior is now opt-in via a new `--fail-on-error` flag, preserving the CI assertion use case. `console follow --help`, the `examples` listing, and the capabilities manifest document both modes with worked examples. (#228)

## [1.39.1] - 2026-04-22

### Fixed

- Fix `agentchrome interact key <KEY>` so `keyup` listeners on the target page observe the dispatched key: `Input.dispatchKeyEvent` payloads now include `windowsVirtualKeyCode` on both `keyDown` and `keyUp`, include `text` on `keyDown` for printable keys, and `cdp_key_value` returns the proper `"Enter"`/`"Tab"` strings (previously `"\r"` / `"\t"`) so page listeners reading `event.key` observe the expected value. Modifier combinations and `interact type` char-synthesis are unchanged. (#227)

## [1.39.0] - 2026-04-21

### Added

- Add `agentchrome connect --status` subcommand that reports the active session as JSON `{active, ws_url, port, pid, timestamp, ...}` with exit code 0 whether a session exists or not, so scripts and agents can probe discovery state without conflating "no session" with an error. (#226)
- Document session file path (`~/.agentchrome/session.json` on Unix; `%USERPROFILE%\.agentchrome\session.json` on Windows) and auto-discovery precedence (flag → env var → session file → default 9222) in `connect --help` long-form text. (#226)
- Emit structured stderr warning when the persisted session file points to an unreachable port, rather than silently falling back to the default. (#226)

### Fixed

- Harden Windows auto-discovery so `connect --launch` persistence is reliably observed by subsequent invocations in new shells without `--port` / `AGENTCHROME_PORT`: verify `USERPROFILE` resolution, atomic writes, and round-trip JSON parsing. (#226)

## [1.38.0] - 2026-04-21

### Fixed

- Fix dialog handling across separate `agentchrome` invocations: `interact` entry points (click, click_at, scroll, and other dialog-triggering actions) now install dialog interceptors via `setup_session_with_interceptors`, so Process 1 writes the `__agentchrome_dialog` cookie before exiting and Process 2's `dialog info` / `dialog handle` can observe the open dialog. `--auto-dismiss-dialogs` on click now awaits a bounded post-dispatch settle so the dismiss lands reliably on the clicking session. (#225)

## [1.37.0] - 2026-04-21

### Changed

- **BREAKING**: `agentchrome examples --json` top-level listing now returns only `{command, description}` per entry; the nested `examples` array is removed from the listing payload. Fetch individual command-group examples via `agentchrome examples <group> --json` (unchanged). (#218)
- **BREAKING**: `agentchrome capabilities --json` listing now returns `{name, description}` per command entry; `subcommands`, `args`, and `flags` are no longer present on listing entries. (#218)
- **BREAKING**: The `--command <name>` flag on `capabilities` has been replaced by a positional: `agentchrome capabilities <command>` returns the full `CommandDescriptor` (subcommands, args, flags) for the named command. (#218)

### Added

- Add `agentchrome capabilities <command>` detail path that returns the full descriptor for a single command, complementing the lightweight listing. (#218)

## [1.35.0] - 2026-04-21

### Added

- Add `script run <file>` subcommand for batch execution of AgentChrome commands from a JSON script file, with sequential execution, fail-fast mode (`--fail-fast`), conditional branching (if/else), loop constructs (count and condition-based), variable binding for passing output between commands, and `--dry-run` validation; stdin supported via `-` as the file argument, reducing multi-step workflows (e.g., 92-slide SCORM courses) from hundreds of tool calls to one (#199)
- Add `run_from_session` adapters across all command modules enabling script-driven reuse of existing CDP sessions without per-command reconnect overhead (#199)
- Add BDD coverage in `tests/features/batch-script-execution.feature` for script execution, fail-fast, branching, loops, variable binding, and dry-run (#199)

## [1.34.0] - 2026-04-21

### Added

- Add structured JSON error output on all command failure paths with consistent `{"error", "code"}` schema on stderr, including optional `custom_json` context for enriched diagnostics (#197)
- Add descriptive error for `form fill` on incompatible element types (e.g., `div`, `canvas`) identifying the element type and suggesting fillable alternatives (#197)
- Add syntax-suggestion hints for common clap argument mistakes (e.g., `interact click --uid s6` → suggests `interact click s6`) (#197)
- Add error-handling guidance to help documentation explaining the structured stderr format and exit codes (0=success, 1=general, 2=connection, 3=target, 4=timeout, 5=protocol) (#197)
- Add BDD coverage in `tests/features/197-improve-error-output-consistency.feature` for all acceptance criteria covering silent-failure audits, form-fill element type errors, and syntax suggestions (#197)

### Fixed

- Fix silent failure paths in `form.rs`, `interact.rs`, and `page/wait.rs` that previously exited with code 1 without writing structured error output to stderr (#197)

## [1.33.1] - 2026-04-19

### Fixed

- Anchor `package.exclude` patterns to repo root (`/examples/`, `/tests/`, etc.) so the gitignore-style globs no longer strip `src/examples/` and other in-tree module directories from the published crate tarball, unblocking `cargo publish` for 1.33.x

## [1.33.0] - 2026-04-19

### Added

- Add WebSocket keep-alive pings with pong watchdog and `--keepalive-interval` / `--no-keepalive` global flags (env: `AGENTCHROME_KEEPALIVE_INTERVAL`, config: `[keepalive].interval_ms`, default 30 s) to keep long-running CDP sessions alive across idle gaps (#185)
- Add invocation-level auto-reconnect: when the cached `ws_url` is stale or rotated, every command now rediscovers Chrome on the stored port via the bounded `ReconnectPolicy` (max attempts, exponential backoff, per-probe `probe_timeout_ms`) and rewrites the session file in-band, preserving `pid`, `port`, and unrelated fields (#185)
- Add structured connection-loss errors distinguishing `kind: "chrome_terminated"` (`recoverable: false`, suggests `connect --launch`) from `kind: "transient"` (`recoverable: true`, suggests `connect`) via a PID liveness probe in `chrome::platform` (#185)
- Add `last_reconnect_at` (ISO 8601) and `reconnect_count` to the session file, surfaced via `connect --status` along with the effective keep-alive interval and enabled flag (#185)
- Add `[keepalive]` and `[reconnect]` sections to the config file with precedence ordering CLI > env > config > built-in default; `--no-keepalive` and `--keepalive-interval 0` always disable pings (#185)
- Add README "Session resilience" section documenting auto-reconnect, keep-alive flag/env/config, disable mechanism, and `kind` / `recoverable` scripting guidance, plus capabilities-manifest and man-page coverage of the new flags (#185)
- Add BDD coverage for AC21–AC36 in `tests/features/185-session-reconnect-keepalive.feature`, including Scenario Outlines for uniform reconnect across commands and keep-alive interval precedence (#185)

### Fixed

- Fix reconnect fast-path so a `/json/version` ping that returns a rotated `ws_debugger_url` rewrites the session file instead of returning the stale URL (#185)

## [1.32.0] - 2026-04-18

### Added

- Add `--pierce-shadow` flag to `page snapshot` to include open shadow DOM content in the accessibility tree, plus supplemental shadow-root traversal that merges interactive nodes under their shadow host (#181)
- Add `--include-iframes` to `page snapshot` to produce a single aggregated accessibility tree that inlines every enumerable iframe's content under its owner `<iframe>` node with a `"frame": <index>` annotation, and extend `SnapshotState` with aggregate UID routing so subsequent `interact`, `form`, and `dom` commands resolve UIDs through the originating frame without an explicit `--frame` (#181)
- Add `--deep` flag to `page text` to extract visible text from the main frame, every iframe, and all open shadow roots in document order, mutually exclusive with `--frame` (#181)
- Add BDD coverage for AC29-AC32 covering aggregate snapshot/text modes, combined `--include-iframes --pierce-shadow`, and idempotency on pages without nested content (#181)

## [1.31.0] - 2026-04-17

### Added

- Add `examples strategies` subcommand with scenario-based interaction strategy guides (`iframes`, `overlays`, `scorm`, `drag-and-drop`, `shadow-dom`, `spa-navigation-waits`, `react-controlled-inputs`, `debugging-failed-interactions`, `authentication-cookie-reuse`, `multi-tab-workflows`), accessible via `examples strategies` for listings and `examples strategies <name>` for full guide detail, with progressive-disclosure JSON output (lightweight summaries for listings, full bodies only on explicit selection) (#201)
- Add "strategies" entry to the top-level `examples` listing alongside existing command groups (#201)
- Update `examples --help` long-help and `after_long_help` EXAMPLES to cover the new strategies path (#201)

## [1.30.0] - 2026-04-16

### Added

- Add `page coords --selector <target>` command returning frame-local and page-level coordinates, bounding box, center, and frame offset as structured JSON, with `--frame` support for iframe-scoped resolution (#198)
- Add `--relative-to <selector>` flag and percentage syntax (e.g., `50%`) to `interact click-at`, `drag-at`, `mousedown-at`, and `mouseup-at` for element-anchored, frame-aware coordinate dispatch that survives viewport dimension shifts (#198)
- Update `examples interact` and `examples page` with coordinate helper examples (#198)

## [1.29.0] - 2026-04-16

### Added

- Add top-level `diagnose` command that scans a page for automation challenges (iframes, overlays, shadow DOM, canvas/WebGL, media gates, framework quirks) and matches known patterns (Storyline acc-blocker, SCORM player, React portal), emitting a structured JSON report with per-category severity and actionable strategy suggestions. Supports both URL mode and `--current` mode. (#200)

## [1.28.0] - 2026-04-16

### Added

- Add Google Gemini CLI as a 7th supported agentic tool for `skill install`, with a standalone skill file at `~/.gemini/instructions/agentchrome.md` plus Tier 1 env-var (`GEMINI_*`) and Tier 3 directory (`~/.gemini/`) auto-detection (#214)

## [1.27.0] - 2026-04-16

### Added

- Add ARIA combobox support to `form fill` with automatic click-type-confirm sequence for elements with `role="combobox"`, eliminating the need for manual 3-step interaction workarounds (#196)
- Add `--confirm-key` option to `form fill` for customizing the confirmation key used in combobox interactions (default: Enter) (#196)
- Add combobox examples to `examples form` output (#196)

## [1.26.0] - 2026-04-16

### Added

- Add `--js-expression` condition to `page wait` for blocking on arbitrary JavaScript expressions evaluated to truthy via `Runtime.evaluate`, with consecutive error detection that reports expression failures after 3 consecutive JS exceptions (#195)
- Add `--count` modifier for `page wait --selector` to wait until at least N elements match the selector, enabling dynamic content count thresholds without custom JS polling (#195)
- Add frame-scoped support (`--frame`) for `--js-expression` and `--count` conditions (#195)

### Fixed

- Fix intermittent exit code 1 in `page wait` when poll-based conditions are already satisfied on first check, caused by transient CDP evaluation errors during page load being treated as failures instead of retried (#195)

## [1.25.0] - 2026-04-16

### Added

- Add `interact drag-at` command for coordinate-based drag sequences (mousedown → interpolated mousemoves → mouseup) with `--steps` for smooth interpolation (#194)
- Add `interact mousedown-at` and `interact mouseup-at` decomposed mouse commands for long-press, hover-then-click, and custom multi-step mouse interaction patterns (#194)
- Add `--button` option (left/middle/right) on decomposed mouse commands and `--frame` support on all new coordinate-based commands (#194)

## [1.24.0] - 2026-04-16

### Added

- Add `--selector` and `--uid` flags to `interact scroll` for targeting specific scrollable inner containers by CSS selector or accessibility UID, with scrollability validation that returns a descriptive error for non-scrollable targets (#182)

### Fixed

- Fix panic in `interact scroll` when `--uid` references a missing UID by replacing `unwrap()` with proper error propagation (#182)

## [1.23.0] - 2026-04-16

### Added

- Add scope isolation for `js exec` by wrapping expressions in block scope so `let`/`const` re-declarations across sequential invocations no longer cause `SyntaxError`. Add `--stdin` flag as a discoverable alias for piping JavaScript via stdin, and `--code` named argument for cross-platform quoting resilience (avoids PowerShell single-quote issues). (#183)

## [1.22.0] - 2026-04-16

### Added

- Add `--scroll-container` flag to `page screenshot --full-page` for capturing inner scrollable containers whose content overflows beyond the viewport. Auto-detects and warns when full-page dimensions match viewport dimensions, suggesting `--scroll-container` usage. Includes validation for flag conflicts and viewport restoration after capture (#184)

## [1.21.0] - 2026-04-16

### Added

- Add `media` command group for audio/video element control: `media list` to enumerate all media elements with playback state, `media play`, `media pause`, `media seek <time>`, and `media seek-end` for individual element control, `--all` flag for bulk operations across all media elements, `--frame` support for iframe-scoped targeting, CSS selector targeting via `css:` prefix, and built-in examples. Eliminates repetitive `js exec` boilerplate for SCORM course narration gates (#193)

## [1.20.0] - 2026-04-16

### Added

- Add `page analyze` command for page structure discovery that reveals iframes (with URLs, visibility, dimensions, cross-origin detection), frontend framework detection (React, Angular, Vue, Svelte, Storyline, SCORM), interactive element counts per frame, media element cataloging with playback state, overlay/blocker detection, and accessibility shadow DOM presence — all in a single structured JSON call. Supports frame-scoped analysis via `--frame` and includes built-in examples (#190)

## [1.19.0] - 2026-04-16

### Added

- Add `page hittest` command for click debugging that reveals the actual hit target, intercepting overlays, and stacked elements at viewport coordinates via CDP `DOM.getNodeForLocation` and `elementsFromPoint()`. Includes overlay detection with workaround suggestions, z-index stack enumeration, frame-scoped hit testing with `--frame`, and built-in examples (#191)

## [1.18.0] - 2026-04-16

### Added

- Add `dom events` subcommand for event listener introspection via CDP `DOMDebugger.getEventListeners`, returning structured JSON with event type, capture/bubble phase, handler source location, `once` flag, and `passive` flag. Supports UID and CSS selector targeting with `--frame` for frame-scoped queries (#192)

## [1.17.0] - 2026-04-15

### Added

- Add iframe/frame targeting support with `page frames` command to list all frames, `--frame <index>` parameter on all page, dom, interact, form, js, and network commands for targeting specific iframe contexts, CDP frame session attachment for cross-origin iframes (OOPIFs), frame-scoped accessibility tree snapshots, JS execution, input dispatch with coordinate translation, and `page workers` command for worker enumeration (#189)

## [1.16.0] - 2026-04-15

### Added

- Add `--compact` flag to `page snapshot` and all `--include-snapshot` commands for AI agent token efficiency. Compact mode filters the accessibility tree to only interactive elements (with UIDs), landmarks, and structural nodes — reducing output by 50%+ while preserving all actionable information (#162)

## [1.15.0] - 2026-03-16

### Added

- Add `audit lighthouse` command for running Google Lighthouse audits via the CLI, returning structured JSON category scores (Performance, Accessibility, Best Practices, SEO, PWA) on stdout. Supports `--only` category filtering, `--output-file` for full report, and optional URL override (#169)

## [1.14.0] - 2026-03-15

### Fixed

- Add `--wait-for-selector` flag to `navigate` so the command waits until a CSS selector is present in the DOM after the primary load strategy completes, fixing premature returns on SPA-heavy sites like Outlook Web (#178)

## [1.13.0] - 2026-03-15

### Changed

- Replace large-response guidance object with temp file output: responses exceeding the threshold are now written to a UUID-named file in the OS temp directory, with the file path returned on stdout as a `TempFileOutput` object (#177)

### Removed

- Remove `--search` per-command flag from `page snapshot`, `page text`, `js exec`, `network list`, and `network get` (#177)
- Remove `--full-response` global flag (#177)

## [1.12.0] - 2026-03-12

### Added

- Add `--page-id` global flag for stateless page routing, enabling parallel agents to target specific pages by CDP target ID without interfering with shared session state (#170)

## [1.11.0] - 2026-03-12

### Fixed

- Fix `form fill` silently failing on React-controlled inputs by using keyboard simulation (`Input.dispatchKeyEvent`) instead of the native JS setter, and focus via `DOM.focus` to prevent accessibility node ID invalidation (#161)

## [1.10.0] - 2026-03-12

### Added

- Add large-response detection with structured guidance object, `--search` filtering, and `--full-response` override for `page snapshot`, `page text`, `js exec`, `network list`, and `network get` (#168)

## [1.9.0] - 2026-03-12

### Added

- Add `skill` command group with `install`, `uninstall`, `update`, and `list` subcommands for installing agentchrome skill files into agentic coding tools (#172)

## [1.8.0] - 2026-03-12

### Added

- Add `page wait` subcommand for condition-based waiting with `--url` (glob), `--text`, `--selector`, and `--network-idle` conditions (#163)

## [1.7.0] - 2026-03-11

### Added

- Add `cookie` command group with `list`, `set`, `delete`, and `clear` subcommands for full cookie management via CDP (#164)

## [1.6.0] - 2026-03-11

### Added

- Add `page element` subcommand for targeted element state queries by UID or CSS selector (#165)

## [1.5.0] - 2026-02-27

### Added

- Add `form submit` subcommand for programmatic form submission via CDP (#147)

## [1.4.0] - 2026-02-26

### Added

- Add `--wait-until` flag to `interact click` and `interact click-at` commands for SPA-aware waiting after clicks (#148)

## [1.3.0] - 2026-02-20

### Changed

- Rename project from chrome-cli to AgentChrome — binary, crate, and all references (#155)
- Refocus README on agentic use and Claude Code integration

### Fixed

- Navigate back/forward/reload commands now respect the global `--timeout` flag and `AGENTCHROME_TIMEOUT` environment variable instead of always using the hardcoded 30-second default (#145)
- Navigate back/forward timeout on SPA same-document history navigations — listen for both `Page.frameNavigated` and `Page.navigatedWithinDocument` CDP events using `tokio::select!` (#144)

## [1.2.0] - 2026-02-19

### Added

- DOM command group with 13 subcommands for element queries and manipulation: select (CSS/XPath), get-attribute, get-text, get-html, set-attribute, set-text, remove, get-style, set-style, parent, children, siblings, and tree (#149)

## [1.1.0] - 2026-02-19

### Changed

- Replace page-reload strategy in `console read` with CDP replay buffer drain to capture runtime interaction messages across CLI invocations (#146)

## [1.0.5] - 2026-02-17

### Fixed

- Page commands targeting wrong tab after `tabs activate` — persist active tab ID in session file and prefer it in `resolve_target()` for cross-invocation state consistency (#137)

## [1.0.4] - 2026-02-17

### Fixed

- Tabs create --background not preventing tab activation — replace positional `/json/list` heuristic with CDP `document.visibilityState` queries and HTTP `/json/activate` for reliable active-tab detection (#133)

## [1.0.3] - 2026-02-17

### Fixed

- Page screenshot --uid failing with 'Could not find node with given id' regression — pass backendNodeId directly to DOM.getBoxModel (#132)

## [1.0.2] - 2026-02-17

### Fixed

- Form fill and clear not setting value on textarea elements due to incorrect native setter prototype selection (#136)

## [1.0.1] - 2026-02-17

### Fixed

- Dialog info returning wrong type (`"unknown"`) and empty message for open dialogs (#134)

## [1.0.0] - 2026-02-16

### Fixed

- Tabs activate not reflected in subsequent tabs list due to missing state propagation polling (#122)

## [0.1.8] - 2026-02-16

### Fixed

- Tabs create --background not keeping original tab active due to insufficient polling budget (#121)

## [0.1.7] - 2026-02-16

### Fixed

- Tabs close reporting incorrect remaining count due to off-by-one race condition (#120)

## [0.1.6] - 2026-02-16

### Fixed

- Perf vitals returning null for CLS and TTFB metrics (#119)

## [0.1.5] - 2026-02-16

### Fixed

- Perf record --duration reporting incorrect duration_ms (only measuring collection overhead instead of actual recording time) (#118)

## [0.1.4] - 2026-02-16

### Fixed

- Network list showing size 0 for most requests by falling back to content-length header (#117)

## [0.1.3] - 2026-02-16

### Fixed

- Network list timestamps showing 1970-01-01 instead of real wall-clock time (#116)

## [0.1.2] - 2026-02-16

### Fixed

- Page screenshot --uid failing with 'Could not find node with given id' (#115)

## [0.1.1] - 2026-02-16

### Fixed

- Connect --status ignoring --pretty and --plain output format flags (#114)

## [0.1.0] - 2026-02-16

### Added

- Cargo workspace setup with Rust 2024 edition (#1)
- Cross-platform release pipeline and BDD test harness (#2)
- CLI skeleton with clap derive macros and 12 subcommand stubs (#3)
- CDP WebSocket client with async transport and session multiplexing (#4)
- Chrome instance discovery and launch with connect subcommand (#5)
- Session and connection management with ManagedSession (#6)
- Tab management commands (#7)
- URL navigation and history commands (#8)
- Page text extraction command (#9)
- Accessibility tree snapshot (#10)
- Element finding by text, CSS selector, and accessibility attributes (#11)
- Screenshot capture — viewport, full-page, and element (#12)
- JavaScript execution in page context (#13)
- Mouse interactions — click, double-click, hover, drag (#14)
- Keyboard input — type and key commands (#15)
- Form input and filling (#16)
- Scroll interactions (#17)
- Console message reading with filtering (#18)
- Network request monitoring with filtering (#19)
- Browser dialog handling (#20)
- Device, network, and viewport emulation (#21)
- Performance tracing — start, stop, analyze (#22)
- File upload to page elements (#23)
- Configuration file support (#24)
- Shell completions generation for bash, zsh, fish, PowerShell, and elvish (#25)
- Comprehensive --help text for all commands, subcommands, and flags (#26)
- Man page generation via xtask (#27)
- Comprehensive README with quick-start, examples, and architecture overview (#28)
- Built-in examples subcommand (#29)
- Capabilities manifest subcommand (#30)
- Claude Code integration guide (#31)
- CI/CD workflows, dependabot config, and project specs

### Fixed

- Connect/launch timeout caused by EOF-based HTTP reading (#68)
- Missing --enable-automation flag in Chrome launch args (#70)
- Navigate back/forward cross-origin timeout (#72)
- Page snapshot empty accessibility tree on complex pages (#73)
- Emulate status inaccurate state due to CDP session isolation (#74)
- Perf vitals missing metrics serialized incorrectly (#75)
- Perf cross-invocation state loss due to session-scoped tracing (#76)
- Tabs create background flag ignored by Chrome (#82)
- Snapshot ignored nodes dropping children instead of promoting (#83)
- Form fill many JSON arg collision (#84)
- Emulate overrides persistence across CLI invocations (#85)
- Dialog commands timeout when dialog already open (#86)
- Connect auto-discover overwrites session PID on reconnect (#87)
- Connect auto-discover reconnect (#94)
- Tabs create background workaround (#95)
- JS exec double JSON stderr (#96)
- Page find role standalone (#97)
- Clap validation errors output JSON stderr with exit code 1 (#98)
- Dialog handle no-dialog-open (#99)
- Emulate reset viewport (#100)
- Disconnect process not killed (#101)
- Network list empty array (#102)
- Channel passing for linux and windows candidate functions
- Rustfmt and clippy components in rust-toolchain.toml

### Changed

- Prefix spec directories with issue numbers for organization
- Extract Page.enable timeout constant and dialog test helper
- Extract resolve_to_object_id helper in form.rs
- Add Chrome instance cleanup rule to steering docs
- Add off-limits files section to steering docs
- Bump actions/checkout from 4.3.1 to 6.0.2
- Bump actions/upload-artifact from 4.6.2 to 6.0.0
- Bump actions/download-artifact from 4.3.0 to 7.0.0
