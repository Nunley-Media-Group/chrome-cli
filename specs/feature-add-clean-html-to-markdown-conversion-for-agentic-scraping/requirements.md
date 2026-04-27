# Requirements: Clean HTML-to-Markdown Conversion for Agentic Scraping

**Issues**: #269
**Date**: 2026-04-27
**Status**: Draft
**Author**: Codex (write-spec)

---

## User Story

**As an** AI agent or automation engineer
**I want** AgentChrome to convert noisy browser-page HTML and raw HTML inputs into cleaned Markdown
**So that** scraping and research workflows consume less context and produce clearer downstream reasoning

---

## Background

AgentChrome already exposes several adjacent inspection surfaces: `page text` returns visible text, `page snapshot --compact` returns a token-efficient accessibility tree, and `dom get-html` returns raw `outerHTML` for a targeted element. Agentic scraping often needs a middle path: structure richer than plain text, but much less noise than raw HTML.

This feature adds a standard cleanup pipeline that removes page chrome and boilerplate, prefers primary content regions, preserves useful document structure, and emits Markdown through AgentChrome's existing output contracts. The issue references an external converter gist as prior art, but the product behavior is specified here rather than copied from that script.

---

## Acceptance Criteria

**IMPORTANT: Each criterion becomes a Gherkin BDD test scenario.**

### AC1: Convert the current browser page to cleaned Markdown

**Given** AgentChrome is connected to a browser page containing primary article content plus navigation, header, footer, cookie banner, scripts, styles, SVG, hidden elements, and sidebar content
**When** I run `agentchrome markdown`
**Then** stdout contains structured JSON with `markdown`, `source`, and `metadata` fields
**And** `source.kind` is `page`
**And** the Markdown preserves meaningful headings, paragraphs, lists, links, and code blocks
**And** the Markdown excludes boilerplate and non-content noise
**And** the command respects the global `--tab` and `--page-id` target-selection flags

### AC2: Convert raw HTML from file, stdin, and URL inputs

**Given** equivalent HTML is available from a local file, stdin, and an HTTPS URL
**When** I run `agentchrome markdown --file <path>`, `agentchrome markdown --stdin`, and `agentchrome markdown --url <url>`
**Then** each invocation emits cleaned Markdown using the same cleanup rules
**And** each output identifies the input source in `source.kind`
**And** file and stdin sources use `--base-url` when provided to resolve relative URLs
**And** URL fetching respects the command timeout
**And** URL fetch failures emit a structured JSON error on stderr with a non-zero exit code

### AC3: Prefer primary content containers when present

**Given** HTML contains a substantive `<main>`, `[role="main"]`, or `<article>` region along with surrounding site chrome
**When** I run `agentchrome markdown`
**Then** the output focuses on the highest-value primary content region
**And** surrounding navigation, header, footer, search, complementary, and contentinfo regions are omitted
**And** the same primary-content selection is applied to browser-page, file, stdin, and URL sources

### AC4: Scope extraction explicitly when requested

**Given** a browser page or raw HTML input contains multiple content regions
**When** I run `agentchrome markdown --selector "<css-selector>"`
**Then** the output includes only the matched subtree or subtrees and their descendants
**And** normal cleanup still removes scripts, styles, hidden nodes, and obvious boilerplate inside the selected scope
**And** a selector that matches no nodes returns exactly one structured JSON error on stderr with a non-zero exit code

### AC5: Control links and images deterministically

**Given** HTML contains relative links, absolute links, internal anchors, and images
**When** I run `agentchrome markdown` with default options
**Then** useful hyperlinks are preserved as Markdown links
**And** relative URLs are resolved against the page URL, fetched URL, or supplied `--base-url`
**And** images are omitted by default to keep the output concise
**When** I run `agentchrome markdown --strip-links`
**Then** link text remains while link destinations are removed
**When** I run `agentchrome markdown --include-images`
**Then** useful images are preserved as Markdown image references with resolved URLs and alt text when available

### AC6: Preserve code and readable document structure

**Given** HTML contains preformatted code with language hints, headings, ordered and unordered lists, blockquotes, horizontal separators, content tables, and layout-only tables
**When** I run `agentchrome markdown`
**Then** code blocks preserve language annotations when the source exposes a language hint
**And** headings, paragraphs, lists, blockquotes, separators, and content tables remain readable in Markdown
**And** layout-only tables are unwrapped or simplified without losing text content

### AC7: Keep AgentChrome output and error contracts

**Given** conversion succeeds or fails from any supported source
**When** the command exits
**Then** success output follows AgentChrome's structured JSON stdout conventions
**And** `--plain` emits only the Markdown body when the body is within the large-response threshold
**And** large JSON or plain output uses the existing large-response temp-file gate
**And** failures emit exactly one structured JSON error on stderr
**And** exit codes remain consistent with existing typed error behavior
**And** optional output fields that cannot be determined are present as `null`, not silently omitted

### AC8: Document and expose the new CLI surface

**Given** the new command is installed
**When** I run `agentchrome markdown --help`
**Then** help text describes the command and its canonical invocation shape
**And** long help includes realistic examples for current-page, file, stdin, URL, `--plain`, selector, link-stripping, and image-inclusion modes
**When** I run `agentchrome capabilities` or generate man pages
**Then** the new command, source flags, output flags, and examples are discoverable from those surfaces

### AC9: Bound raw input and generated output

**Given** an input source is very large or produces very large Markdown
**When** I run `agentchrome markdown`
**Then** raw file, stdin, and URL inputs are rejected once they exceed the documented input-size limit unless the user raises that limit
**And** oversized Markdown output is offloaded through the existing large-response temp-file behavior
**And** the JSON summary reports enough metadata for an agent to decide whether to read the temp file

---

## Functional Requirements

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| FR1 | Provide a CLI-accessible clean HTML-to-Markdown conversion command at `agentchrome markdown`. | Must | Single command covers browser and raw sources. |
| FR2 | Use the current browser page as the default source when no raw input source flag is provided. | Must | Reuses global connection, `--tab`, and `--page-id` behavior. |
| FR3 | Support raw HTML sources from `--file`, `--stdin`, and `--url`; these source selectors must be mutually exclusive. | Must | URL mode supports `http` and `https` only. |
| FR4 | Support `--selector` to scope conversion to matching CSS subtrees and fail cleanly when nothing matches. | Must | Applies to all source types. |
| FR5 | Remove non-content elements including `script`, `style`, `head`, `template`, `svg`, hidden nodes, skip links, share/social widgets, cookie/consent banners, and obvious advertisement/promo containers. | Must | Cleanup must be deterministic. |
| FR6 | Prefer substantive primary containers such as `main`, `[role="main"]`, and `article` when no explicit selector is provided. | Must | Highest-value region is selected by structural and text-density heuristics. |
| FR7 | Preserve useful document structure in Markdown, including headings, paragraphs, lists, links, blockquotes, separators, content tables, and code blocks. | Must | Code-fence language hints are required when available. |
| FR8 | Preserve links by default, support `--strip-links`, omit images by default, and support `--include-images`. | Must | Relative URLs resolve against source base URL. |
| FR9 | Emit default JSON containing `markdown`, `source`, and `metadata`, and support global `--plain`, `--json`, `--pretty`, and `--large-response-threshold`. | Must | Uses shared output helpers. |
| FR10 | Bound raw input size with `--max-input-bytes` and reject over-limit sources with one structured stderr JSON error. | Should | Prevents accidental context and memory blowups. |
| FR11 | Add clap help metadata, examples, capabilities coverage, and generated man-page coverage for the new command and flags. | Should | Required by `steering/tech.md`. |

---

## Non-Functional Requirements

| Aspect | Requirement |
|--------|-------------|
| **Performance** | Typical single-page conversion should complete within the global command timeout; URL fetches must honor timeout and should not block the async runtime directly. |
| **Security** | URL mode fetches only `http` and `https`, does not execute scripts, does not send browser cookies, and does not crawl linked pages. |
| **Reliability** | Disconnected Chrome, missing selectors, unreadable files, invalid URLs, timeout, and oversized input all return one JSON error on stderr with a typed exit code. |
| **Platforms** | macOS, Linux, and Windows, matching AgentChrome's baseline. |
| **Output** | Data goes to stdout; errors go to stderr; large responses use existing temp-file output semantics. |
| **Licensing** | Any new dependencies must be compatible with AgentChrome's MIT OR Apache-2.0 licensing. GPL-only converter crates are not acceptable. |

---

## Data Requirements

### Input Data

| Field | Type | Validation | Required |
|-------|------|------------|----------|
| `--file` | Path | Must exist, be readable, and be mutually exclusive with `--stdin` and `--url` | No |
| `--stdin` | Boolean flag | Reads all stdin up to `--max-input-bytes`; mutually exclusive with `--file` and `--url` | No |
| `--url` | URL string | Must parse as `http` or `https`; fetch must complete within timeout and input limit | No |
| `--base-url` | URL string | Must parse as absolute URL; used for file/stdin relative link resolution | No |
| `--selector` | CSS selector string | Must parse as selector and match at least one node | No |
| `--strip-links` | Boolean flag | Conflicts with no other markdown option | No |
| `--include-images` | Boolean flag | Includes useful image references when true | No |
| `--max-input-bytes` | Positive integer | Must be greater than zero | No |

If none of `--file`, `--stdin`, or `--url` is supplied, the source is the current browser page.

### Output Data (JSON mode)

| Field | Type | Description |
|-------|------|-------------|
| `markdown` | String | Cleaned Markdown body, unless offloaded by the large-response gate |
| `source.kind` | String enum | `page`, `file`, `stdin`, or `url` |
| `source.url` | String or null | Browser page URL, fetched URL, or base URL when known |
| `source.title` | String or null | Browser page title or discovered document title when known |
| `source.path` | String or null | File path for file input; null otherwise |
| `source.selector` | String or null | Selector scope used for extraction |
| `metadata.input_bytes` | Integer | Raw HTML bytes consumed |
| `metadata.markdown_bytes` | Integer | Markdown bytes produced before large-response offload |
| `metadata.removed_node_count` | Integer | Count of removed nodes where determinable |
| `metadata.primary_region` | String or null | Selected region type, such as `main`, `article`, or `selector` |
| `metadata.links_preserved` | Boolean | False when `--strip-links` is used |
| `metadata.images_included` | Boolean | True when `--include-images` is used |

### Output Data (Plain mode)

Raw Markdown body to stdout when the body is within the configured large-response threshold. If it exceeds the threshold, use the existing plain-output temp-file behavior.

---

## Dependencies

### Internal Dependencies

- `src/cli/mod.rs` for clap command and flag definitions
- `src/main.rs` for dispatch
- `src/output.rs` for structured output and large-response gating
- `src/error.rs` for typed errors and JSON stderr
- `src/page/text.rs`, `src/dom.rs`, and `src/page/mod.rs` for adjacent source-acquisition and selector patterns
- `tests/bdd.rs` and `tests/features/` for executable BDD coverage

### External Dependencies

- A permissively licensed HTML parser/tree manipulation crate for cleanup
- A permissively licensed HTML-to-Markdown converter crate for baseline Markdown rendering
- A lightweight HTTP client for URL input, or an equivalent async-safe fetch implementation

### Blocked By

- None.

---

## Out of Scope

- Web crawling across multiple pages or recursively following links
- Site-specific extraction recipes or per-domain heuristics
- Browser interaction planning or content summarization
- Replacing `page text`, `page snapshot`, or `dom get-html`
- Firefox, Safari, or non-CDP browser engines
- Executing JavaScript from raw HTML file/stdin/URL sources
- Preserving pixel-perfect page layout in Markdown

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Boilerplate reduction | Fixture navigation, header, footer, cookie, script, style, SVG, hidden, and sidebar content absent from Markdown | BDD assertions against `tests/fixtures/clean-html-markdown.html` |
| Structure preservation | Headings, paragraphs, lists, links, code fences, and content tables are present and readable | BDD and unit assertions |
| Output contract | One JSON stdout object on success; one JSON stderr object on failure | BDD assertions for success and error paths |
| Cross-source consistency | File, stdin, URL, and page sources produce semantically equivalent Markdown for the same HTML | BDD scenario comparing normalized Markdown |

---

## Open Questions

- None from the issue body. Unattended mode skipped interactive gap detection, so implementation should preserve the behavior specified here unless a concrete blocker appears.

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #269 | 2026-04-27 | Initial feature spec |

---

## Validation Checklist

Before moving to PLAN phase:

- [x] User story follows "As a / I want / So that" format
- [x] All acceptance criteria use Given/When/Then format
- [x] No implementation details in requirements beyond public CLI contract
- [x] All criteria are testable and unambiguous
- [x] Success metrics are measurable
- [x] Edge cases and error states are specified
- [x] Dependencies are identified
- [x] Out of scope is defined
- [x] Open questions are documented or resolved
