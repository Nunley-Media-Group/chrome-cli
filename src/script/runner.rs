/// Sequential script runner with if/else and loop support.
///
/// Walks a `Script`'s command list, dispatching each step to the appropriate
/// command module adapter and accumulating a `RunReport`.
use std::time::Instant;

use agentchrome::cdp::CdpClient;
use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use crate::cli::GlobalOpts;
use crate::script::context::VarContext;
use crate::script::dispatch::invoke;
use crate::script::eval::eval_bool;
use crate::script::parser::{LoopKind, Script, Step};
use crate::script::result::{RunReport, StepResult, StepStatus};

/// Options controlling script execution behaviour.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Stop at the first failing step and return an error.
    pub fail_fast: bool,
    /// Validate without dispatching to Chrome (dry-run mode).
    pub dry_run: bool,
}

/// Execute a script and return a `RunReport`.
///
/// # Errors
///
/// Under `fail_fast`, returns `AppError` as soon as a step fails.
/// Without `fail_fast`, errors are collected in `results[i]` but the function
/// returns `Ok` when the run finishes (even if some steps failed).
pub async fn run_script(
    script: &Script,
    client: &CdpClient,
    managed: &mut ManagedSession,
    global: &GlobalOpts,
    opts: &RunOptions,
) -> Result<RunReport, AppError> {
    let total_start = Instant::now();
    let mut results: Vec<StepResult> = Vec::new();
    let mut ctx = VarContext::new();

    let mut index = 0usize;

    for step in &script.commands {
        execute_step(
            step,
            client,
            managed,
            global,
            opts,
            &mut ctx,
            &mut results,
            &mut index,
            None,
        )
        .await?;
    }

    let (mut executed, mut skipped, mut failed) = (0usize, 0usize, 0usize);
    for r in &results {
        match r.status {
            StepStatus::Ok => executed += 1,
            StepStatus::Skipped => skipped += 1,
            StepStatus::Error => failed += 1,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    let total_ms = total_start.elapsed().as_millis() as u64;

    Ok(RunReport {
        results,
        executed,
        skipped,
        failed,
        total_ms,
    })
}

/// Execute a single step, recursing for if/loop branches.
///
/// # Errors
///
/// Returns `AppError` under `fail_fast` when a step errors.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn execute_step<'a>(
    step: &'a Step,
    client: &'a CdpClient,
    managed: &'a mut ManagedSession,
    global: &'a GlobalOpts,
    opts: &'a RunOptions,
    ctx: &'a mut VarContext,
    results: &'a mut Vec<StepResult>,
    index: &'a mut usize,
    loop_index: Option<u64>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), AppError>> + Send + 'a>> {
    Box::pin(async move {
        match step {
            Step::Cmd(cmd_step) => {
                let current_index = *index;
                *index += 1;
                let step_start = Instant::now();

                let substituted =
                    crate::script::context::substitute(&cmd_step.cmd, ctx).map_err(AppError::from);

                let (status, output, error_msg) = match substituted {
                    Err(sub_err) => (StepStatus::Error, None, Some(sub_err.message)),
                    Ok(argv) if opts.dry_run => {
                        let sub = argv.first().map_or("", String::as_str);
                        if crate::script::dispatch::is_known_subcommand(sub) {
                            (
                                StepStatus::Ok,
                                Some(serde_json::json!({"dry_run": true})),
                                None,
                            )
                        } else {
                            (
                                StepStatus::Error,
                                None,
                                Some(format!("unknown subcommand: '{sub}'")),
                            )
                        }
                    }
                    Ok(argv) => match invoke(&argv, ctx, client, managed, global).await {
                        Ok(value) => {
                            if let Some(bind_name) = &cmd_step.bind {
                                ctx.bind(bind_name, bind_value_for(&argv, &value));
                            }
                            ctx.set_prev(value.clone());
                            (StepStatus::Ok, Some(value), None)
                        }
                        Err(e) => (StepStatus::Error, None, Some(e.message)),
                    },
                };

                #[allow(clippy::cast_possible_truncation)]
                let duration_ms = step_start.elapsed().as_millis() as u64;

                let is_error = matches!(status, StepStatus::Error);
                let err_clone = error_msg.clone();

                results.push(StepResult {
                    index: current_index,
                    command: Some(cmd_step.cmd.clone()),
                    status,
                    output,
                    error: error_msg,
                    duration_ms,
                    loop_index,
                });

                if is_error && opts.fail_fast {
                    let message = format!(
                        "script step {current_index} failed: {}",
                        err_clone.unwrap_or_default()
                    );
                    let custom = serde_json::json!({
                        "error": message,
                        "code": ExitCode::GeneralError as u8,
                        "failing_index": current_index,
                        "failing_command": cmd_step.cmd,
                    });
                    return Err(AppError {
                        message,
                        code: ExitCode::GeneralError,
                        custom_json: Some(custom.to_string()),
                    });
                }
            }

            Step::If(if_step) => {
                let cond = eval_bool(managed, &if_step.r#if, ctx, loop_index.unwrap_or(0)).await?;

                let (active_branch, skipped_branch) = if cond {
                    (&if_step.then, &if_step.r#else)
                } else {
                    (&if_step.r#else, &if_step.then)
                };

                for sub_step in active_branch {
                    execute_step(
                        sub_step, client, managed, global, opts, ctx, results, index, loop_index,
                    )
                    .await?;
                }
                for sub_step in skipped_branch {
                    emit_skipped_step(sub_step, results, index, loop_index);
                }
            }

            Step::Loop(loop_step) => match &loop_step.r#loop {
                LoopKind::Count(count_loop) => {
                    for i in 0..count_loop.count {
                        for sub_step in &loop_step.body {
                            execute_step(
                                sub_step,
                                client,
                                managed,
                                global,
                                opts,
                                ctx,
                                results,
                                index,
                                Some(i),
                            )
                            .await?;
                        }
                    }
                }
                LoopKind::While(while_loop) => {
                    let mut iterations = 0u64;
                    loop {
                        if iterations >= while_loop.max {
                            let warn = serde_json::json!({
                                "warning": "loop max iterations reached",
                                "max": while_loop.max
                            });
                            let warn_str = serde_json::to_string(&warn).unwrap_or_default();
                            eprintln!("{warn_str}");
                            break;
                        }

                        let cond = eval_bool(managed, &while_loop.r#while, ctx, iterations).await?;
                        if !cond {
                            break;
                        }

                        for sub_step in &loop_step.body {
                            execute_step(
                                sub_step,
                                client,
                                managed,
                                global,
                                opts,
                                ctx,
                                results,
                                index,
                                Some(iterations),
                            )
                            .await?;
                        }
                        iterations += 1;
                    }
                }
            },
        }

        Ok(())
    })
}

/// Compute the value stored in `$vars` when a step has a `bind`.
///
/// `js exec` wraps its result in a `{result, type, truncated}` envelope on the
/// wire, but script authors naturally expect `$vars.<bind>` to hold the
/// underlying JS value. Unwrap the envelope at the bind site for `js exec` only
/// — other commands keep their returned shape intact.
fn bind_value_for(cmd: &[String], value: &serde_json::Value) -> serde_json::Value {
    if cmd.len() >= 2
        && cmd[0] == "js"
        && cmd[1] == "exec"
        && let Some(inner) = value.get("result")
    {
        return inner.clone();
    }
    value.clone()
}

/// Emit a `skipped` result entry for a step that was not selected by an `if` branch.
fn emit_skipped_step(
    step: &Step,
    results: &mut Vec<StepResult>,
    index: &mut usize,
    loop_index: Option<u64>,
) {
    match step {
        Step::Cmd(cmd_step) => {
            results.push(StepResult {
                index: *index,
                command: Some(cmd_step.cmd.clone()),
                status: StepStatus::Skipped,
                output: None,
                error: None,
                duration_ms: 0,
                loop_index,
            });
            *index += 1;
        }
        Step::If(if_step) => {
            for sub in &if_step.then {
                emit_skipped_step(sub, results, index, loop_index);
            }
            for sub in &if_step.r#else {
                emit_skipped_step(sub, results, index, loop_index);
            }
        }
        Step::Loop(loop_step) => {
            for sub in &loop_step.body {
                emit_skipped_step(sub, results, index, loop_index);
            }
        }
    }
}

/// Validate a script's schema and subcommand names without dispatching.
///
/// Returns a count of valid steps, or an error describing the first invalid step.
///
/// # Errors
///
/// Returns `AppError` if any step references an unknown subcommand.
pub fn validate_dry_run(script: &Script) -> Result<usize, AppError> {
    let mut count = 0usize;
    for step in &script.commands {
        validate_dry_step(step, &mut count)?;
    }
    Ok(count)
}

fn validate_dry_step(step: &Step, count: &mut usize) -> Result<(), AppError> {
    match step {
        Step::Cmd(cmd_step) => {
            let sub = cmd_step.cmd.first().map(String::as_str).unwrap_or_default();
            if !crate::script::dispatch::is_known_subcommand(sub) {
                return Err(AppError {
                    message: format!("dry-run: unknown subcommand '{sub}' in script step {count}"),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                });
            }
            *count += 1;
        }
        Step::If(if_step) => {
            for s in &if_step.then {
                validate_dry_step(s, count)?;
            }
            for s in &if_step.r#else {
                validate_dry_step(s, count)?;
            }
        }
        Step::Loop(loop_step) => {
            for s in &loop_step.body {
                validate_dry_step(s, count)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::parser::parse_script;
    use crate::script::result::DryRunReport;

    #[test]
    fn dry_run_valid_script() {
        let bytes = br#"{"commands":[{"cmd":["navigate","https://example.com"]},{"cmd":["js","exec","document.title"]}]}"#;
        let script = parse_script(bytes).expect("should parse");
        let count = validate_dry_run(&script).expect("should be valid");
        assert_eq!(count, 2);
    }

    #[test]
    fn dry_run_unknown_subcommand() {
        let bytes = br#"{"commands":[{"cmd":["unknown_cmd","arg"]}]}"#;
        let script = parse_script(bytes).expect("should parse");
        let err = validate_dry_run(&script).expect_err("should fail");
        assert!(err.message.contains("unknown subcommand"));
        assert!(err.message.contains("unknown_cmd"));
    }

    #[test]
    fn bind_value_for_js_exec_scalar_unwraps_result() {
        let cmd = vec![
            "js".to_string(),
            "exec".to_string(),
            "document.title".to_string(),
        ];
        let envelope = serde_json::json!({
            "result": "The Internet",
            "type": "string",
            "truncated": false
        });
        let bound = bind_value_for(&cmd, &envelope);
        assert_eq!(bound, serde_json::Value::String("The Internet".to_string()));
    }

    #[test]
    fn bind_value_for_js_exec_object_unwraps_result() {
        let cmd = vec![
            "js".to_string(),
            "exec".to_string(),
            "({a:1,b:2})".to_string(),
        ];
        let envelope = serde_json::json!({
            "result": {"a": 1, "b": 2},
            "type": "object",
            "truncated": false
        });
        let bound = bind_value_for(&cmd, &envelope);
        assert_eq!(bound, serde_json::json!({"a": 1, "b": 2}));
        assert!(bound.get("truncated").is_none());
    }

    #[test]
    fn bind_value_for_page_find_passes_through_array() {
        let cmd = vec!["page".to_string(), "find".to_string(), "Submit".to_string()];
        let value = serde_json::json!([{"uid": "u-1", "role": "button", "name": "Submit"}]);
        let bound = bind_value_for(&cmd, &value);
        assert_eq!(bound, value);
    }

    #[test]
    fn bind_value_for_navigate_passes_through_object() {
        let cmd = vec!["navigate".to_string(), "https://example.com".to_string()];
        let value = serde_json::json!({"url": "https://example.com", "title": "Example"});
        let bound = bind_value_for(&cmd, &value);
        assert_eq!(bound, value);
    }

    #[test]
    fn bind_value_for_js_without_exec_passes_through() {
        let cmd = vec!["js".to_string(), "help".to_string()];
        let value = serde_json::json!({"result": "something"});
        let bound = bind_value_for(&cmd, &value);
        assert_eq!(bound, value);
    }

    #[test]
    fn dry_run_report_structure() {
        let report = DryRunReport {
            dispatched: false,
            ok: true,
            steps: 3,
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["dispatched"], false);
        assert_eq!(json["ok"], true);
        assert_eq!(json["steps"], 3);
    }
}
