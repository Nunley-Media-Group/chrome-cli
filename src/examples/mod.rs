mod commands;
mod strategies;

use std::fmt::Write;

use agentchrome::error::{AppError, ExitCode};

use crate::cli::{ExamplesArgs, GlobalOpts};
use crate::output::print_output;

/// Append a `<left> — <right>` line (with trailing newline) to `out`.
/// Shared by plain-text summary and strategy-list formatters.
pub(super) fn write_em_dash_line(out: &mut String, left: &str, right: &str) {
    let _ = writeln!(out, "{left} \u{2014} {right}");
}

use commands::{
    CommandGroupSummary, ExampleEntry, all_examples, format_plain_detail, format_plain_summary,
};
use strategies::{
    find_strategy, format_plain_strategy_detail, format_plain_strategy_list, strategy_summaries,
};

// =============================================================================
// Dispatcher
// =============================================================================

pub fn execute_examples(global: &GlobalOpts, args: &ExamplesArgs) -> Result<(), AppError> {
    let is_plain = !global.output.json && !global.output.pretty;

    match args.command.as_deref() {
        None => {
            // Top-level listing: append synthetic "strategies" entry to groups
            let mut groups = all_examples();
            groups.push(CommandGroupSummary {
                command: "strategies".into(),
                description: "Scenario-based interaction strategy guides (iframes, overlays, SCORM, drag-and-drop, and more)".into(),
                examples: vec![
                    ExampleEntry {
                        cmd: "agentchrome examples strategies".into(),
                        description: "List all strategy guides".into(),
                        flags: None,
                    },
                    ExampleEntry {
                        cmd: "agentchrome examples strategies iframes".into(),
                        description: "Show the iframe strategy guide".into(),
                        flags: None,
                    },
                    ExampleEntry {
                        cmd: "agentchrome examples strategies --json".into(),
                        description: "Machine-readable strategy listing".into(),
                        flags: Some(vec!["--json".into()]),
                    },
                ],
            });
            if is_plain {
                print!("{}", format_plain_summary(&groups));
            } else {
                print_output(&groups, &global.output)?;
            }
        }
        Some("strategies") => {
            // Strategy path
            match &args.name {
                None => {
                    // Listing — progressive disclosure (summary only)
                    let summaries = strategy_summaries();
                    if is_plain {
                        print!("{}", format_plain_strategy_list(&summaries));
                    } else {
                        print_output(&summaries, &global.output)?;
                    }
                }
                Some(strategy_name) => {
                    // Detail — full body
                    if let Some(strategy) = find_strategy(strategy_name) {
                        if is_plain {
                            print!("{}", format_plain_strategy_detail(strategy));
                        } else {
                            print_output(strategy, &global.output)?;
                        }
                    } else {
                        let summaries = strategy_summaries();
                        let available: Vec<&str> =
                            summaries.iter().map(|s| s.name.as_str()).collect();
                        return Err(AppError {
                            message: format!(
                                "Unknown strategy: '{}'. Available: {}",
                                strategy_name,
                                available.join(", ")
                            ),
                            code: ExitCode::GeneralError,
                            custom_json: None,
                        });
                    }
                }
            }
        }
        Some(name) => {
            if args.name.is_some() {
                return Err(AppError {
                    message: format!(
                        "Extra argument: '{}' only accepts a second positional when it is 'strategies' (got group '{name}')",
                        "examples"
                    ),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                });
            }
            let groups = all_examples();
            if let Some(g) = groups.into_iter().find(|g| g.command == name) {
                if is_plain {
                    print!("{}", format_plain_detail(&g));
                } else {
                    print_output(&g, &global.output)?;
                }
            } else {
                let all = all_examples();
                let available: Vec<&str> = all.iter().map(|g| g.command.as_str()).collect();
                return Err(AppError {
                    message: format!(
                        "Unknown command group: '{name}'. Available: {}",
                        available.join(", ")
                    ),
                    code: ExitCode::GeneralError,
                    custom_json: None,
                });
            }
        }
    }

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputFormat;

    fn plain_global() -> GlobalOpts {
        GlobalOpts {
            port: None,
            host: "127.0.0.1".into(),
            ws_url: None,
            timeout: None,
            tab: None,
            page_id: None,
            auto_dismiss_dialogs: false,
            config: None,
            keepalive_interval: None,
            no_keepalive: false,
            output: OutputFormat {
                json: false,
                pretty: false,
                plain: false,
                large_response_threshold: None,
            },
        }
    }

    fn make_args(command: Option<&str>, name: Option<&str>) -> ExamplesArgs {
        ExamplesArgs {
            command: command.map(String::from),
            name: name.map(String::from),
        }
    }

    #[test]
    fn execute_examples_unknown_command_returns_error() {
        let global = plain_global();
        let args = make_args(Some("nonexistent"), None);
        let result = execute_examples(&global, &args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unknown command group"));
        assert!(err.message.contains("nonexistent"));
    }

    #[test]
    fn error_message_lists_all_available_groups() {
        let global = plain_global();
        let args = make_args(Some("bogus"), None);
        let err = execute_examples(&global, &args).unwrap_err();
        for group in all_examples() {
            assert!(
                err.message.contains(&group.command),
                "Error message should list '{}' as available",
                group.command
            );
        }
    }

    // T014: dispatcher routing tests

    #[test]
    fn strategy_listing_contains_all_names() {
        let summaries = strategy_summaries();
        let output = format_plain_strategy_list(&summaries);
        let names = [
            "iframes",
            "overlays",
            "scorm",
            "drag-and-drop",
            "shadow-dom",
            "spa-navigation-waits",
            "react-controlled-inputs",
            "debugging-failed-interactions",
            "authentication-cookie-reuse",
            "multi-tab-workflows",
        ];
        for name in &names {
            assert!(
                output.contains(name),
                "Strategy listing missing '{name}'\noutput: {output}"
            );
        }
    }

    #[test]
    fn dispatcher_rejects_extra_positional_for_non_strategies_group() {
        let global = plain_global();
        let args = make_args(Some("navigate"), Some("iframes"));
        let err = execute_examples(&global, &args).unwrap_err();
        assert!(
            err.message.contains("Extra argument"),
            "Error should mention 'Extra argument': {}",
            err.message
        );
        assert!(
            err.message.contains("navigate"),
            "Error should mention the group name: {}",
            err.message
        );
    }

    #[test]
    fn dispatcher_routes_strategies_detail() {
        let strategy = find_strategy("iframes").expect("iframes strategy should exist");
        let output = format_plain_strategy_detail(strategy);
        assert!(
            output.contains("CURRENT CAPABILITIES"),
            "Detail output missing 'CURRENT CAPABILITIES'\noutput: {output}"
        );
        assert!(
            output.contains("RECOMMENDED SEQUENCE"),
            "Detail output missing 'RECOMMENDED SEQUENCE'\noutput: {output}"
        );
    }

    #[test]
    fn dispatcher_unknown_strategy_returns_error() {
        let global = plain_global();
        let args = make_args(Some("strategies"), Some("bogus"));
        let err = execute_examples(&global, &args).unwrap_err();
        assert!(
            err.message.contains("Unknown strategy"),
            "Error should mention 'Unknown strategy': {}",
            err.message
        );
        // All ten strategy names should be listed
        let names = [
            "iframes",
            "overlays",
            "scorm",
            "drag-and-drop",
            "shadow-dom",
            "spa-navigation-waits",
            "react-controlled-inputs",
            "debugging-failed-interactions",
            "authentication-cookie-reuse",
            "multi-tab-workflows",
        ];
        for name in &names {
            assert!(
                err.message.contains(name),
                "Error message should list '{name}': {}",
                err.message
            );
        }
    }

    #[test]
    fn dispatcher_top_level_listing_includes_strategies() {
        // The top-level listing (command == None) should include "strategies" as a group
        let mut groups = all_examples();
        groups.push(CommandGroupSummary {
            command: "strategies".into(),
            description: "Scenario-based interaction strategy guides".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "agentchrome examples strategies".into(),
                    description: "List all strategy guides".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome examples strategies iframes".into(),
                    description: "Show the iframe strategy guide".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "agentchrome examples strategies --json".into(),
                    description: "Machine-readable strategy listing".into(),
                    flags: Some(vec!["--json".into()]),
                },
            ],
        });
        assert!(
            groups.iter().any(|g| g.command == "strategies"),
            "Top-level listing should contain 'strategies' group"
        );
    }

    #[test]
    fn dispatcher_existing_group_behavior_preserved() {
        let group = all_examples()
            .into_iter()
            .find(|g| g.command == "navigate")
            .expect("navigate group must exist");
        let output = format_plain_detail(&group);
        assert!(
            output.contains("navigate"),
            "navigate group detail should contain 'navigate'"
        );
        assert!(
            output.contains("https://example.com"),
            "navigate group detail should contain example URL"
        );
    }

    #[test]
    fn dispatcher_existing_unknown_group_error_preserved() {
        let global = plain_global();
        let args = make_args(Some("nonexistent"), None);
        let err = execute_examples(&global, &args).unwrap_err();
        assert!(
            err.message.contains("Unknown command group"),
            "Error should say 'Unknown command group': {}",
            err.message
        );
    }
}
