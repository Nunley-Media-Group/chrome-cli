use std::time::Instant;

use globset::GlobBuilder;
use serde::Serialize;
use tokio::time::Duration;

use agentchrome::error::AppError;

use crate::cli::{GlobalOpts, PageWaitArgs};
use crate::navigate::{DEFAULT_NAVIGATE_TIMEOUT_MS, wait_for_network_idle};

use super::{get_page_info, print_output, setup_session};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct WaitResult {
    condition: String,
    matched: bool,
    url: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<String>,
}

// =============================================================================
// Plain text output
// =============================================================================

fn print_wait_plain(result: &WaitResult) {
    println!("Condition: {}", result.condition);
    println!("Matched:   {}", result.matched);
    println!("URL:       {}", result.url);
    println!("Title:     {}", result.title);
    if let Some(ref p) = result.pattern {
        println!("Pattern:   {p}");
    }
    if let Some(ref t) = result.text {
        println!("Text:      {t}");
    }
    if let Some(ref s) = result.selector {
        println!("Selector:  {s}");
    }
}

// =============================================================================
// Condition checking helpers
// =============================================================================

/// Evaluate a JS expression via Runtime.evaluate, returning the result value.
/// Returns `None` if the evaluation fails (e.g. page is navigating).
async fn eval_js(
    managed: &agentchrome::connection::ManagedSession,
    expression: &str,
) -> Option<serde_json::Value> {
    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": expression })),
        )
        .await
        .ok()?;
    Some(result["result"]["value"].clone())
}

/// Check the URL condition: fetch location.href and match against a glob.
async fn check_url_condition(
    managed: &agentchrome::connection::ManagedSession,
    matcher: &globset::GlobMatcher,
) -> bool {
    let Some(val) = eval_js(managed, "location.href").await else {
        return false;
    };
    let Some(href) = val.as_str() else {
        return false;
    };
    matcher.is_match(href)
}

/// Check the text condition: evaluate document.body.innerText.includes(text).
async fn check_text_condition(
    managed: &agentchrome::connection::ManagedSession,
    text: &str,
) -> bool {
    let encoded = serde_json::to_string(text).unwrap_or_default();
    let expr = format!("document.body.innerText.includes({encoded})");
    let Some(val) = eval_js(managed, &expr).await else {
        return false;
    };
    val.as_bool().unwrap_or(false)
}

/// Check the selector condition: evaluate document.querySelector(sel) !== null.
async fn check_selector_condition(
    managed: &agentchrome::connection::ManagedSession,
    selector: &str,
) -> bool {
    let encoded = serde_json::to_string(selector).unwrap_or_default();
    let expr = format!("document.querySelector({encoded}) !== null");
    let Some(val) = eval_js(managed, &expr).await else {
        return false;
    };
    val.as_bool().unwrap_or(false)
}

// =============================================================================
// Command executor
// =============================================================================

pub async fn execute_wait(global: &GlobalOpts, args: &PageWaitArgs) -> Result<(), AppError> {
    let timeout_ms = global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);

    // Network idle path (event-driven, not polled)
    if args.network_idle {
        return execute_network_idle_wait(global, timeout_ms).await;
    }

    // Poll-based conditions: --url, --text, --selector
    let (_client, mut managed) = setup_session(global).await?;
    managed.ensure_domain("Runtime").await?;

    if let Some(ref pattern) = args.url {
        poll_url(global, &managed, pattern, timeout_ms, args.interval).await
    } else if let Some(ref text) = args.text {
        poll_text(global, &managed, text, timeout_ms, args.interval).await
    } else if let Some(ref selector) = args.selector {
        poll_selector(global, &managed, selector, timeout_ms, args.interval).await
    } else {
        unreachable!("No condition specified — clap should have caught this");
    }
}

/// Poll for URL matching a glob pattern.
async fn poll_url(
    global: &GlobalOpts,
    managed: &agentchrome::connection::ManagedSession,
    pattern: &str,
    timeout_ms: u64,
    interval_ms: u64,
) -> Result<(), AppError> {
    let glob = GlobBuilder::new(pattern)
        .literal_separator(false)
        .build()
        .map_err(|e| AppError {
            message: format!("Invalid glob pattern: {e}"),
            code: agentchrome::error::ExitCode::GeneralError,
            custom_json: None,
        })?;
    let matcher = glob.compile_matcher();
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(interval_ms);

    // Immediate pre-check
    if check_url_condition(managed, &matcher).await {
        return finish_poll_wait(
            global,
            managed,
            "url",
            Some(pattern.to_string()),
            None,
            None,
        )
        .await;
    }

    loop {
        tokio::time::sleep(interval).await;
        if Instant::now() > deadline {
            return Err(AppError::wait_timeout(
                timeout_ms,
                &format!("url \"{pattern}\" not matched"),
            ));
        }
        if check_url_condition(managed, &matcher).await {
            return finish_poll_wait(
                global,
                managed,
                "url",
                Some(pattern.to_string()),
                None,
                None,
            )
            .await;
        }
    }
}

/// Poll for text appearing in page content.
async fn poll_text(
    global: &GlobalOpts,
    managed: &agentchrome::connection::ManagedSession,
    text: &str,
    timeout_ms: u64,
    interval_ms: u64,
) -> Result<(), AppError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(interval_ms);

    if check_text_condition(managed, text).await {
        return finish_poll_wait(global, managed, "text", None, Some(text.to_string()), None).await;
    }

    loop {
        tokio::time::sleep(interval).await;
        if Instant::now() > deadline {
            return Err(AppError::wait_timeout(
                timeout_ms,
                &format!("text \"{text}\" not found"),
            ));
        }
        if check_text_condition(managed, text).await {
            return finish_poll_wait(global, managed, "text", None, Some(text.to_string()), None)
                .await;
        }
    }
}

/// Poll for a CSS selector matching an element.
async fn poll_selector(
    global: &GlobalOpts,
    managed: &agentchrome::connection::ManagedSession,
    selector: &str,
    timeout_ms: u64,
    interval_ms: u64,
) -> Result<(), AppError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(interval_ms);

    if check_selector_condition(managed, selector).await {
        return finish_poll_wait(
            global,
            managed,
            "selector",
            None,
            None,
            Some(selector.to_string()),
        )
        .await;
    }

    loop {
        tokio::time::sleep(interval).await;
        if Instant::now() > deadline {
            return Err(AppError::wait_timeout(
                timeout_ms,
                &format!("selector \"{selector}\" not found"),
            ));
        }
        if check_selector_condition(managed, selector).await {
            return finish_poll_wait(
                global,
                managed,
                "selector",
                None,
                None,
                Some(selector.to_string()),
            )
            .await;
        }
    }
}

/// Build and output the `WaitResult` after a poll-based condition is met.
async fn finish_poll_wait(
    global: &GlobalOpts,
    managed: &agentchrome::connection::ManagedSession,
    condition: &str,
    pattern: Option<String>,
    text: Option<String>,
    selector: Option<String>,
) -> Result<(), AppError> {
    let (url, title) = get_page_info(managed).await?;
    let result = WaitResult {
        condition: condition.to_string(),
        matched: true,
        url,
        title,
        pattern,
        text,
        selector,
    };

    if global.output.plain {
        print_wait_plain(&result);
    } else {
        print_output(&result, &global.output)?;
    }

    Ok(())
}

/// Event-driven network idle wait path.
async fn execute_network_idle_wait(global: &GlobalOpts, timeout_ms: u64) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    managed.ensure_domain("Runtime").await?;
    managed.ensure_domain("Network").await?;

    let req_rx = managed.subscribe("Network.requestWillBeSent").await?;
    let fin_rx = managed.subscribe("Network.loadingFinished").await?;
    let fail_rx = managed.subscribe("Network.loadingFailed").await?;

    wait_for_network_idle(req_rx, fin_rx, fail_rx, timeout_ms).await?;

    let (url, title) = get_page_info(&managed).await?;
    let result = WaitResult {
        condition: "network-idle".to_string(),
        matched: true,
        url,
        title,
        pattern: None,
        text: None,
        selector: None,
    };

    if global.output.plain {
        print_wait_plain(&result);
    } else {
        print_output(&result, &global.output)?;
    }

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use globset::GlobBuilder;

    fn build_matcher(pattern: &str) -> globset::GlobMatcher {
        GlobBuilder::new(pattern)
            .literal_separator(false)
            .build()
            .unwrap()
            .compile_matcher()
    }

    #[test]
    fn glob_wildcard_matches_across_slashes() {
        let m = build_matcher("*/dashboard*");
        assert!(m.is_match("https://example.com/dashboard"));
        assert!(m.is_match("https://example.com/dashboard/settings"));
        assert!(m.is_match("http://localhost:3000/dashboard"));
    }

    #[test]
    fn glob_exact_url_match() {
        let m = build_matcher("https://example.com");
        assert!(m.is_match("https://example.com"));
        assert!(!m.is_match("https://example.com/path"));
    }

    #[test]
    fn glob_no_match() {
        let m = build_matcher("*/login*");
        assert!(!m.is_match("https://example.com/dashboard"));
        assert!(!m.is_match("https://example.com/"));
    }

    #[test]
    fn glob_star_matches_everything() {
        let m = build_matcher("*");
        assert!(m.is_match("https://example.com/anything/at/all"));
        assert!(m.is_match(""));
    }

    #[test]
    fn glob_complex_pattern() {
        let m = build_matcher("https://*.example.com/*");
        assert!(m.is_match("https://app.example.com/page"));
        assert!(m.is_match("https://sub.example.com/"));
    }

    #[test]
    fn wait_result_serialization_url_condition() {
        let result = super::WaitResult {
            condition: "url".to_string(),
            matched: true,
            url: "https://example.com/dashboard".to_string(),
            title: "Dashboard".to_string(),
            pattern: Some("*/dashboard*".to_string()),
            text: None,
            selector: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "url");
        assert_eq!(json["matched"], true);
        assert_eq!(json["url"], "https://example.com/dashboard");
        assert_eq!(json["pattern"], "*/dashboard*");
        assert!(json.get("text").is_none());
        assert!(json.get("selector").is_none());
    }

    #[test]
    fn wait_result_serialization_text_condition() {
        let result = super::WaitResult {
            condition: "text".to_string(),
            matched: true,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            pattern: None,
            text: Some("Products".to_string()),
            selector: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "text");
        assert_eq!(json["text"], "Products");
        assert!(json.get("pattern").is_none());
        assert!(json.get("selector").is_none());
    }

    #[test]
    fn wait_result_serialization_selector_condition() {
        let result = super::WaitResult {
            condition: "selector".to_string(),
            matched: true,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            pattern: None,
            text: None,
            selector: Some("#results-table".to_string()),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "selector");
        assert_eq!(json["selector"], "#results-table");
        assert!(json.get("pattern").is_none());
        assert!(json.get("text").is_none());
    }

    #[test]
    fn wait_result_serialization_network_idle() {
        let result = super::WaitResult {
            condition: "network-idle".to_string(),
            matched: true,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            pattern: None,
            text: None,
            selector: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "network-idle");
        assert_eq!(json["matched"], true);
        assert!(json.get("pattern").is_none());
        assert!(json.get("text").is_none());
        assert!(json.get("selector").is_none());
    }
}
