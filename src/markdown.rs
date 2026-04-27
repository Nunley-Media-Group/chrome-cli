use std::io::Read as _;
use std::path::Path;
use std::time::Duration;

use kuchiki::traits::TendrilSink;
use kuchiki::{NodeRef, parse_html};
use quick_html2md::{MarkdownOptions, html_to_markdown_with_options};
use serde::Serialize;
use url::Url;

use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, MarkdownArgs};
use crate::output;

const PAGE_SOURCE_SCRIPT: &str = r#"(() => JSON.stringify({
  html: document.documentElement ? document.documentElement.outerHTML : "",
  url: location.href || null,
  base_url: document.baseURI || location.href || null,
  title: document.title || null
}))()"#;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SourceKind {
    Page,
    File,
    Stdin,
    Url,
}

impl SourceKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::File => "file",
            Self::Stdin => "stdin",
            Self::Url => "url",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct SourceInfo {
    kind: SourceKind,
    url: Option<String>,
    title: Option<String>,
    path: Option<String>,
    selector: Option<String>,
}

#[derive(Debug)]
struct SourceDocument {
    html: String,
    source: SourceInfo,
    base_url: Option<Url>,
}

#[derive(Debug, Clone, Serialize)]
struct MarkdownMetadata {
    input_bytes: usize,
    markdown_bytes: usize,
    removed_node_count: usize,
    primary_region: Option<String>,
    links_preserved: bool,
    images_included: bool,
}

#[derive(Debug, Clone, Serialize)]
struct MarkdownResult {
    markdown: String,
    source: SourceInfo,
    metadata: MarkdownMetadata,
}

#[derive(Debug, Clone)]
struct ConversionOptions {
    selector: Option<String>,
    strip_links: bool,
    include_images: bool,
}

/// Execute the `markdown` command.
///
/// # Errors
///
/// Returns `AppError` when source acquisition, cleanup, conversion, or output
/// emission fails.
pub async fn execute_markdown(global: &GlobalOpts, args: &MarkdownArgs) -> Result<(), AppError> {
    validate_base_url_contract(args)?;

    let mut document = acquire_source(global, args).await?;
    document.source.selector.clone_from(&args.selector);

    let options = ConversionOptions {
        selector: args.selector.clone(),
        strip_links: args.strip_links,
        include_images: args.include_images,
    };
    let result = convert_clean_markdown(document, &options)?;

    if global.output.plain {
        output::emit_plain(&result.markdown, &global.output)?;
        return Ok(());
    }

    output::emit(&result, &global.output, "markdown", |r| {
        serde_json::json!({
            "source": r.source.kind.as_str(),
            "url": r.source.url,
            "path": r.source.path,
            "selector": r.source.selector,
            "markdown_bytes": r.metadata.markdown_bytes,
            "primary_region": r.metadata.primary_region,
            "links_preserved": r.metadata.links_preserved,
            "images_included": r.metadata.images_included,
        })
    })
}

fn validate_base_url_contract(args: &MarkdownArgs) -> Result<(), AppError> {
    if args.base_url.is_some() && args.url.is_some() {
        return Err(markdown_error(
            "--base-url is only supported with --file or --stdin; --url uses the fetched URL",
            ExitCode::GeneralError,
        ));
    }
    if args.base_url.is_some() && args.file.is_none() && !args.stdin {
        return Err(markdown_error(
            "--base-url is only supported with --file or --stdin; page mode uses document.baseURI",
            ExitCode::GeneralError,
        ));
    }
    Ok(())
}

async fn acquire_source(
    global: &GlobalOpts,
    args: &MarkdownArgs,
) -> Result<SourceDocument, AppError> {
    if let Some(path) = &args.file {
        return read_file_source(path, args);
    }
    if args.stdin {
        return read_stdin_source(args);
    }
    if let Some(url) = &args.url {
        return fetch_url_source(url, global.timeout, args.max_input_bytes).await;
    }
    read_page_source(global).await
}

async fn read_page_source(global: &GlobalOpts) -> Result<SourceDocument, AppError> {
    let (_client, mut managed) = output::setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }
    managed.ensure_domain("Runtime").await?;

    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": PAGE_SOURCE_SCRIPT,
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| {
            markdown_error(
                &format!("Page HTML extraction failed: {e}"),
                ExitCode::ProtocolError,
            )
        })?;

    if let Some(exception) = result.get("exceptionDetails") {
        let description = exception["exception"]["description"]
            .as_str()
            .or_else(|| exception["text"].as_str())
            .unwrap_or("unknown error");
        return Err(markdown_error(
            &format!("Page HTML extraction failed: {description}"),
            ExitCode::ProtocolError,
        ));
    }

    let value = result["result"]["value"].as_str().ok_or_else(|| {
        markdown_error(
            "Page HTML extraction returned no value",
            ExitCode::ProtocolError,
        )
    })?;
    let page: PageSourcePayload = serde_json::from_str(value).map_err(|e| {
        markdown_error(
            &format!("Page HTML extraction returned invalid payload: {e}"),
            ExitCode::ProtocolError,
        )
    })?;

    let base_url = page
        .base_url
        .as_deref()
        .or(page.url.as_deref())
        .and_then(|u| Url::parse(u).ok());

    Ok(SourceDocument {
        html: page.html,
        source: SourceInfo {
            kind: SourceKind::Page,
            url: page.url,
            title: page.title,
            path: None,
            selector: None,
        },
        base_url,
    })
}

#[derive(Debug, serde::Deserialize)]
struct PageSourcePayload {
    html: String,
    url: Option<String>,
    base_url: Option<String>,
    title: Option<String>,
}

fn read_file_source(path: &Path, args: &MarkdownArgs) -> Result<SourceDocument, AppError> {
    let path_display = path.display().to_string();
    let file = std::fs::File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            markdown_error(
                &format!("File not found: {path_display}"),
                ExitCode::GeneralError,
            )
        } else {
            markdown_error(
                &format!("File not readable: {path_display}: {e}"),
                ExitCode::GeneralError,
            )
        }
    })?;
    let html = read_bounded_utf8(file, args.max_input_bytes, &path_display)?;
    let title = extract_title(&html);
    let base_url = parse_optional_base_url(args.base_url.as_deref())?;
    Ok(SourceDocument {
        html,
        source: SourceInfo {
            kind: SourceKind::File,
            url: base_url.as_ref().map(ToString::to_string),
            title,
            path: Some(path_display),
            selector: None,
        },
        base_url,
    })
}

fn read_stdin_source(args: &MarkdownArgs) -> Result<SourceDocument, AppError> {
    let html = read_bounded_utf8(std::io::stdin().lock(), args.max_input_bytes, "stdin")?;
    let title = extract_title(&html);
    let base_url = parse_optional_base_url(args.base_url.as_deref())?;
    Ok(SourceDocument {
        html,
        source: SourceInfo {
            kind: SourceKind::Stdin,
            url: base_url.as_ref().map(ToString::to_string),
            title,
            path: None,
            selector: None,
        },
        base_url,
    })
}

async fn fetch_url_source(
    raw_url: &str,
    timeout_ms: Option<u64>,
    max_input_bytes: usize,
) -> Result<SourceDocument, AppError> {
    let parsed = Url::parse(raw_url).map_err(|e| {
        markdown_error(
            &format!("Invalid URL for --url: {raw_url}: {e}"),
            ExitCode::GeneralError,
        )
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(markdown_error(
            "--url only supports http and https URLs",
            ExitCode::GeneralError,
        ));
    }

    let url = parsed.to_string();
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(30_000));
    let html =
        tokio::task::spawn_blocking(move || fetch_url_blocking(&url, timeout, max_input_bytes))
            .await
            .map_err(|e| {
                markdown_error(
                    &format!("URL fetch task failed: {e}"),
                    ExitCode::GeneralError,
                )
            })??;
    let title = extract_title(&html);

    Ok(SourceDocument {
        html,
        source: SourceInfo {
            kind: SourceKind::Url,
            url: Some(parsed.to_string()),
            title,
            path: None,
            selector: None,
        },
        base_url: Some(parsed),
    })
}

fn fetch_url_blocking(
    url: &str,
    timeout: Duration,
    max_input_bytes: usize,
) -> Result<String, AppError> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| map_ureq_error(error, max_input_bytes))?;
    let limit = limit_plus_one(max_input_bytes);
    let body = response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_string()
        .map_err(|error| map_ureq_error(error, max_input_bytes))?;
    enforce_input_limit(body, max_input_bytes, url)
}

fn read_bounded_utf8<R: std::io::Read>(
    mut reader: R,
    max_input_bytes: usize,
    label: &str,
) -> Result<String, AppError> {
    let mut bytes = Vec::new();
    let limit = limit_plus_one(max_input_bytes);
    reader
        .by_ref()
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|e| {
            markdown_error(
                &format!("Failed to read HTML input from {label}: {e}"),
                ExitCode::GeneralError,
            )
        })?;
    if bytes.len() > max_input_bytes {
        return Err(input_limit_error(label, max_input_bytes));
    }
    String::from_utf8(bytes).map_err(|e| {
        markdown_error(
            &format!("HTML input from {label} is not valid UTF-8: {e}"),
            ExitCode::GeneralError,
        )
    })
}

fn enforce_input_limit(
    html: String,
    max_input_bytes: usize,
    label: &str,
) -> Result<String, AppError> {
    if html.len() > max_input_bytes {
        return Err(input_limit_error(label, max_input_bytes));
    }
    Ok(html)
}

fn limit_plus_one(max_input_bytes: usize) -> u64 {
    u64::try_from(max_input_bytes)
        .unwrap_or(u64::MAX - 1)
        .saturating_add(1)
}

fn parse_optional_base_url(value: Option<&str>) -> Result<Option<Url>, AppError> {
    value
        .map(|raw| {
            let parsed = Url::parse(raw).map_err(|e| {
                markdown_error(
                    &format!("Invalid URL for --base-url: {raw}: {e}"),
                    ExitCode::GeneralError,
                )
            })?;
            if parsed.cannot_be_a_base() {
                return Err(markdown_error(
                    &format!("Invalid URL for --base-url: {raw} is not a base URL"),
                    ExitCode::GeneralError,
                ));
            }
            Ok(parsed)
        })
        .transpose()
}

fn convert_clean_markdown(
    document: SourceDocument,
    options: &ConversionOptions,
) -> Result<MarkdownResult, AppError> {
    let input_bytes = document.html.len();
    let mut removed_node_count = 0;
    let primary_region;
    let conversion_html;

    let parsed = parse_html().one(document.html);
    if let Some(selector) = &options.selector {
        let mut matches = Vec::new();
        let selected = parsed.select(selector).map_err(|()| {
            markdown_error(
                &format!("invalid selector '{selector}'"),
                ExitCode::GeneralError,
            )
        })?;
        for node in selected {
            matches.push(node.as_node().to_string());
        }
        if matches.is_empty() {
            return Err(markdown_error(
                &format!("selector '{selector}' did not match any nodes"),
                ExitCode::TargetError,
            ));
        }
        conversion_html = format!("<html><body>{}</body></html>", matches.join("\n"));
        primary_region = Some("selector".to_string());
    } else {
        removed_node_count += remove_noise_nodes(&parsed, false);
        let (region_node, region_name) = select_primary_region(&parsed);
        conversion_html = region_node.to_string();
        primary_region = Some(region_name);
    }

    let scoped = parse_html().one(conversion_html);
    removed_node_count += remove_noise_nodes(&scoped, options.selector.is_some());
    removed_node_count += unwrap_layout_tables(&scoped);
    normalize_code_language_hints(&scoped);

    let cleaned_html = scoped.to_string();
    let mut markdown_options = MarkdownOptions::new()
        .include_links(!options.strip_links)
        .include_images(options.include_images);
    if let Some(base_url) = &document.base_url {
        markdown_options = markdown_options.base_url(base_url.as_str());
    }
    let markdown = html_to_markdown_with_options(&cleaned_html, &markdown_options)
        .trim()
        .to_string();
    let markdown_bytes = markdown.len();

    Ok(MarkdownResult {
        markdown,
        source: document.source,
        metadata: MarkdownMetadata {
            input_bytes,
            markdown_bytes,
            removed_node_count,
            primary_region,
            links_preserved: !options.strip_links,
            images_included: options.include_images,
        },
    })
}

fn remove_noise_nodes(root: &NodeRef, selector_mode: bool) -> usize {
    let mut removed = 0;
    for selector in [
        "script",
        "style",
        "noscript",
        "head",
        "template",
        "svg",
        "canvas",
        "[hidden]",
        "[aria-hidden=\"true\"]",
    ] {
        removed += detach_matches(root, selector);
    }

    let mut attr_noise = Vec::new();
    if let Ok(elements) = root.select("*") {
        for element in elements {
            let node = element.as_node();
            if should_remove_element(node, selector_mode) {
                attr_noise.push(node.clone());
            }
        }
    }
    let count = attr_noise.len();
    for node in attr_noise {
        node.detach();
    }
    removed + count
}

fn detach_matches(root: &NodeRef, selector: &str) -> usize {
    let Ok(matches) = root.select(selector) else {
        return 0;
    };
    let nodes: Vec<NodeRef> = matches.map(|node| node.as_node().clone()).collect();
    let count = nodes.len();
    for node in nodes {
        node.detach();
    }
    count
}

fn should_remove_element(node: &NodeRef, selector_mode: bool) -> bool {
    let Some(element) = node.as_element() else {
        return false;
    };
    let tag = element.name.local.to_string().to_ascii_lowercase();
    let attrs = element.attributes.borrow();
    if has_hidden_style(attrs.get("style")) {
        return true;
    }

    let role = attrs.get("role").map(str::to_ascii_lowercase);
    if !selector_mode && is_structural_noise(&tag, role.as_deref()) {
        return true;
    }

    let mut fields = Vec::new();
    for name in ["id", "class", "aria-label"] {
        if let Some(value) = attrs.get(name) {
            fields.push(value.to_string());
        }
    }
    for (name, attr) in &attrs.map {
        let local = name.local.to_string();
        if local.starts_with("data-") {
            fields.push(local);
            fields.push(attr.value.clone());
        }
    }
    fields.iter().any(|value| has_boilerplate_keyword(value))
}

fn is_structural_noise(tag: &str, role: Option<&str>) -> bool {
    matches!(tag, "header" | "footer" | "nav" | "aside" | "form")
        || matches!(
            role,
            Some("banner" | "navigation" | "contentinfo" | "search" | "complementary")
        )
}

fn has_hidden_style(style: Option<&str>) -> bool {
    let Some(style) = style else {
        return false;
    };
    let compact = style
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    compact.contains("display:none") || compact.contains("visibility:hidden")
}

fn has_boilerplate_keyword(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if [
        "cookie",
        "consent",
        "gdpr",
        "advert",
        "promo",
        "share",
        "social",
        "newsletter",
        "subscribe",
        "skip-link",
        "skip_to",
        "sidebar",
        "side-bar",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return true;
    }
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|token| matches!(token, "ad" | "ads" | "cookie" | "consent" | "share"))
}

fn select_primary_region(root: &NodeRef) -> (NodeRef, String) {
    for (selector, fallback_name) in [
        ("main", "main"),
        ("[role=\"main\"]", "main"),
        ("article", "article"),
    ] {
        if let Some(best) = best_candidate(root, selector)
            && region_score(&best) >= 40
        {
            return (best, fallback_name.to_string());
        }
    }

    if let Some(body) = best_candidate(root, "body") {
        return (body, "body".to_string());
    }
    (root.clone(), "body".to_string())
}

fn best_candidate(root: &NodeRef, selector: &str) -> Option<NodeRef> {
    let matches = root.select(selector).ok()?;
    matches
        .map(|node| node.as_node().clone())
        .max_by_key(region_score)
}

fn region_score(node: &NodeRef) -> usize {
    let text = normalized_text(&node.text_contents());
    if text.is_empty() {
        return 0;
    }
    let text_len = text.len();
    let content_nodes = count_matches(
        node,
        "p, li, pre, blockquote, table, h1, h2, h3, h4, h5, h6",
    );
    let link_text_len = link_text_len(node);
    let link_penalty = (link_text_len.saturating_mul(100) / text_len.max(1)).min(80);
    text_len
        .saturating_add(content_nodes.saturating_mul(80))
        .saturating_sub(link_penalty)
}

fn count_matches(node: &NodeRef, selector: &str) -> usize {
    node.select(selector)
        .map(std::iter::Iterator::count)
        .unwrap_or(0)
}

fn link_text_len(node: &NodeRef) -> usize {
    node.select("a")
        .map(|matches| {
            matches
                .map(|link| normalized_text(&link.as_node().text_contents()).len())
                .sum()
        })
        .unwrap_or(0)
}

fn unwrap_layout_tables(root: &NodeRef) -> usize {
    let Ok(tables) = root.select("table") else {
        return 0;
    };
    let tables: Vec<NodeRef> = tables.map(|table| table.as_node().clone()).collect();
    let mut removed = 0;
    for table in tables {
        if is_layout_table(&table) {
            let text = normalized_text(&table.text_contents());
            if !text.is_empty() {
                table.insert_before(NodeRef::new_text(format!("{text}\n")));
            }
            table.detach();
            removed += 1;
        }
    }
    removed
}

fn is_layout_table(table: &NodeRef) -> bool {
    if count_matches(table, "th") > 0 {
        return false;
    }
    let row_count = count_matches(table, "tr");
    let cell_count = count_matches(table, "td");
    row_count <= 2 || cell_count <= 2
}

fn normalize_code_language_hints(root: &NodeRef) {
    let Ok(nodes) = root.select("pre, code") else {
        return;
    };
    let nodes: Vec<NodeRef> = nodes.map(|node| node.as_node().clone()).collect();
    for node in nodes {
        let Some(element) = node.as_element() else {
            continue;
        };
        let mut attrs = element.attributes.borrow_mut();
        let class = attrs.get("class").unwrap_or_default().to_string();
        if class
            .split_whitespace()
            .any(|cls| cls.starts_with("language-") || cls.starts_with("lang-"))
        {
            continue;
        }
        let language = attrs
            .get("data-language")
            .map(str::to_string)
            .or_else(|| language_from_highlight_source(&class));
        if let Some(language) = language
            && !language.is_empty()
        {
            let new_class = if class.is_empty() {
                format!("language-{language}")
            } else {
                format!("{class} language-{language}")
            };
            attrs.insert("class", new_class);
        }
    }
}

fn language_from_highlight_source(class: &str) -> Option<String> {
    class.split_whitespace().find_map(|cls| {
        cls.strip_prefix("highlight-source-")
            .map(std::string::ToString::to_string)
    })
}

fn extract_title(html: &str) -> Option<String> {
    let parsed = parse_html().one(html);
    parsed
        .select_first("title")
        .ok()
        .map(|title| normalized_text(&title.as_node().text_contents()))
        .filter(|title| !title.is_empty())
}

fn normalized_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn input_limit_error(label: &str, max_input_bytes: usize) -> AppError {
    markdown_error(
        &format!("HTML input from {label} exceeds input byte limit of {max_input_bytes}"),
        ExitCode::GeneralError,
    )
}

fn map_ureq_error(error: ureq::Error, max_input_bytes: usize) -> AppError {
    match error {
        ureq::Error::Timeout(_) => markdown_error(
            &format!("URL fetch timed out: {error}"),
            ExitCode::TimeoutError,
        ),
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Tls(_) => {
            markdown_error(
                &format!("URL fetch failed: {error}"),
                ExitCode::ConnectionError,
            )
        }
        ureq::Error::Io(ref io_error)
            if matches!(io_error.kind(), std::io::ErrorKind::TimedOut) =>
        {
            markdown_error(
                &format!("URL fetch timed out: {error}"),
                ExitCode::TimeoutError,
            )
        }
        ureq::Error::Io(_) => markdown_error(
            &format!("URL fetch failed: {error}"),
            ExitCode::ConnectionError,
        ),
        ureq::Error::BodyExceedsLimit(_) => input_limit_error("URL response", max_input_bytes),
        ureq::Error::StatusCode(code) => markdown_error(
            &format!("URL fetch failed with HTTP status {code}"),
            ExitCode::ConnectionError,
        ),
        other => markdown_error(
            &format!("URL fetch failed: {other}"),
            ExitCode::GeneralError,
        ),
    }
}

fn markdown_error(message: &str, code: ExitCode) -> AppError {
    AppError {
        message: message.to_string(),
        code,
        custom_json: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(html: &str, base_url: Option<&str>) -> SourceDocument {
        SourceDocument {
            html: html.to_string(),
            source: SourceInfo {
                kind: SourceKind::File,
                url: base_url.map(str::to_string),
                title: None,
                path: Some("fixture.html".to_string()),
                selector: None,
            },
            base_url: base_url.and_then(|url| Url::parse(url).ok()),
        }
    }

    fn convert(html: &str, options: &ConversionOptions, base_url: Option<&str>) -> MarkdownResult {
        convert_clean_markdown(source(html, base_url), options).expect("conversion should pass")
    }

    fn default_options() -> ConversionOptions {
        ConversionOptions {
            selector: None,
            strip_links: false,
            include_images: false,
        }
    }

    #[test]
    fn removes_boilerplate_and_prefers_main() {
        let html = r"
            <header>Global navigation</header>
            <main><h1>Title</h1><p>Primary article paragraph</p></main>
            <aside>Newsletter signup</aside>
            <script>function trackingPixel(){}</script>
        ";
        let result = convert(html, &default_options(), None);
        assert!(result.markdown.contains("# Title"));
        assert!(result.markdown.contains("Primary article paragraph"));
        assert!(!result.markdown.contains("Global navigation"));
        assert!(!result.markdown.contains("Newsletter signup"));
        assert!(!result.markdown.contains("trackingPixel"));
        assert_eq!(result.metadata.primary_region.as_deref(), Some("main"));
    }

    #[test]
    fn selector_scope_bypasses_primary_region() {
        let options = ConversionOptions {
            selector: Some("#appendix".to_string()),
            ..default_options()
        };
        let html = r#"
            <main><h1>Main</h1><p>Primary article paragraph</p></main>
            <section id="appendix"><h2>Appendix</h2><p>Scoped content</p></section>
        "#;
        let result = convert(html, &options, None);
        assert!(result.markdown.contains("Appendix"));
        assert!(result.markdown.contains("Scoped content"));
        assert!(!result.markdown.contains("Primary article paragraph"));
        assert_eq!(result.metadata.primary_region.as_deref(), Some("selector"));
    }

    #[test]
    fn missing_selector_is_target_error() {
        let options = ConversionOptions {
            selector: Some("#missing".to_string()),
            ..default_options()
        };
        let err = convert_clean_markdown(source("<main>content</main>", None), &options)
            .expect_err("missing selector should fail");
        assert!(err.message.contains("did not match"));
        assert!(matches!(err.code, ExitCode::TargetError));
    }

    #[test]
    fn link_and_image_options_are_deterministic() {
        let html = r#"<main><p><a href="/reference">Reference</a><img src="images/a.png" alt="Architecture diagram"></p></main>"#;
        let result = convert(
            html,
            &default_options(),
            Some("https://example.test/articles/"),
        );
        assert!(
            result
                .markdown
                .contains("[Reference](https://example.test/reference)")
        );
        assert!(!result.markdown.contains("![Architecture diagram]"));

        let stripped = convert(
            html,
            &ConversionOptions {
                strip_links: true,
                ..default_options()
            },
            Some("https://example.test/articles/"),
        );
        assert!(stripped.markdown.contains("Reference"));
        assert!(
            !stripped
                .markdown
                .contains("](https://example.test/reference)")
        );

        let with_images = convert(
            html,
            &ConversionOptions {
                include_images: true,
                ..default_options()
            },
            Some("https://example.test/articles/"),
        );
        assert!(
            with_images
                .markdown
                .contains("![Architecture diagram](https://example.test/articles/images/a.png)")
        );
    }

    #[test]
    fn preserves_code_language_and_unwraps_layout_table() {
        let html = r#"
            <main>
              <pre><code data-language="rust">fn scrape() {}</code></pre>
              <table><tr><td>Layout table text</td></tr></table>
              <table><tr><th>Field</th><th>Meaning</th></tr><tr><td>url</td><td>source</td></tr></table>
            </main>
        "#;
        let result = convert(html, &default_options(), None);
        assert!(result.markdown.contains("```rust"));
        assert!(result.markdown.contains("fn scrape()"));
        assert!(result.markdown.contains("Layout table text"));
        assert!(result.markdown.contains("| Field | Meaning |"));
        assert!(!result.markdown.contains("<table"));
    }

    #[test]
    fn bounded_reader_rejects_oversized_input() {
        let err = read_bounded_utf8("abcdef".as_bytes(), 3, "fixture")
            .expect_err("oversized input should fail");
        assert!(err.message.contains("input byte limit"));
        assert!(matches!(err.code, ExitCode::GeneralError));
    }
}
