# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
