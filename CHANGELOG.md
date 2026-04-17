# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
