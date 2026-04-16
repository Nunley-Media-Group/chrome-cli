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
    #[serde(skip_serializing_if = "Option::is_none")]
    js_expression: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u64>,
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
    if let Some(ref expr) = result.js_expression {
        println!("Expression: {expr}");
    }
    if let Some(count) = result.count {
        println!("Count:     {count}");
    }
}

// =============================================================================
// Condition checking helpers
// =============================================================================

/// Evaluate a JS expression via Runtime.evaluate, returning the result value.
/// Returns `None` if the evaluation fails (e.g. page is navigating).
pub(crate) async fn eval_js(
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
    // If the evaluation threw a JS exception, treat it as failure
    if result.get("exceptionDetails").is_some() {
        return None;
    }
    Some(result["result"]["value"].clone())
}

/// Outcome of evaluating a JS expression with rich error discrimination.
enum EvalOutcome {
    /// Expression evaluated successfully, result value returned.
    Value(serde_json::Value),
    /// Expression threw a JavaScript exception (`SyntaxError`, `TypeError`, etc.).
    JsException(String),
    /// CDP communication failed (page navigating, context destroyed, etc.).
    TransientError,
}

/// Evaluate a JS expression with rich error information for `--js-expression`.
/// Distinguishes between successful evaluation, JS exceptions, and CDP failures.
async fn eval_js_checked(
    managed: &agentchrome::connection::ManagedSession,
    expression: &str,
) -> EvalOutcome {
    let Ok(result) = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": expression })),
        )
        .await
    else {
        return EvalOutcome::TransientError;
    };

    // Check for JavaScript exception
    if let Some(exception) = result.get("exceptionDetails") {
        let msg = exception["exception"]["description"]
            .as_str()
            .or_else(|| exception["text"].as_str())
            .unwrap_or("Unknown JavaScript error")
            .to_string();
        return EvalOutcome::JsException(msg);
    }

    let value = result["result"]["value"].clone();
    EvalOutcome::Value(value)
}

/// Check whether a `serde_json::Value` is truthy according to JavaScript semantics.
fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Null => false,
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => true,
    }
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

/// Check the selector condition with optional count threshold.
/// When `count <= 1`: checks `document.querySelector(sel) !== null` (presence).
/// When `count > 1`: checks `document.querySelectorAll(sel).length >= count`.
pub(crate) async fn check_selector_condition(
    managed: &agentchrome::connection::ManagedSession,
    selector: &str,
    count: u64,
) -> bool {
    let encoded = serde_json::to_string(selector).unwrap_or_default();
    let expr = if count <= 1 {
        format!("document.querySelector({encoded}) !== null")
    } else {
        format!("document.querySelectorAll({encoded}).length >= {count}")
    };
    let Some(val) = eval_js(managed, &expr).await else {
        return false;
    };
    val.as_bool().unwrap_or(false)
}

// =============================================================================
// Command executor
// =============================================================================

pub async fn execute_wait(
    global: &GlobalOpts,
    args: &PageWaitArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let timeout_ms = global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);

    // Network idle path (event-driven, not polled)
    if args.network_idle {
        return execute_network_idle_wait(global, timeout_ms).await;
    }

    // Poll-based conditions: --url, --text, --selector, --js-expression
    let (client, mut managed) = setup_session(global).await?;

    // Resolve optional frame context
    let mut frame_ctx = if let Some(frame_str) = frame {
        let arg = agentchrome::frame::parse_frame_arg(frame_str)?;
        Some(agentchrome::frame::resolve_frame(&client, &mut managed, &arg).await?)
    } else {
        None
    };

    // Enable Runtime domain (needs &mut)
    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("Runtime").await?;
    }

    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    if let Some(ref pattern) = args.url {
        poll_url(global, effective, pattern, timeout_ms, args.interval).await
    } else if let Some(ref text) = args.text {
        poll_text(global, effective, text, timeout_ms, args.interval).await
    } else if let Some(ref selector) = args.selector {
        poll_selector(
            global,
            effective,
            selector,
            args.count,
            timeout_ms,
            args.interval,
        )
        .await
    } else if let Some(ref expression) = args.js_expression {
        poll_js_expression(global, effective, expression, timeout_ms, args.interval).await
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
        return finish_poll_wait(
            global,
            managed,
            "text",
            None,
            Some(text.to_string()),
            None,
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
                &format!("text \"{text}\" not found"),
            ));
        }
        if check_text_condition(managed, text).await {
            return finish_poll_wait(
                global,
                managed,
                "text",
                None,
                Some(text.to_string()),
                None,
                None,
                None,
            )
            .await;
        }
    }
}

/// Poll for a CSS selector matching elements, with optional count threshold.
async fn poll_selector(
    global: &GlobalOpts,
    managed: &agentchrome::connection::ManagedSession,
    selector: &str,
    count: u64,
    timeout_ms: u64,
    interval_ms: u64,
) -> Result<(), AppError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(interval_ms);
    let count_output = if count > 1 { Some(count) } else { None };

    if check_selector_condition(managed, selector, count).await {
        return finish_poll_wait(
            global,
            managed,
            "selector",
            None,
            None,
            Some(selector.to_string()),
            None,
            count_output,
        )
        .await;
    }

    loop {
        tokio::time::sleep(interval).await;
        if Instant::now() > deadline {
            let condition = if count > 1 {
                format!("selector \"{selector}\" count >= {count} not reached")
            } else {
                format!("selector \"{selector}\" not found")
            };
            return Err(AppError::wait_timeout(timeout_ms, &condition));
        }
        if check_selector_condition(managed, selector, count).await {
            return finish_poll_wait(
                global,
                managed,
                "selector",
                None,
                None,
                Some(selector.to_string()),
                None,
                count_output,
            )
            .await;
        }
    }
}

/// Poll for a JavaScript expression to evaluate to truthy.
async fn poll_js_expression(
    global: &GlobalOpts,
    managed: &agentchrome::connection::ManagedSession,
    expression: &str,
    timeout_ms: u64,
    interval_ms: u64,
) -> Result<(), AppError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let interval = Duration::from_millis(interval_ms);
    let mut consecutive_errors: u32 = 0;

    // Immediate pre-check
    match eval_js_checked(managed, expression).await {
        EvalOutcome::Value(ref v) if is_truthy(v) => {
            return finish_poll_wait(
                global,
                managed,
                "js-expression",
                None,
                None,
                None,
                Some(expression.to_string()),
                None,
            )
            .await;
        }
        EvalOutcome::JsException(_) => {
            consecutive_errors += 1;
        }
        EvalOutcome::Value(_) | EvalOutcome::TransientError => {
            consecutive_errors = 0;
        }
    }

    loop {
        tokio::time::sleep(interval).await;
        if Instant::now() > deadline {
            return Err(AppError::wait_timeout(
                timeout_ms,
                "js-expression not truthy",
            ));
        }
        match eval_js_checked(managed, expression).await {
            EvalOutcome::Value(ref v) if is_truthy(v) => {
                return finish_poll_wait(
                    global,
                    managed,
                    "js-expression",
                    None,
                    None,
                    None,
                    Some(expression.to_string()),
                    None,
                )
                .await;
            }
            EvalOutcome::JsException(msg) => {
                consecutive_errors += 1;
                if consecutive_errors >= 3 {
                    return Err(AppError::js_eval_error(&msg));
                }
            }
            EvalOutcome::Value(_) | EvalOutcome::TransientError => {
                consecutive_errors = 0;
            }
        }
    }
}

/// Build and output the `WaitResult` after a poll-based condition is met.
#[allow(clippy::too_many_arguments)]
async fn finish_poll_wait(
    global: &GlobalOpts,
    managed: &agentchrome::connection::ManagedSession,
    condition: &str,
    pattern: Option<String>,
    text: Option<String>,
    selector: Option<String>,
    js_expression: Option<String>,
    count: Option<u64>,
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
        js_expression,
        count,
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
        js_expression: None,
        count: None,
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

    // --- is_truthy tests ---

    #[test]
    fn is_truthy_bool_true() {
        assert!(super::is_truthy(&serde_json::Value::Bool(true)));
    }

    #[test]
    fn is_truthy_bool_false() {
        assert!(!super::is_truthy(&serde_json::Value::Bool(false)));
    }

    #[test]
    fn is_truthy_number_nonzero() {
        assert!(super::is_truthy(&serde_json::json!(42)));
        assert!(super::is_truthy(&serde_json::json!(-1)));
        assert!(super::is_truthy(&serde_json::json!(0.5)));
    }

    #[test]
    fn is_truthy_number_zero() {
        assert!(!super::is_truthy(&serde_json::json!(0)));
        assert!(!super::is_truthy(&serde_json::json!(0.0)));
    }

    #[test]
    fn is_truthy_string_nonempty() {
        assert!(super::is_truthy(&serde_json::json!("hello")));
        assert!(super::is_truthy(&serde_json::json!("0")));
    }

    #[test]
    fn is_truthy_string_empty() {
        assert!(!super::is_truthy(&serde_json::json!("")));
    }

    #[test]
    fn is_truthy_null() {
        assert!(!super::is_truthy(&serde_json::Value::Null));
    }

    #[test]
    fn is_truthy_array() {
        assert!(super::is_truthy(&serde_json::json!([])));
        assert!(super::is_truthy(&serde_json::json!([1, 2])));
    }

    #[test]
    fn is_truthy_object() {
        assert!(super::is_truthy(&serde_json::json!({})));
        assert!(super::is_truthy(&serde_json::json!({"a": 1})));
    }

    // --- WaitResult serialization tests ---

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
            js_expression: None,
            count: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "url");
        assert_eq!(json["matched"], true);
        assert_eq!(json["url"], "https://example.com/dashboard");
        assert_eq!(json["pattern"], "*/dashboard*");
        assert!(json.get("text").is_none());
        assert!(json.get("selector").is_none());
        assert!(json.get("js_expression").is_none());
        assert!(json.get("count").is_none());
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
            js_expression: None,
            count: None,
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
            js_expression: None,
            count: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "selector");
        assert_eq!(json["selector"], "#results-table");
        assert!(json.get("pattern").is_none());
        assert!(json.get("text").is_none());
        assert!(json.get("count").is_none());
    }

    #[test]
    fn wait_result_serialization_selector_with_count() {
        let result = super::WaitResult {
            condition: "selector".to_string(),
            matched: true,
            url: "https://example.com/items".to_string(),
            title: "Item List".to_string(),
            pattern: None,
            text: None,
            selector: Some(".item".to_string()),
            js_expression: None,
            count: Some(3),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "selector");
        assert_eq!(json["selector"], ".item");
        assert_eq!(json["count"], 3);
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
            js_expression: None,
            count: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "network-idle");
        assert_eq!(json["matched"], true);
        assert!(json.get("pattern").is_none());
        assert!(json.get("text").is_none());
        assert!(json.get("selector").is_none());
        assert!(json.get("js_expression").is_none());
    }

    #[test]
    fn wait_result_serialization_js_expression() {
        let result = super::WaitResult {
            condition: "js-expression".to_string(),
            matched: true,
            url: "https://example.com/wizard".to_string(),
            title: "Setup Wizard".to_string(),
            pattern: None,
            text: None,
            selector: None,
            js_expression: Some(
                "document.querySelector('.next-btn').disabled === false".to_string(),
            ),
            count: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["condition"], "js-expression");
        assert_eq!(json["matched"], true);
        assert_eq!(
            json["js_expression"],
            "document.querySelector('.next-btn').disabled === false"
        );
        assert!(json.get("pattern").is_none());
        assert!(json.get("text").is_none());
        assert!(json.get("selector").is_none());
        assert!(json.get("count").is_none());
    }
}
