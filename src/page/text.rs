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

// =============================================================================
// Command executor
// =============================================================================

pub async fn execute_text(
    global: &GlobalOpts,
    args: &PageTextArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    if args.deep && frame.is_some() {
        return Err(AppError {
            message: "--deep and --frame are mutually exclusive".to_string(),
            code: agentchrome::error::ExitCode::GeneralError,
            custom_json: None,
        });
    }

    if args.deep {
        return execute_text_deep(global).await;
    }

    let (client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Resolve optional frame context
    let mut frame_ctx = if let Some(frame_str) = frame {
        let arg = agentchrome::frame::parse_frame_arg(frame_str)?;
        Some(agentchrome::frame::resolve_frame(&client, &mut managed, &arg).await?)
    } else {
        None
    };

    // Enable Runtime domain on effective session (needs &mut)
    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("Runtime").await?;
    }

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

    let mut params = serde_json::json!({
        "expression": expression,
        "returnByValue": true,
    });

    // For same-origin frames, scope evaluation to the frame's execution context
    if let Some(ctx_id) = frame_ctx
        .as_ref()
        .and_then(agentchrome::frame::execution_context_id)
    {
        params["contextId"] = serde_json::Value::from(ctx_id);
    }

    let result = {
        let effective = if let Some(ref ctx) = frame_ctx {
            agentchrome::frame::frame_session(ctx, &managed)
        } else {
            &managed
        };
        effective
            .send_command("Runtime.evaluate", Some(params))
            .await?
    };

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
    if let Some(error) = value.get("__error")
        && error.as_str() == Some("not_found")
    {
        let selector = args.selector.as_deref().unwrap_or("unknown");
        return Err(AppError::element_not_found(selector));
    }

    let text = value.as_str().unwrap_or_default().to_string();

    // Get page info (always from main frame)
    let (url, title) = get_page_info(&managed).await?;

    // Output
    if global.output.plain {
        crate::output::emit_plain(&text, &global.output)?;
        return Ok(());
    }

    let output = PageTextResult { text, url, title };

    crate::output::emit(&output, &global.output, "page text", |r| {
        serde_json::json!({
            "character_count": r.text.len(),
            "line_count": r.text.lines().count(),
        })
    })
}

/// Aggregate text: concatenate main-frame text, every iframe's text, and
/// shadow-root text in document order.
async fn execute_text_deep(global: &GlobalOpts) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Subscribe BEFORE enabling Runtime so we capture the replay of all
    // existing execution contexts (Chrome replays them immediately on enable).
    let frame_ctx_map = collect_frame_execution_contexts(&mut managed).await;

    let frames = agentchrome::frame::list_frames(&mut managed).await?;

    let shadow_expr = r"(function(){
        function walk(root, out){
            var hosts = root.querySelectorAll('*');
            for (var i = 0; i < hosts.length; i++){
                if (hosts[i].shadowRoot){
                    var t = hosts[i].shadowRoot.textContent;
                    if (t && t.trim()) out.push(t);
                    walk(hosts[i].shadowRoot, out);
                }
            }
        }
        var out = [];
        walk(document, out);
        return out.join('\n');
    })()";

    let mut parts: Vec<String> = Vec::new();

    for frame_info in &frames {
        let body_expr = "document.body?.innerText ?? ''";
        let ctx_id_opt: Option<i64> = if frame_info.index == 0 {
            None
        } else {
            frame_ctx_map.get(&frame_info.id).copied()
        };

        // Body text.
        let mut body_params = serde_json::json!({
            "expression": body_expr,
            "returnByValue": true,
        });
        if let Some(cid) = ctx_id_opt {
            body_params["contextId"] = serde_json::Value::from(cid);
        }
        if let Ok(result) = managed
            .send_command("Runtime.evaluate", Some(body_params))
            .await
        {
            let s = result["result"]["value"].as_str().unwrap_or_default();
            if !s.is_empty() {
                parts.push(s.to_string());
            }
        }

        // Shadow DOM text (only meaningful where shadow roots exist).
        let mut shadow_params = serde_json::json!({
            "expression": shadow_expr,
            "returnByValue": true,
        });
        if let Some(cid) = ctx_id_opt {
            shadow_params["contextId"] = serde_json::Value::from(cid);
        }
        if let Ok(result) = managed
            .send_command("Runtime.evaluate", Some(shadow_params))
            .await
        {
            let s = result["result"]["value"].as_str().unwrap_or_default();
            if !s.is_empty() {
                parts.push(s.to_string());
            }
        }
    }

    let text = parts.join("\n");
    let (url, title) = get_page_info(&managed).await?;

    if global.output.plain {
        crate::output::emit_plain(&text, &global.output)?;
        return Ok(());
    }

    let output = PageTextResult { text, url, title };

    crate::output::emit(&output, &global.output, "page text", |r| {
        serde_json::json!({
            "character_count": r.text.len(),
            "line_count": r.text.lines().count(),
        })
    })
}

/// Subscribe to `Runtime.executionContextCreated` BEFORE enabling the Runtime
/// domain, then enable it. Chrome replays all existing execution contexts
/// immediately on enable, so subscribing first guarantees we receive them.
///
/// Returns a map of `frameId → default execution context ID` for all frames.
async fn collect_frame_execution_contexts(
    managed: &mut agentchrome::connection::ManagedSession,
) -> std::collections::HashMap<String, i64> {
    let mut frame_ctx_map = std::collections::HashMap::new();

    let Ok(mut rx) = managed.subscribe("Runtime.executionContextCreated").await else {
        // If subscribe fails, fall back gracefully (all frames will use default context).
        let _ = managed.ensure_domain("Runtime").await;
        return frame_ctx_map;
    };

    let _ = managed.ensure_domain("Runtime").await;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(300);
    while let Ok(Some(event)) = tokio::time::timeout_at(deadline, rx.recv()).await {
        let ctx = &event.params["context"];
        let aux = &ctx["auxData"];
        if aux["isDefault"].as_bool() == Some(true)
            && let (Some(fid), Some(cid)) = (aux["frameId"].as_str(), ctx["id"].as_i64())
        {
            frame_ctx_map.insert(fid.to_string(), cid);
        }
    }

    frame_ctx_map
}

// =============================================================================
// Script runner compute function
// =============================================================================

/// Compute page text against an existing session and return the JSON value.
///
/// Used by the script runner to invoke `page text` without printing to stdout.
///
/// # Errors
///
/// Returns `AppError` on evaluation failure.
pub async fn compute_text(
    managed: &mut agentchrome::connection::ManagedSession,
    args: &PageTextArgs,
) -> Result<serde_json::Value, agentchrome::error::AppError> {
    managed.ensure_domain("Runtime").await?;

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
        .await
        .map_err(|e| agentchrome::error::AppError {
            message: format!("page text evaluation failed: {e}"),
            code: agentchrome::error::ExitCode::GeneralError,
            custom_json: None,
        })?;

    if let Some(exception) = result.get("exceptionDetails") {
        let description = exception["exception"]["description"]
            .as_str()
            .or_else(|| exception["text"].as_str())
            .unwrap_or("unknown error");
        return Err(agentchrome::error::AppError::evaluation_failed(description));
    }

    let value = &result["result"]["value"];
    if let Some(error) = value.get("__error")
        && error.as_str() == Some("not_found")
    {
        let selector = args.selector.as_deref().unwrap_or("unknown");
        return Err(agentchrome::error::AppError::element_not_found(selector));
    }

    let text = value.as_str().unwrap_or_default().to_string();
    let (url, title) = super::get_page_info(managed).await?;

    Ok(serde_json::json!({ "text": text, "url": url, "title": title }))
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
