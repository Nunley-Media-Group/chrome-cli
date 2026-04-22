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

// =============================================================================
// Runner options
// =============================================================================

/// Options controlling script execution behaviour.
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Stop at the first failing step and return an error.
    pub fail_fast: bool,
    /// Validate without dispatching to Chrome (dry-run mode).
    pub dry_run: bool,
}

// =============================================================================
// Runner entry point
// =============================================================================

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
    let mut ctx = VarContext::new(std::path::PathBuf::from("."));

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

    let executed = results
        .iter()
        .filter(|r| matches!(r.status, StepStatus::Ok))
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r.status, StepStatus::Skipped))
        .count();
    let failed = results
        .iter()
        .filter(|r| matches!(r.status, StepStatus::Error))
        .count();

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

// =============================================================================
// Step execution
// =============================================================================

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

                // Perform argument substitution
                let substituted =
                    crate::script::context::substitute(&cmd_step.cmd, ctx).map_err(|e| {
                        let err: AppError = e.into();
                        err
                    });

                let (status, output, error_msg) = match substituted {
                    Err(sub_err) => {
                        let msg = sub_err.message.clone();
                        (StepStatus::Error, None, Some(msg))
                    }
                    Ok(argv) if opts.dry_run => {
                        // Dry-run: just validate the subcommand name
                        let sub = argv.first().map_or("", String::as_str);
                        if crate::script::dispatch::is_known_subcommand(sub) {
                            (
                                StepStatus::Ok,
                                Some(serde_json::json!({"dry_run": true})),
                                None,
                            )
                        } else {
                            let msg = format!("unknown subcommand: '{sub}'");
                            (StepStatus::Error, None, Some(msg))
                        }
                    }
                    Ok(argv) => {
                        match invoke(&argv, ctx, client, managed, global).await {
                            Ok(value) => {
                                // Update $prev
                                ctx.set_prev(value.clone());
                                // Bind to $vars if requested
                                if let Some(bind_name) = &cmd_step.bind {
                                    ctx.bind(bind_name, value.clone());
                                }
                                (StepStatus::Ok, Some(value), None)
                            }
                            Err(e) => {
                                let msg = e.message.clone();
                                (StepStatus::Error, None, Some(msg))
                            }
                        }
                    }
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
                    return Err(AppError {
                        message: format!(
                            "script step {current_index} failed: {}",
                            err_clone.unwrap_or_default()
                        ),
                        code: ExitCode::GeneralError,
                        custom_json: None,
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

                // Execute active branch
                for sub_step in active_branch {
                    execute_step(
                        sub_step, client, managed, global, opts, ctx, results, index, loop_index,
                    )
                    .await?;
                }

                // Emit skipped entries for inactive branch
                for sub_step in skipped_branch {
                    emit_skipped_step(sub_step, results, index, loop_index);
                }
            }

            Step::Loop(loop_step) => {
                match &loop_step.r#loop {
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
                                // Emit max-iterations warning
                                let warn = serde_json::json!({
                                    "warning": "loop max iterations reached",
                                    "max": while_loop.max
                                });
                                let warn_str = serde_json::to_string(&warn).unwrap_or_default();
                                eprintln!("{warn_str}");
                                break;
                            }

                            let cond =
                                eval_bool(managed, &while_loop.r#while, ctx, iterations).await?;
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
                }
            }
        }

        Ok(())
    }) // end Box::pin
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
            // Recursively skip all nested steps
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

// =============================================================================
// Dry-run validation
// =============================================================================

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

// =============================================================================
// Unit tests
// =============================================================================

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
