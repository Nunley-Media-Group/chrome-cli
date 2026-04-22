/// Variable context for script execution.
///
/// Holds `$prev` (previous step output) and `$vars` (bound variables),
/// plus the script's working directory for future relative-path resolution.
use std::collections::HashMap;
use std::path::PathBuf;

use agentchrome::error::{AppError, ExitCode};

// =============================================================================
// VarContext
// =============================================================================

/// In-process variable context threaded through a script run.
#[derive(Debug, Clone)]
pub struct VarContext {
    /// The output of the last non-skipped step (null initially).
    pub prev: serde_json::Value,
    /// Named variables bound by `bind:` on cmd steps.
    pub vars: HashMap<String, serde_json::Value>,
    /// Script's working directory (reserved for future relative-path resolution).
    #[allow(dead_code)]
    pub cwd_script: PathBuf,
}

impl VarContext {
    /// Create a new empty context.
    #[must_use]
    pub fn new(cwd_script: PathBuf) -> Self {
        Self {
            prev: serde_json::Value::Null,
            vars: HashMap::new(),
            cwd_script,
        }
    }

    /// Bind a named variable.
    pub fn bind(&mut self, name: &str, value: serde_json::Value) {
        self.vars.insert(name.to_string(), value);
    }

    /// Update `prev` with the latest step output.
    pub fn set_prev(&mut self, value: serde_json::Value) {
        self.prev = value;
    }
}

// =============================================================================
// Argument substitution
// =============================================================================

/// Error returned when substitution fails.
#[derive(Debug)]
pub enum SubstitutionError {
    /// Referenced variable is not defined.
    Undefined(String),
}

impl From<SubstitutionError> for AppError {
    fn from(e: SubstitutionError) -> Self {
        match e {
            SubstitutionError::Undefined(name) => AppError {
                message: format!("undefined variable: $vars.{name}"),
                code: ExitCode::GeneralError,
                custom_json: None,
            },
        }
    }
}

/// Perform argument substitution on an argv slice.
///
/// Substitution rules (applied per token):
/// - Whole-token `$prev` → serialize `ctx.prev` to JSON string (or unwrap if string).
/// - Whole-token `$vars.<name>` → look up `ctx.vars[name]`; error if missing.
/// - Inline interpolation (`"hello $vars.name"`) is currently treated as whole-token
///   matching; true inline interpolation requires a CDP round-trip and is deferred.
///
/// # Errors
///
/// Returns `SubstitutionError::Undefined` if a `$vars.<name>` reference is missing.
pub fn substitute(argv: &[String], ctx: &VarContext) -> Result<Vec<String>, SubstitutionError> {
    argv.iter().map(|arg| substitute_token(arg, ctx)).collect()
}

fn substitute_token(token: &str, ctx: &VarContext) -> Result<String, SubstitutionError> {
    // Whole-token $prev
    if token == "$prev" {
        return Ok(value_to_string(&ctx.prev));
    }

    // Whole-token $vars.<name>
    if let Some(name) = token.strip_prefix("$vars.") {
        let value = ctx
            .vars
            .get(name)
            .ok_or_else(|| SubstitutionError::Undefined(name.to_string()))?;
        return Ok(value_to_string(value));
    }

    // No substitution needed
    Ok(token.to_string())
}

/// Serialize a JSON value to a string for argument substitution.
///
/// - Strings are unwrapped (returned without quotes).
/// - Everything else is serialized as compact JSON.
fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => "null".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

// =============================================================================
// Unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> VarContext {
        let mut c = VarContext::new(PathBuf::from("/tmp"));
        c.set_prev(serde_json::json!("previous output"));
        c.bind("title", serde_json::json!("Example Domain"));
        c.bind("count", serde_json::json!(42));
        c.bind("obj", serde_json::json!({"key": "value"}));
        c
    }

    #[test]
    fn whole_token_prev_string() {
        let ctx = ctx();
        let argv = vec!["$prev".to_string()];
        let result = substitute(&argv, &ctx).expect("ok");
        assert_eq!(result, vec!["previous output"]);
    }

    #[test]
    fn whole_token_vars_string() {
        let ctx = ctx();
        let argv = vec!["$vars.title".to_string()];
        let result = substitute(&argv, &ctx).expect("ok");
        assert_eq!(result, vec!["Example Domain"]);
    }

    #[test]
    fn whole_token_vars_number() {
        let ctx = ctx();
        let argv = vec!["$vars.count".to_string()];
        let result = substitute(&argv, &ctx).expect("ok");
        assert_eq!(result, vec!["42"]);
    }

    #[test]
    fn whole_token_vars_object_serialized() {
        let ctx = ctx();
        let argv = vec!["$vars.obj".to_string()];
        let result = substitute(&argv, &ctx).expect("ok");
        assert_eq!(result[0], r#"{"key":"value"}"#);
    }

    #[test]
    fn no_substitution_passthrough() {
        let ctx = ctx();
        let argv = vec!["navigate".to_string(), "https://example.com".to_string()];
        let result = substitute(&argv, &ctx).expect("ok");
        assert_eq!(result, argv);
    }

    #[test]
    fn undefined_variable_returns_error() {
        let ctx = ctx();
        let argv = vec!["$vars.does_not_exist".to_string()];
        let err = substitute(&argv, &ctx).expect_err("should fail");
        match err {
            SubstitutionError::Undefined(name) => assert_eq!(name, "does_not_exist"),
        }
    }

    #[test]
    fn prev_null_serializes_as_null() {
        let mut ctx = VarContext::new(PathBuf::from("/tmp"));
        ctx.set_prev(serde_json::Value::Null);
        let argv = vec!["$prev".to_string()];
        let result = substitute(&argv, &ctx).expect("ok");
        assert_eq!(result, vec!["null"]);
    }
}
