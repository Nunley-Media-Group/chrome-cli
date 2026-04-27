# Design: Clean HTML-to-Markdown Conversion for Agentic Scraping

**Issues**: #269
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (write-spec)

---

## Overview

Add a top-level `agentchrome markdown` command that converts the current browser page or one raw HTML source into cleaned Markdown. The command uses current AgentChrome conventions: clap-derived CLI metadata, structured JSON stdout by default, global `--plain` support, one structured JSON error on stderr, typed exit codes, global target flags for browser sources, and shared large-response behavior.

The implementation should not mutate browser state. Browser-page mode reads `document.documentElement.outerHTML`, `document.baseURI`, `location.href`, and `document.title` from the selected tab or page target. Raw modes read file, stdin, or URL bytes without executing scripts. All sources then flow through the same cleanup and conversion pipeline so behavior is consistent across page, file, stdin, and URL inputs.

---

## Architecture

### Component Diagram

```text
CLI Layer
  src/cli/mod.rs
    Command::Markdown(MarkdownArgs)
        |
        v
Dispatch Layer
  src/main.rs
    markdown::execute_markdown(global, args)
        |
        v
Source Acquisition
  src/markdown.rs
    page source -> CDP Runtime.evaluate
    file source -> std::fs
    stdin source -> std::io
    URL source -> bounded HTTP fetch
        |
        v
Cleanup + Conversion
  src/markdown.rs
    parse HTML -> scope selector -> remove noise
    choose primary region -> normalize links/images
    convert HTML tree to Markdown
        |
        v
Output
  src/output.rs
    emit JSON or plain Markdown through large-response gate
```

### Data Flow

```text
1. User runs agentchrome markdown [source option] [conversion options].
2. CLI validates mutually exclusive source options and bounded numeric flags.
3. Source acquisition returns SourceDocument { html, source, base_url, title }.
4. Converter parses HTML into a tree.
5. If --selector is present, matching subtrees become the conversion root; no matches fail.
6. If --selector is absent, primary-content heuristics select main, role=main, article, or body.
7. Cleanup removes non-content nodes and unwraps layout-only containers/tables.
8. Link and image options normalize anchors/images before conversion.
9. Markdown converter renders cleaned HTML to Markdown.
10. Result is emitted as JSON or plain text using shared output helpers.
```

---

## API / Interface Changes

### New Command

| Command | Type | Purpose |
|---------|------|---------|
| `agentchrome markdown` | Top-level CLI command | Convert the current page or raw HTML source into cleaned Markdown |

### CLI Arguments

| Argument | Type | Purpose |
|----------|------|---------|
| `--file <PATH>` | Option<PathBuf> | Read raw HTML from a local file |
| `--stdin` | bool | Read raw HTML from stdin |
| `--url <URL>` | Option<String> | Fetch raw HTML from an HTTP/HTTPS URL |
| `--base-url <URL>` | Option<String> | Base URL for resolving relative links in file/stdin input |
| `--selector <CSS>` | Option<String> | Scope conversion to matching CSS subtrees |
| `--strip-links` | bool | Keep link text but remove link destinations |
| `--include-images` | bool | Preserve useful images as Markdown image references |
| `--max-input-bytes <N>` | Option<usize> | Override the default raw-input byte limit |

Source options `--file`, `--stdin`, and `--url` must be mutually exclusive. When none is supplied, the command uses the current browser page. Browser-page mode uses existing global flags such as `--tab`, `--page-id`, `--timeout`, `--plain`, `--pretty`, and `--large-response-threshold`.

### Request / Response Schemas

#### Success JSON

```json
{
  "markdown": "# Example Article\n\nBody text...",
  "source": {
    "kind": "page",
    "url": "https://example.test/article",
    "title": "Example Article",
    "path": null,
    "selector": null
  },
  "metadata": {
    "input_bytes": 18421,
    "markdown_bytes": 982,
    "removed_node_count": 17,
    "primary_region": "article",
    "links_preserved": true,
    "images_included": false
  }
}
```

#### Plain Success

```text
# Example Article

Body text...
```

#### Error JSON

```json
{
  "error": "selector '#missing' did not match any nodes",
  "code": 3
}
```

### Error Mapping

| Condition | Exit Code |
|-----------|-----------|
| Invalid source option combination, invalid base URL, invalid input limit | 1 GeneralError |
| File missing, unreadable file, stdin read failure, input over `--max-input-bytes` | 1 GeneralError |
| Browser connection or target resolution failure | Existing connection/target codes from shared helpers |
| URL DNS/TLS/connect failure | 2 ConnectionError |
| Selector matches no nodes | 3 TargetError |
| Browser evaluation, HTML parsing, or conversion failure | 5 ProtocolError for CDP-origin failures; 1 GeneralError for local conversion failures |
| URL fetch timeout | 4 TimeoutError |

---

## Component Design

### `src/cli/mod.rs`

Add:

```rust
Markdown(MarkdownArgs)
```

`MarkdownArgs` owns source and conversion flags. It must include clap `long_about`, `after_long_help`, doc comments, mutual exclusions, value parsers, and examples that cover all required invocation modes.

### `src/main.rs`

Add:

```rust
mod markdown;
```

Dispatch `Command::Markdown(args)` to `markdown::execute_markdown(&global, args).await`.

### `src/markdown.rs`

Create a command module with these internal concepts:

```rust
struct MarkdownArgsResolved { ... }
struct SourceDocument { html: String, source: SourceInfo, base_url: Option<Url>, title: Option<String> }
struct SourceInfo { kind: SourceKind, url: Option<String>, title: Option<String>, path: Option<String>, selector: Option<String> }
struct MarkdownMetadata { input_bytes: usize, markdown_bytes: usize, removed_node_count: usize, primary_region: Option<String>, links_preserved: bool, images_included: bool }
struct MarkdownResult { markdown: String, source: SourceInfo, metadata: MarkdownMetadata }
```

Suggested functions:

| Function | Responsibility |
|----------|----------------|
| `execute_markdown` | Resolve source, run conversion, emit output |
| `read_page_source` | Use CDP `Runtime.evaluate` to return page HTML, URL, base URL, title |
| `read_file_source` | Read bounded file bytes |
| `read_stdin_source` | Read bounded stdin bytes |
| `fetch_url_source` | Fetch bounded URL bytes with timeout without blocking the async runtime directly |
| `convert_clean_markdown` | Parse, clean, normalize, and convert HTML |
| `select_scope` | Apply `--selector` or primary-region heuristics |
| `remove_noise_nodes` | Remove scripts, styles, hidden nodes, landmarks, ads, cookie banners, and related boilerplate |
| `normalize_links_and_images` | Resolve URLs, unwrap links when requested, remove/include images |
| `summarize_result` | Build large-response summary metadata |

### HTML Parsing and Markdown Conversion

Use permissively licensed Rust dependencies. A viable baseline is:

| Crate | Purpose | Notes |
|-------|---------|-------|
| `kuchiki` | Parse and manipulate HTML trees with CSS selector support | MIT licensed; suitable for cleanup and scoping |
| `quick_html2md` | Convert cleaned HTML to GitHub-flavored Markdown | MIT OR Apache-2.0 licensed; supports GFM conversion |
| `ureq` | Lightweight HTTP/HTTPS client for URL mode | MIT OR Apache-2.0 licensed; run in `spawn_blocking` or equivalent |

Do not use GPL-only converter crates. If implementation exploration finds the selected Markdown converter cannot preserve a required structure, wrap it with targeted preprocessing/postprocessing rather than weakening the acceptance criteria.

---

## Cleanup Rules

### Hard Removals

Remove these elements wherever they appear:

- `script`, `style`, `noscript`, `head`, `template`, `svg`, `canvas`
- Elements with `hidden`
- Elements with `aria-hidden="true"`
- Elements with inline `display:none` or `visibility:hidden`

### Boilerplate Removals

When the element is outside an explicitly selected content scope, remove common page chrome:

- `header`, `footer`, `nav`, `aside`
- `[role="banner"]`, `[role="navigation"]`, `[role="contentinfo"]`, `[role="search"]`, `[role="complementary"]`
- Elements whose `id`, `class`, `aria-label`, or `data-*` naming clearly indicates cookie/consent banners, advertisements, promo blocks, share widgets, social widgets, newsletter forms, skip links, or sidebar-only content

Inside an explicit selector scope, still remove hard removals and obvious cookie/ad/share noise, but do not apply primary-region narrowing.

### Primary Region Selection

When `--selector` is absent:

1. Score candidates in this order: `main`, `[role="main"]`, `article`, then `body`.
2. Prefer candidates with meaningful text length, heading density, paragraph/list content, and low link-density.
3. If multiple candidates score similarly, choose document-order first for determinism.
4. Fall back to `body` when no candidate has enough content.

### Tables

Content tables are preserved as Markdown tables where headers or multiple rows/columns indicate tabular data. Layout-only tables are unwrapped into paragraph/list text without losing visible text.

### Code Blocks

Preserve fenced code blocks. Language hints should be detected from classes like `language-rust`, `lang-rust`, `highlight-source-rust`, or `data-language="rust"` when present.

---

## Security Considerations

- URL mode supports only `http` and `https`.
- URL mode does not send browser cookies, session data, or custom credentials.
- Raw HTML sources are parsed as data; scripts are not executed.
- Input size is bounded before parsing.
- Relative URL resolution must not turn invalid URLs into misleading links; invalid relative URLs should be left as text or omitted according to the link option.

---

## Performance Considerations

- Read raw sources up to the configured byte limit.
- Avoid repeatedly serializing the full HTML tree in cleanup loops.
- Use shared `output::emit` and `output::emit_plain` for generated output.
- URL fetching must respect `--timeout` and should use `spawn_blocking` if a blocking HTTP client is selected.
- Unit tests should cover large fixture cleanup without requiring a live network.

---

## Testing Strategy

| Layer | Type | Coverage |
|-------|------|----------|
| CLI parsing | Unit / clap assertions | Source mutual exclusion, `--max-input-bytes`, help text |
| Source acquisition | Unit | file/stdin limits, URL error mapping, browser source script output parsing |
| Cleanup pipeline | Unit | hard removals, boilerplate removals, primary-region scoring, selector scoping |
| Markdown conversion | Unit | links, images, code fences, content tables, layout tables |
| Output | Unit / BDD | JSON schema, null optional fields, plain output, large-response offload |
| Feature | BDD | One scenario per acceptance criterion in `tests/features/clean-html-markdown.feature` |
| Manual smoke | Verification task | Real headless Chrome fixture conversion with `agentchrome markdown` |

---

## Alternatives Considered

| Option | Description | Pros | Cons | Decision |
|--------|-------------|------|------|----------|
| `page markdown` | Add a new `page` subcommand | Fits current-page mode and `--frame` precedent | Raw file/stdin/URL sources do not naturally belong under `page` | Rejected |
| `dom get-html --markdown` | Extend targeted DOM HTML extraction | Reuses existing DOM command | Does not cover full-page primary extraction or raw input sources cleanly | Rejected |
| `scrape markdown` | Add a scrape command group | Names the agentic scraping use case | Implies crawling/site scraping, which is explicitly out of scope | Rejected |
| Top-level `markdown` | Single command with mutually exclusive source options | Covers page, file, stdin, and URL with one discoverable surface | Adds a new top-level command | Selected |
| Copy the referenced gist | Port prior-art script directly | Fast conceptual start | Not Rust-native, not AgentChrome output/error style, and may import unreviewed behavior | Rejected |
| GPL-only `html2md` crate | Use existing converter crate | Existing conversion implementation | GPL-3.0+ is not compatible with this project's licensing posture | Rejected |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cleanup heuristics remove useful content | Medium | High | Keep rules deterministic, test primary/selector paths, and prefer selector override when precision matters |
| Markdown converter drops code language hints | Medium | Medium | Add explicit unit and BDD coverage; preprocess code nodes if the converter does not preserve hints |
| URL input increases binary size | Medium | Medium | Use a lightweight client, measure build impact, and avoid heavy optional features |
| Output shape drifts from AgentChrome conventions | Low | High | Route through shared output/error helpers and add BDD output-contract scenarios |
| Large input causes high memory usage | Medium | High | Enforce `--max-input-bytes` before parse and large-response output after conversion |

---

## Open Questions

- None. If implementation discovers a selected dependency cannot satisfy the required behavior, preserve the public contract and swap the internal implementation with an equally permissive alternative.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #269 | 2026-04-27 | Initial feature spec |
