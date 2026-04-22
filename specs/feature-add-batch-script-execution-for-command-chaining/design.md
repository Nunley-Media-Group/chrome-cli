# Design: Batch Script Execution

**Issues**: #199
**Date**: 2026-04-21
**Status**: Draft
**Author**: Rich Nunley

---

## Overview

This design introduces a new `script` command group with a `run <file>` subcommand that reads a JSON script, dispatches each step against an existing CDP session, and emits a structured JSON result array on stdout. The runner lives in a new `src/script/` module and invokes existing command modules *as library functions* — it does not spawn sub-processes of `agentchrome`. This keeps per-step overhead below 5 ms, preserves the single-CDP-session invariant, and lets every existing command's structured output flow directly into `results[i].output`.

Control flow (`if`, `loop`) and variable references are part of the JSON script language, not part of clap. Expressions inside `if` / `while` are evaluated via the CDP `Runtime.evaluate` call the `js` module already uses, keeping the "no new host" constraint from requirements — `$prev` and `$vars` are injected as top-level bindings inside that evaluation scope.

Script v1 is sequential, single-session, JSON-only. The schema is open to additive fields (annotations, descriptions, tags) without breaking existing scripts. Streaming output and parallel execution are explicitly deferred.

---

## Architecture

### Component Diagram

```
┌──────────────────────────────────────────────────────────┐
│                      CLI Layer (cli/mod.rs)              │
│  Script(ScriptArgs)   →   ScriptSubcommand::Run(RunArgs) │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│              Dispatch (main.rs: dispatch_script)         │
│  Loads script → new ScriptRunner → runner.execute()      │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                  Script Runner (src/script/)             │
│  ┌──────────────┐  ┌─────────────┐  ┌────────────────┐   │
│  │  parser.rs   │  │  runner.rs  │  │  dispatch.rs   │   │
│  │ schema + JSON│  │ sequential  │  │ maps Step.cmd  │   │
│  │  validation  │  │ + if + loop │  │ to command mod │   │
│  └──────┬───────┘  └──────┬──────┘  └────────┬───────┘   │
│         │                 │                  │           │
│         └─────────────────▼──────────────────┘           │
│                       VarContext                         │
│           { prev: JsonValue, vars: HashMap }             │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│            Command Modules (navigate.rs, page/,          │
│            js.rs, form.rs, tabs.rs, interact.rs, …)      │
│   Called as library fns, not subprocess invocations      │
└───────────────────────────┬──────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────┐
│                CDP Client (src/cdp/)                     │
└──────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. clap parses `script run <file> [--fail-fast] [--dry-run]`
2. main dispatcher loads script bytes (file or stdin), hands them to script::parser
3. Parser validates v1 schema; returns Script { commands: Vec<Step> }
4. ScriptRunner walks commands sequentially:
     - for Cmd:   dispatch::invoke(step.cmd, &mut VarContext) → Result
     - for If:    evaluate expression via RuntimeEvaluator → Vec<Step> to execute
     - for Loop:  evaluate condition/count → iterate body, pushing loop_index
5. Each step yields a Result entry appended to the accumulator
6. Per-step: update prev; if bind present, store output under $vars.<name>
7. On error:  if --fail-fast → abort; else continue
8. On completion: serialize { results, executed, skipped, failed, total_ms }
9. Print JSON to stdout; exit 0 (unless --fail-fast path or connection error)
```

---

## API / Interface Changes

### New CLI Surface

| Command | Positional / Flag | Purpose |
|---------|-------------------|---------|
| `agentchrome script` | (subcommand) | New command group |
| `agentchrome script run <file>` | `<file>` positional | Path to JSON script; `-` reads from stdin |
| `agentchrome script run` | `--fail-fast` | Stop at first error; exit non-zero |
| `agentchrome script run` | `--dry-run` | Parse + validate only; no CDP dispatch |

`--json` / `--pretty` are inherited from GlobalOpts.

Clap shape (abbreviated):

```rust
#[derive(Subcommand)]
enum Command {
    // ... existing variants ...

    /// Execute a batch script of agentchrome commands
    #[command(
        long_about = "Execute a JSON batch script composed of agentchrome \
            commands, conditional branches, and loops. The script runs \
            sequentially against the active CDP session and emits a \
            structured JSON result array on stdout.",
        after_long_help = "EXAMPLES:\n  \
          # Run a script file\n  \
          agentchrome script run workflow.json\n\n  \
          # Read a script from stdin\n  \
          echo '{\"commands\":[{\"cmd\":[\"navigate\",\"https://example.com\"]}]}' | agentchrome script run -\n\n  \
          # Stop at the first failure\n  \
          agentchrome script run --fail-fast workflow.json\n\n  \
          # Validate without dispatching\n  \
          agentchrome script run --dry-run workflow.json"
    )]
    Script(ScriptArgs),
}

#[derive(Args)]
struct ScriptArgs {
    #[command(subcommand)]
    sub: ScriptSubcommand,
}

#[derive(Subcommand)]
enum ScriptSubcommand {
    /// Run a JSON script
    Run(RunArgs),
}

#[derive(Args)]
struct RunArgs {
    /// Path to a JSON script file (`-` reads from stdin)
    file: String,

    /// Stop at the first failing step and exit non-zero
    #[arg(long)]
    fail_fast: bool,

    /// Validate the script without dispatching any command
    #[arg(long)]
    dry_run: bool,
}
```

### Script v1 JSON Schema (abbreviated)

```json
{
  "commands": [
    { "cmd": ["navigate", "https://example.com"] },
    { "cmd": ["js", "exec", "document.title"], "bind": "title" },
    {
      "if": "$vars.title.includes('Example')",
      "then": [{ "cmd": ["page", "screenshot", "--file", "ok.png"] }],
      "else": [{ "cmd": ["page", "screenshot", "--file", "fail.png"] }]
    },
    {
      "loop": { "count": 3 },
      "body": [
        { "cmd": ["interact", "click-at", "--x", "100", "--y", "200"] }
      ]
    }
  ]
}
```

### Result JSON Shape

```json
{
  "results": [
    { "index": 0, "command": ["navigate", "https://example.com"], "status": "ok", "output": { "url": "https://example.com" }, "duration_ms": 124 },
    { "index": 1, "command": ["js", "exec", "document.title"], "status": "ok", "output": { "result": "Example Domain" }, "duration_ms": 11 }
  ],
  "executed": 2,
  "skipped": 0,
  "failed": 0,
  "total_ms": 138
}
```

### Error Shape (stderr on --fail-fast abort)

```json
{
  "error": "script step 2 failed: Chrome CDP returned no result",
  "code": 1,
  "failing_index": 2,
  "failing_command": ["js", "exec", "$vars.missing"]
}
```

---

## Database / Storage Changes

None. Script runner is stateless; all state lives in process memory for the duration of the run.

---

## State Management

### In-process `VarContext`

```rust
struct VarContext {
    prev: serde_json::Value,          // last non-skipped step output
    vars: std::collections::HashMap<String, serde_json::Value>,
    cwd_script: std::path::PathBuf,   // for future relative-path resolution
}
```

### Argument Substitution

When a `cmd` argv contains a token shaped `$vars.<name>` or `$prev`:

- Whole-token match → replace the argv element with the bound value serialized appropriately (JSON strings unwrap to their Rust `String`; non-strings serialize to JSON).
- Inline interpolation (`"hello $vars.name"`) → fall back to Chrome-side evaluation of the token as an expression in a `Runtime.evaluate` call.
- Unknown variable → step-level error; under `--fail-fast` aborts execution.

### Expression Evaluation

`if` / `while` expressions execute through `js::evaluate_expression(session, expr, &VarContext)`:

1. Prefix the expression with a bound preamble: `const $prev = <json>; const $vars = <json>; const $i = <loop-index>;`
2. Call CDP `Runtime.evaluate` with `returnByValue: true` and `throwOnSideEffect: false`.
3. Coerce the result to boolean for conditionals; keep the raw JSON value if needed later.
4. On evaluation exception → step-level error (`status: "error"`).

---

## UI Components

N/A — CLI only.

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| **A: Shell out to `agentchrome` per step** | Fork + exec the binary for each step | Trivial to implement | Loses the < 50 ms startup budget per step; kills the performance win | Rejected |
| **B: Embed a full JS engine (QuickJS / Deno)** | Run scripts in a dedicated JS host | Powerful scripting | New dependency, security surface, binary bloat (>10 MB constraint) | Rejected |
| **C: JSON DSL + CDP `Runtime.evaluate` for expressions** | Declarative JSON, Chrome evaluates the few needed expressions | No new host; uses existing CDP path; small schema | JS expressions require an active session; dry-run can't fully evaluate expressions | **Selected** |
| **D: Ship a YAML + JSON dual format now** | Accept both | Slightly friendlier authoring | Doubles parsing surface; issue scope says JSON only in v1 | Deferred |

---

## Security Considerations

- [x] **No new code execution host.** `if` / `while` expressions run in the browser via existing `Runtime.evaluate` — they cannot touch the local filesystem, environment, or agentchrome internals.
- [x] **No credential material in scripts.** Script files are plain JSON; users should treat them as code artifacts. README and `examples script` will call this out.
- [x] **Argument substitution sanitization.** Whole-token substitution replaces argv elements with serialized JSON; the substituted string is not re-parsed as shell.
- [x] **File read limited to the declared path.** Stdin mode (`-`) reads the process stdin only. No path traversal, no URL fetch.

---

## Performance Considerations

- [x] **Per-step overhead < 5 ms.** Runner holds a single session handle; no re-connection per step.
- [x] **No allocation-heavy result copying.** `results[].output` takes ownership of the command's JSON value; no deep clones in the happy path.
- [x] **`--dry-run` avoids any CDP round trip.** Parsing + schema validation + subcommand-name lookup only.
- [x] **Loop `max` guard** prevents accidental infinite loops (AC7 warning).

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| Script parser | Unit | Schema validation, error messages for malformed steps (nested `if` / `loop`, missing fields, bad types) |
| Argument substitution | Unit | Whole-token and inline cases; unknown variable path |
| ScriptRunner dispatch | Unit | Dispatch table round-trip (`cmd` → library fn) with a stub command registry |
| Runner control flow | Unit | Count loop, while + max, if/then/else selection |
| BDD | Integration | AC1–AC16 scenarios in `tests/features/batch-script-execution.feature` |
| Smoke test | Manual | Real headless Chrome; exercises happy path, fail-fast, loop, stdin |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Command modules today take `&GlobalOpts` + CLI args; making them callable from the runner requires refactoring | High | Med | Introduce a thin `CommandEntry` dispatch table per module exposing a `fn run(ctx, argv) -> Result<Value>` adapter. Refactor lazily; start with the 8 commands used in examples (navigate, page, js, form, interact, tabs, console, dialog). |
| Cross-invocation state regressions (retrospective learning) | Med | High | Add AC16 (state propagation across steps) and smoke-test both headed and headless runtimes. |
| Script runs that create headed Chrome leak processes | Low | Med | v1 forbids `connect` / `disconnect` inside scripts; session is external. Smoke-test cleanup per steering `tech.md`. |
| Expression evaluation leaking internal state to the page | Low | Med | Use `contextId` tied to the active page; document that expressions run in page context. |
| Loop semantics inconsistent across nested loops (`$i`) | Med | Low | Spec-level: `$i` refers to the innermost loop; outer via explicit `bind` if needed. Unit-test nested loops. |

---

## Open Questions

- [ ] Does the runner serialize each step's output via `serde_json::Value`, or keep a type-erased handle to avoid re-serialization for the response? (Leaning `Value` for simplicity.)
- [ ] Should `--dry-run` still require an active session (for capability lookup) or stay fully offline? Design currently picks "fully offline."

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #199 | 2026-04-21 | Initial feature spec |

---

## Validation Checklist

- [x] Architecture follows existing project patterns (`src/<cmd>.rs` / `src/<cmd>/` modules; clap derive in `cli/mod.rs`)
- [x] API/interface changes documented with clap shape and result schemas
- [x] No database/storage changes required
- [x] State management approach is clear (`VarContext` + CDP `Runtime.evaluate`)
- [x] Security considerations addressed
- [x] Performance impact analyzed (< 5 ms per-step overhead)
- [x] Testing strategy defined (unit + BDD + manual smoke)
- [x] Alternatives considered and documented
- [x] Risks identified with mitigations
