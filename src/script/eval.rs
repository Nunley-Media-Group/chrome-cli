/// Expression evaluator for `if` / `while` conditions.
///
/// Evaluates JavaScript expressions via Chrome `Runtime.evaluate` with
/// `$prev`, `$vars`, and `$i` injected as top-level bindings.
use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use crate::script::context::VarContext;

// =============================================================================
// Public API
// =============================================================================

/// Evaluate a JavaScript expression and return a boolean result.
///
/// The expression runs in the page context with the following preamble injected:
/// ```js
/// const $prev = <json>;
/// const $vars = <json>;
/// const $i    = <loop_index>;
/// ```
///
/// # Errors
///
/// Returns `AppError` if:
/// - The expression throws a JavaScript exception.
/// - The CDP call fails.
pub async fn eval_bool(
    managed: &mut ManagedSession,
    expr: &str,
    ctx: &VarContext,
    loop_index: u64,
) -> Result<bool, AppError> {
    let prev_json = serde_json::to_string(&ctx.prev).unwrap_or_else(|_| "null".to_string());
    let vars_json = serde_json::to_string(&ctx.vars).unwrap_or_else(|_| "{}".to_string());

    let preamble =
        format!("const $prev = {prev_json}; const $vars = {vars_json}; const $i = {loop_index};");
    let full_expr = format!("({preamble} ({expr}))");

    managed.ensure_domain("Runtime").await?;

    let params = serde_json::json!({
        "expression": full_expr,
        "returnByValue": true,
        "awaitPromise": false,
    });

    let result = managed
        .send_command("Runtime.evaluate", Some(params))
        .await
        .map_err(|e| AppError {
            message: format!("expression evaluation failed: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    // Check for JavaScript exceptions
    if let Some(exc) = result.get("exceptionDetails") {
        let msg = exc["exception"]["description"]
            .as_str()
            .or_else(|| exc["text"].as_str())
            .unwrap_or("unknown exception");
        return Err(AppError {
            message: format!("script expression threw: {msg}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    // Coerce the result to boolean
    let value = &result["result"]["value"];
    let truthy = is_truthy(value);
    Ok(truthy)
}

/// JavaScript truthiness coercion for a `serde_json::Value`.
fn is_truthy(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => false,
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        serde_json::Value::String(s) => !s.is_empty(),
        serde_json::Value::Array(a) => !a.is_empty(),
        serde_json::Value::Object(o) => !o.is_empty(),
    }
}

// =============================================================================
// Unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_truthy_null() {
        assert!(!is_truthy(&serde_json::Value::Null));
    }

    #[test]
    fn is_truthy_bool() {
        assert!(is_truthy(&serde_json::json!(true)));
        assert!(!is_truthy(&serde_json::json!(false)));
    }

    #[test]
    fn is_truthy_number() {
        assert!(is_truthy(&serde_json::json!(1)));
        assert!(is_truthy(&serde_json::json!(-1)));
        assert!(!is_truthy(&serde_json::json!(0)));
    }

    #[test]
    fn is_truthy_string() {
        assert!(is_truthy(&serde_json::json!("hello")));
        assert!(!is_truthy(&serde_json::json!("")));
    }

    #[test]
    fn is_truthy_array() {
        assert!(is_truthy(&serde_json::json!([1, 2, 3])));
        assert!(!is_truthy(&serde_json::json!([])));
    }

    #[test]
    fn is_truthy_object() {
        assert!(is_truthy(&serde_json::json!({"key": "value"})));
        assert!(!is_truthy(&serde_json::json!({})));
    }
}
