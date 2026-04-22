/// Expression evaluator for `if` / `while` conditions.
///
/// Evaluates JavaScript expressions via Chrome `Runtime.evaluate` with
/// `$prev`, `$vars`, and `$i` injected as top-level bindings.
use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use crate::script::context::VarContext;

/// Evaluate a JavaScript expression and return a boolean result.
///
/// The expression runs in the page context with the following preamble injected:
/// ```js
/// const $prev = <json>;
/// const $vars = <json>;
/// const $i    = <loop_index>;
/// ```
///
/// Truthiness is decided by JavaScript (`Boolean(expr)`), not by the Rust
/// side — empty arrays and objects are truthy, matching JS `if` semantics.
///
/// # Errors
///
/// Returns `AppError` if the expression throws a JavaScript exception or the
/// CDP call fails.
pub async fn eval_bool(
    managed: &mut ManagedSession,
    expr: &str,
    ctx: &VarContext,
    loop_index: u64,
) -> Result<bool, AppError> {
    let prev_json = serde_json::to_string(&ctx.prev).unwrap_or_else(|_| "null".to_string());
    let vars_json = serde_json::to_string(&ctx.vars).unwrap_or_else(|_| "{}".to_string());

    let full_expr = format!(
        "(() => {{ const $prev = {prev_json}; const $vars = {vars_json}; const $i = {loop_index}; return Boolean({expr}); }})()"
    );

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

    Ok(result["result"]["value"].as_bool().unwrap_or(false))
}
