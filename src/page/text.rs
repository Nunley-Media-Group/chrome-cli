use serde::Serialize;

use agentchrome::error::AppError;

use crate::cli::{GlobalOpts, PageTextArgs};

use super::{get_page_info, print_output, setup_session};

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

    // Output
    if global.output.plain {
        print!("{text}");
        return Ok(());
    }

    let output = PageTextResult { text, url, title };
    print_output(&output, &global.output)
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
}
