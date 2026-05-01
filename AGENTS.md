# Agent Instructions

This project is managed with Codex. Follow the repository's existing Rust, BDD, and SDLC conventions.

## Project Context

- Product direction lives in `steering/product.md`.
- Technical standards live in `steering/tech.md`.
- Code organization guidance lives in `steering/structure.md`.
- Feature and bug specs live under `specs/`.
- Gherkin acceptance tests live under `tests/features/` and are bound in `tests/bdd.rs`.

## Browser Automation

Use AgentChrome itself for browser automation during manual verification:

```sh
agentchrome connect --launch --headless
agentchrome navigate http://localhost:3000 --wait-until networkidle
agentchrome page snapshot
agentchrome console read --errors-only
```

Always snapshot before interacting with UID-targeted elements, and prefer `--include-snapshot` after actions when the updated page state matters.

## Development

- Use `rg` for searching.
- Keep changes scoped to the requested issue or task.
- Preserve existing JSON output contracts and typed exit-code behavior.
- Run focused tests for touched behavior before finishing; use broader BDD runs when changes touch shared CLI behavior.

<!-- nmg-sdlc-managed: spec-context -->
## nmg-sdlc Spec Context

For SDLC work, project-root `specs/` is the canonical BDD archive. Always identify the active spec first, then use bounded relevant-spec discovery to load only the neighboring specs that can affect the change. Do not load the full archive by default, and do not use legacy `.codex/specs/` as context.
<!-- /nmg-sdlc-managed -->
