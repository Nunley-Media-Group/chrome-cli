/// Script v1 JSON schema types and parser.
///
/// The schema is:
/// ```json
/// {
///   "commands": [
///     { "cmd": ["navigate", "https://example.com"] },
///     { "cmd": ["js", "exec", "document.title"], "bind": "title" },
///     { "if": "$vars.title.includes('Example')",
///       "then": [{ "cmd": [...] }],
///       "else": [{ "cmd": [...] }] },
///     { "loop": { "count": 3 }, "body": [{ "cmd": [...] }] }
///   ]
/// }
/// ```
use agentchrome::error::{AppError, ExitCode};
use serde::{Deserialize, Serialize};

/// Top-level script container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    /// Ordered list of steps to execute.
    pub commands: Vec<Step>,
}

/// A single script step — one of: cmd, if/then/else, or loop/body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Step {
    /// Command invocation step.
    Cmd(CmdStep),
    /// Conditional branch step.
    If(IfStep),
    /// Loop step.
    Loop(LoopStep),
}

/// A step that invokes an agentchrome subcommand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdStep {
    /// Argv-style command: first element is the subcommand name
    /// (e.g. `["navigate", "https://example.com"]`).
    pub cmd: Vec<String>,
    /// Optional variable to bind the command's output to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind: Option<String>,
}

/// A conditional branch step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfStep {
    /// JavaScript expression evaluated via CDP `Runtime.evaluate`.
    /// `$prev` and `$vars` are injected as top-level bindings.
    pub r#if: String,
    /// Steps to execute when the condition is truthy.
    pub then: Vec<Step>,
    /// Steps to execute when the condition is falsy (default: empty).
    #[serde(default)]
    pub r#else: Vec<Step>,
}

/// A loop step with a count or while-condition body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStep {
    /// Loop control: `{ "count": N }` or `{ "while": "<expr>", "max": N }`.
    pub r#loop: LoopKind,
    /// Body steps executed each iteration.
    pub body: Vec<Step>,
}

/// Loop control variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LoopKind {
    /// Execute body exactly `count` times.
    Count(CountLoop),
    /// Execute body while expression is truthy, up to `max` iterations.
    While(WhileLoop),
}

/// Count-based loop control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountLoop {
    /// Number of iterations (>= 0).
    pub count: u64,
}

/// Condition-based loop control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhileLoop {
    /// JavaScript expression evaluated before each iteration.
    pub r#while: String,
    /// Maximum number of iterations (>= 1). Required for while loops.
    pub max: u64,
}

/// Parse raw JSON bytes into a validated `Script`.
///
/// # Errors
///
/// Returns `AppError` if the JSON is malformed, required fields are missing,
/// or schema invariants are violated.
pub fn parse_script(bytes: &[u8]) -> Result<Script, AppError> {
    let script: Script = serde_json::from_slice(bytes).map_err(|e| AppError {
        message: format!("script parse error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    validate_script(&script)?;
    Ok(script)
}

/// Validate a parsed script for schema invariants.
fn validate_script(script: &Script) -> Result<(), AppError> {
    if script.commands.is_empty() {
        return Err(AppError {
            message: "script 'commands' array must not be empty".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }
    for (i, step) in script.commands.iter().enumerate() {
        validate_step(step, i)?;
    }
    Ok(())
}

fn validate_step(step: &Step, index: usize) -> Result<(), AppError> {
    match step {
        Step::Cmd(cmd_step) => {
            if cmd_step.cmd.is_empty() {
                return Err(AppError {
                    message: format!("script step {index}: 'cmd' array must not be empty"),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                });
            }
            if let Some(name) = &cmd_step.bind {
                if !is_valid_identifier(name) {
                    return Err(AppError {
                        message: format!(
                            "script step {index}: 'bind' name '{name}' is not a valid identifier \
                             (must match [a-zA-Z_][a-zA-Z0-9_]*)"
                        ),
                        code: ExitCode::GeneralError,
                        custom_json: None,
                    });
                }
            }
        }
        Step::If(if_step) => {
            if if_step.r#if.trim().is_empty() {
                return Err(AppError {
                    message: format!("script step {index}: 'if' expression must not be empty"),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                });
            }
            for (j, sub) in if_step.then.iter().enumerate() {
                validate_step(sub, j)?;
            }
            for (j, sub) in if_step.r#else.iter().enumerate() {
                validate_step(sub, j)?;
            }
        }
        Step::Loop(loop_step) => {
            match &loop_step.r#loop {
                LoopKind::While(wl) => {
                    if wl.r#while.trim().is_empty() {
                        return Err(AppError {
                            message: format!(
                                "script step {index}: 'while' expression must not be empty"
                            ),
                            code: ExitCode::GeneralError,
                            custom_json: None,
                        });
                    }
                    if wl.max == 0 {
                        return Err(AppError {
                            message: format!(
                                "script step {index}: 'max' must be >= 1 for while loops"
                            ),
                            code: ExitCode::GeneralError,
                            custom_json: None,
                        });
                    }
                }
                LoopKind::Count(_) => {}
            }
            if loop_step.body.is_empty() {
                return Err(AppError {
                    message: format!("script step {index}: 'body' array must not be empty"),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                });
            }
            for (j, sub) in loop_step.body.iter().enumerate() {
                validate_step(sub, j)?;
            }
        }
    }
    Ok(())
}

/// Check whether a string is a valid Rust/JavaScript identifier.
fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(s: &str) -> Vec<u8> {
        s.as_bytes().to_vec()
    }

    #[test]
    fn parse_cmd_step() {
        let bytes = json(r#"{"commands":[{"cmd":["navigate","https://example.com"]}]}"#);
        let script = parse_script(&bytes).expect("should parse");
        assert_eq!(script.commands.len(), 1);
        match &script.commands[0] {
            Step::Cmd(c) => {
                assert_eq!(c.cmd, vec!["navigate", "https://example.com"]);
                assert!(c.bind.is_none());
            }
            _ => panic!("expected Cmd"),
        }
    }

    #[test]
    fn parse_cmd_step_with_bind() {
        let bytes = json(r#"{"commands":[{"cmd":["js","exec","document.title"],"bind":"title"}]}"#);
        let script = parse_script(&bytes).expect("should parse");
        match &script.commands[0] {
            Step::Cmd(c) => assert_eq!(c.bind.as_deref(), Some("title")),
            _ => panic!("expected Cmd"),
        }
    }

    #[test]
    fn parse_if_step() {
        let bytes = json(
            r#"{"commands":[{"if":"true","then":[{"cmd":["navigate","https://a.com"]}],"else":[]}]}"#,
        );
        let script = parse_script(&bytes).expect("should parse");
        match &script.commands[0] {
            Step::If(i) => {
                assert_eq!(i.r#if, "true");
                assert_eq!(i.then.len(), 1);
                assert!(i.r#else.is_empty());
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_count_loop() {
        let bytes = json(
            r#"{"commands":[{"loop":{"count":3},"body":[{"cmd":["navigate","https://a.com"]}]}]}"#,
        );
        let script = parse_script(&bytes).expect("should parse");
        match &script.commands[0] {
            Step::Loop(l) => match &l.r#loop {
                LoopKind::Count(c) => assert_eq!(c.count, 3),
                LoopKind::While(_) => panic!("expected Count"),
            },
            _ => panic!("expected Loop"),
        }
    }

    #[test]
    fn parse_while_loop() {
        let bytes = json(
            r#"{"commands":[{"loop":{"while":"true","max":10},"body":[{"cmd":["navigate","https://a.com"]}]}]}"#,
        );
        let script = parse_script(&bytes).expect("should parse");
        match &script.commands[0] {
            Step::Loop(l) => match &l.r#loop {
                LoopKind::While(w) => {
                    assert_eq!(w.r#while, "true");
                    assert_eq!(w.max, 10);
                }
                LoopKind::Count(_) => panic!("expected While"),
            },
            _ => panic!("expected Loop"),
        }
    }

    #[test]
    fn reject_empty_commands() {
        let bytes = json(r#"{"commands":[]}"#);
        let err = parse_script(&bytes).expect_err("should fail");
        assert!(err.message.contains("must not be empty"));
    }

    #[test]
    fn reject_empty_cmd_array() {
        let bytes = json(r#"{"commands":[{"cmd":[]}]}"#);
        let err = parse_script(&bytes).expect_err("should fail");
        assert!(err.message.contains("'cmd' array must not be empty"));
    }

    #[test]
    fn reject_while_without_max() {
        let bytes =
            json(r#"{"commands":[{"loop":{"while":"true"},"body":[{"cmd":["navigate","x"]}]}]}"#);
        let err = parse_script(&bytes).expect_err("should fail");
        assert!(!err.message.is_empty());
    }

    #[test]
    fn reject_while_with_max_zero() {
        let bytes = json(
            r#"{"commands":[{"loop":{"while":"true","max":0},"body":[{"cmd":["navigate","x"]}]}]}"#,
        );
        let err = parse_script(&bytes).expect_err("should fail");
        assert!(err.message.contains("max"));
    }

    #[test]
    fn reject_empty_body() {
        let bytes = json(r#"{"commands":[{"loop":{"count":3},"body":[]}]}"#);
        let err = parse_script(&bytes).expect_err("should fail");
        assert!(err.message.contains("'body' array must not be empty"));
    }

    #[test]
    fn reject_invalid_bind_name() {
        let bytes = json(r#"{"commands":[{"cmd":["js","exec","document.title"],"bind":"1bad"}]}"#);
        let err = parse_script(&bytes).expect_err("should fail");
        assert!(err.message.contains("valid identifier"));
    }

    #[test]
    fn valid_identifier_check() {
        assert!(is_valid_identifier("title"));
        assert!(is_valid_identifier("_hidden"));
        assert!(is_valid_identifier("my_var2"));
        assert!(!is_valid_identifier("1bad"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("bad-name"));
    }
}
