# Contributing

## Project Context

AgentChrome is a Rust CLI for browser automation through the Chrome DevTools Protocol. It is built for AI agents and developers who need deterministic, scriptable browser control with JSON output, accessibility-tree targeting, and typed exit codes.

Before contributing, read:

- `steering/product.md` for product priorities and user journeys.
- `steering/tech.md` for Rust, CLI, testing, versioning, and verification standards.
- `steering/structure.md` for module boundaries, naming, and repository layout.
- Existing specs under `specs/` for behavior already designed or repaired.

## Issue and Spec Workflow

Start work from a clear GitHub issue with acceptance criteria. Feature and bug work should flow through nmg-sdlc specs in `specs/`, using the issue -> spec -> code -> simplify -> verify -> PR lifecycle.

Feature specs use plural `**Issues**` frontmatter because a feature can accumulate multiple contributing issues. Defect specs use singular `**Issue**` and should point to the related feature spec when one exists.

Existing code and reconciled specs are required context for this brownfield project. Do not draft or implement a change from the issue text alone when surrounding specs, steering, or current CLI behavior define stronger constraints.

## Steering Expectations

AgentChrome contributions must preserve the product's core contract:

- CLI-first, non-interactive behavior suitable for shell pipelines and AI agents.
- Structured JSON on stdout and structured JSON errors on stderr.
- Meaningful typed exit codes.
- Accessibility-tree and UID-driven interaction semantics for agent workflows.
- Cross-platform Chrome discovery and launch behavior.

Technical work should follow the Rust 2024 workspace conventions, strict formatting and clippy expectations, and the module boundaries described in `steering/structure.md`.

## Implementation and Verification

Keep changes scoped to the issue and its spec. Preserve existing JSON output contracts, clap help metadata, generated man page behavior, BDD coverage, and versioning rules unless the issue explicitly changes them.

Run focused verification for touched behavior before opening a PR. For shared CLI behavior, use broader BDD coverage. At minimum, respect the project verification gates in `steering/tech.md`, including `cargo fmt --check`, clippy, focused Rust tests, BDD scenarios, and man page generation when CLI help changes.

## PR Readiness and Contribution Gate

Before requesting review, make the PR body useful to both reviewers and the managed nmg-sdlc contribution gate:

- Link the GitHub issue with `Closes #N` or equivalent issue evidence.
- Link or summarize the relevant `specs/feature-*` or `specs/bug-*` artifacts.
- State how the change aligns with `steering/product.md`, `steering/tech.md`, and `steering/structure.md`.
- Include verification evidence: commands run, focused BDD scenarios, `$nmg-sdlc:verify-code` results, or a committed `verification-report.md`.
- Call out known gaps, skipped checks, or reviewer context before marking the PR ready.

If the nmg-sdlc contribution gate fails, fix the missing evidence category named in the workflow output: issue linkage, spec linkage, steering evidence, verification evidence, or guide discoverability. Do not bypass the gate; update the PR body or the relevant artifacts so the review record stays complete.
