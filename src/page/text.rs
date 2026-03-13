use serde::Serialize;

use agentchrome::error::AppError;

use crate::cli::{GlobalOpts, PageTextArgs};

use super::{get_page_info, setup_session};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct PageTextResult {
    text: String,
    url: String,
    title: String,
}

// =============================================================================
// Helpers
// =============================================================================

/// Escape a CSS selector for embedding in a JavaScript double-quoted string.
fn escape_selector(selector: &str) -> String {
    selector.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Filter text to only paragraphs (double-newline separated) matching the query.
fn filter_text_paragraphs(text: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    text.split("\n\n")
        .filter(|paragraph| paragraph.to_lowercase().contains(&query_lower))
        .collect::<Vec<_>>()
        .join("\n\n")
}

// =============================================================================
// Command executor
// =============================================================================

pub async fn execute_text(global: &GlobalOpts, args: &PageTextArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Enable Runtime domain
    managed.ensure_domain("Runtime").await?;

    // Build JS expression
    let expression = match &args.selector {
        None => "document.body?.innerText ?? ''".to_string(),
        Some(selector) => {
            let escaped = escape_selector(selector);
            format!(
                r#"(() => {{ const el = document.querySelector("{escaped}"); if (!el) return {{ __error: "not_found" }}; return el.innerText; }})()"#
            )
        }
    };

    let params = serde_json::json!({
        "expression": expression,
        "returnByValue": true,
    });

    let result = managed
        .send_command("Runtime.evaluate", Some(params))
        .await?;

    // Check for exception
    if let Some(exception) = result.get("exceptionDetails") {
        let description = exception["exception"]["description"]
            .as_str()
            .or_else(|| exception["text"].as_str())
            .unwrap_or("unknown error");
        return Err(AppError::evaluation_failed(description));
    }

    let value = &result["result"]["value"];

    // Check for sentinel error object
    if let Some(error) = value.get("__error") {
        if error.as_str() == Some("not_found") {
            let selector = args.selector.as_deref().unwrap_or("unknown");
            return Err(AppError::element_not_found(selector));
        }
    }

    let text = value.as_str().unwrap_or_default().to_string();

    // Get page info
    let (url, title) = get_page_info(&managed).await?;

    // Apply --search filter if present
    let text = if let Some(ref query) = args.search {
        filter_text_paragraphs(&text, query)
    } else {
        text
    };

    // Output
    if global.output.plain {
        print!("{text}");
        return Ok(());
    }

    let output = PageTextResult { text, url, title };

    if args.search.is_some() {
        return crate::output::emit_searched(&output, &global.output);
    }

    crate::output::emit(&output, &global.output, "page text", |r| {
        serde_json::json!({
            "character_count": r.text.len(),
            "line_count": r.text.lines().count(),
        })
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_text_result_serialization() {
        let result = PageTextResult {
            text: "Hello, world!".to_string(),
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["text"], "Hello, world!");
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["title"], "Example");
    }

    #[test]
    fn page_text_result_empty_text() {
        let result = PageTextResult {
            text: String::new(),
            url: "about:blank".to_string(),
            title: String::new(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["text"], "");
        assert_eq!(json["url"], "about:blank");
    }

    #[test]
    fn escape_selector_no_special_chars() {
        assert_eq!(escape_selector("#content"), "#content");
    }

    #[test]
    fn escape_selector_with_quotes() {
        assert_eq!(
            escape_selector(r#"div[data-name="test"]"#),
            r#"div[data-name=\"test\"]"#
        );
    }

    #[test]
    fn escape_selector_with_backslash() {
        assert_eq!(escape_selector(r"div\.class"), r"div\\.class");
    }

    #[test]
    fn filter_text_paragraphs_basic() {
        let text = "First paragraph about errors.\n\nSecond paragraph about warnings.\n\nThird paragraph about errors again.";
        let filtered = filter_text_paragraphs(text, "error");
        assert!(filtered.contains("First paragraph about errors."));
        assert!(filtered.contains("Third paragraph about errors again."));
        assert!(!filtered.contains("warnings"));
    }

    #[test]
    fn filter_text_paragraphs_case_insensitive() {
        let text = "ERROR occurred here.\n\nNothing to see.";
        let filtered = filter_text_paragraphs(text, "error");
        assert!(filtered.contains("ERROR occurred here."));
        assert!(!filtered.contains("Nothing to see"));
    }

    #[test]
    fn filter_text_paragraphs_no_match() {
        let text = "Hello world.\n\nGoodbye world.";
        let filtered = filter_text_paragraphs(text, "missing");
        assert!(filtered.is_empty());
    }
}
