use std::fmt::Write;

use serde::Serialize;

use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{ExamplesArgs, GlobalOpts};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct CommandGroupSummary {
    command: String,
    description: String,
    examples: Vec<ExampleEntry>,
}

#[derive(Serialize)]
struct ExampleEntry {
    cmd: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<String>>,
}

// =============================================================================
// Static example data
// =============================================================================

#[allow(clippy::too_many_lines)]
fn all_examples() -> Vec<CommandGroupSummary> {
    vec![
        CommandGroupSummary {
            command: "connect".into(),
            description: "Connect to or launch a Chrome instance".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli connect".into(),
                    description: "Connect to Chrome on the default port (9222)".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli connect --launch --headless".into(),
                    description: "Launch a new headless Chrome instance".into(),
                    flags: Some(vec!["--launch".into(), "--headless".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli connect --port 9333".into(),
                    description: "Connect to Chrome on a specific port".into(),
                    flags: Some(vec!["--port".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli connect --status".into(),
                    description: "Check current connection status".into(),
                    flags: Some(vec!["--status".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli connect --disconnect".into(),
                    description: "Disconnect and remove the session file".into(),
                    flags: Some(vec!["--disconnect".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "tabs".into(),
            description: "Tab management (list, create, close, activate)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli tabs list".into(),
                    description: "List all open tabs".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli tabs create https://example.com".into(),
                    description: "Open a new tab with a URL".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli tabs close ABC123".into(),
                    description: "Close a tab by its ID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli tabs activate ABC123".into(),
                    description: "Activate (focus) a tab by its ID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli tabs list --all".into(),
                    description: "List all tabs including internal Chrome pages".into(),
                    flags: Some(vec!["--all".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "navigate".into(),
            description: "URL navigation and history".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli navigate https://example.com".into(),
                    description: "Navigate to a URL and wait for load".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli navigate https://app.example.com --wait-until networkidle"
                        .into(),
                    description: "Navigate and wait for network idle (for SPAs)".into(),
                    flags: Some(vec!["--wait-until".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli navigate back".into(),
                    description: "Go back in browser history".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli navigate reload --ignore-cache".into(),
                    description: "Reload the page without cache".into(),
                    flags: Some(vec!["--ignore-cache".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "page".into(),
            description: "Page inspection (screenshot, text, accessibility tree, find)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli page text".into(),
                    description: "Extract all visible text from the page".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli page snapshot".into(),
                    description: "Capture the accessibility tree with element UIDs".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli page screenshot --full-page --file page.png".into(),
                    description: "Take a full-page screenshot".into(),
                    flags: Some(vec!["--full-page".into(), "--file".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli page find \"Sign in\"".into(),
                    description: "Find elements by text".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli page resize 1280x720".into(),
                    description: "Resize the viewport to specific dimensions".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "dom".into(),
            description: "DOM inspection and manipulation (not yet implemented)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli page snapshot".into(),
                    description: "Use page snapshot as an alternative to DOM queries".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli js exec \"document.querySelector('#myId').textContent\""
                        .into(),
                    description: "Use js exec to query DOM elements".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli js exec \"document.querySelectorAll('a').length\"".into(),
                    description: "Count elements matching a selector".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "js".into(),
            description: "JavaScript execution in page context".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli js exec \"document.title\"".into(),
                    description: "Get the page title".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli js exec --file script.js".into(),
                    description: "Execute a JavaScript file".into(),
                    flags: Some(vec!["--file".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli js exec --uid s3 \"(el) => el.textContent\"".into(),
                    description: "Run code on a specific element by UID".into(),
                    flags: Some(vec!["--uid".into()]),
                },
                ExampleEntry {
                    cmd: "echo 'document.URL' | chrome-cli js exec -".into(),
                    description: "Read JavaScript from stdin".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "console".into(),
            description: "Console message reading and monitoring".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli console read".into(),
                    description: "Read recent console messages".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli console read --errors-only".into(),
                    description: "Show only error messages".into(),
                    flags: Some(vec!["--errors-only".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli console follow".into(),
                    description: "Stream console messages in real time".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli console follow --errors-only --timeout 10000".into(),
                    description: "Stream errors for 10 seconds".into(),
                    flags: Some(vec!["--errors-only".into(), "--timeout".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "network".into(),
            description: "Network request monitoring and interception".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli network list".into(),
                    description: "List recent network requests".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli network list --type xhr,fetch".into(),
                    description: "Filter requests by resource type".into(),
                    flags: Some(vec!["--type".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli network get 42".into(),
                    description: "Get details of a specific request by ID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli network follow --url api.example.com".into(),
                    description: "Stream network requests matching a URL pattern".into(),
                    flags: Some(vec!["--url".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "interact".into(),
            description: "Mouse, keyboard, and scroll interactions".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli interact click s5".into(),
                    description: "Click an element by UID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli interact click css:#submit-btn".into(),
                    description: "Click an element by CSS selector".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli interact type \"Hello, world!\"".into(),
                    description: "Type text into the focused element".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli interact key Control+A".into(),
                    description: "Press a key combination".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli interact scroll --to-bottom".into(),
                    description: "Scroll to the bottom of the page".into(),
                    flags: Some(vec!["--to-bottom".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "form".into(),
            description: "Form input and submission".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli form fill s5 \"hello@example.com\"".into(),
                    description: "Fill a form field by UID".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli form fill css:#email \"user@example.com\"".into(),
                    description: "Fill a form field by CSS selector".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli form clear s5".into(),
                    description: "Clear a form field".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli form upload s10 ./photo.jpg".into(),
                    description: "Upload a file to a file input element".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "emulate".into(),
            description: "Device and network emulation".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli emulate set --viewport 375x667 --device-scale 2 --mobile"
                        .into(),
                    description: "Emulate a mobile device".into(),
                    flags: Some(vec![
                        "--viewport".into(),
                        "--device-scale".into(),
                        "--mobile".into(),
                    ]),
                },
                ExampleEntry {
                    cmd: "chrome-cli emulate set --network 3g".into(),
                    description: "Simulate slow 3G network".into(),
                    flags: Some(vec!["--network".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli emulate set --color-scheme dark".into(),
                    description: "Force dark mode".into(),
                    flags: Some(vec!["--color-scheme".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli emulate status".into(),
                    description: "Check current emulation settings".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli emulate reset".into(),
                    description: "Clear all emulation overrides".into(),
                    flags: None,
                },
            ],
        },
        CommandGroupSummary {
            command: "perf".into(),
            description: "Performance tracing and metrics".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli perf vitals".into(),
                    description: "Quick Core Web Vitals measurement".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli perf record --duration 5000".into(),
                    description: "Record a trace for 5 seconds".into(),
                    flags: Some(vec!["--duration".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli perf record --reload --duration 5000".into(),
                    description: "Record a trace with page reload".into(),
                    flags: Some(vec!["--reload".into(), "--duration".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli perf analyze RenderBlocking --trace-file trace.json".into(),
                    description: "Analyze render-blocking resources from a trace".into(),
                    flags: Some(vec!["--trace-file".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "dialog".into(),
            description: "Browser dialog handling (alert, confirm, prompt, beforeunload)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli dialog info".into(),
                    description: "Check if a dialog is currently open".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli dialog handle accept".into(),
                    description: "Accept an alert or confirm dialog".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli dialog handle dismiss".into(),
                    description: "Dismiss a dialog".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli dialog handle accept --text \"my input\"".into(),
                    description: "Accept a prompt dialog with text".into(),
                    flags: Some(vec!["--text".into()]),
                },
            ],
        },
        CommandGroupSummary {
            command: "config".into(),
            description: "Configuration file management (show, init, path)".into(),
            examples: vec![
                ExampleEntry {
                    cmd: "chrome-cli config show".into(),
                    description: "Show the resolved configuration from all sources".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli config init".into(),
                    description: "Create a default config file".into(),
                    flags: None,
                },
                ExampleEntry {
                    cmd: "chrome-cli config init --path ./my-config.toml".into(),
                    description: "Create a config at a custom path".into(),
                    flags: Some(vec!["--path".into()]),
                },
                ExampleEntry {
                    cmd: "chrome-cli config path".into(),
                    description: "Show the active config file path".into(),
                    flags: None,
                },
            ],
        },
    ]
}

// =============================================================================
// Output formatting
// =============================================================================

fn print_output(value: &impl Serialize, output: &crate::cli::OutputFormat) -> Result<(), AppError> {
    let json = if output.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };
    let json = json.map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    println!("{json}");
    Ok(())
}

fn format_plain_summary(groups: &[CommandGroupSummary]) -> String {
    let mut out = String::new();
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let _ = writeln!(out, "{} \u{2014} {}", group.command, group.description);
        if let Some(first) = group.examples.first() {
            let _ = writeln!(out, "  {}", first.cmd);
        }
    }
    out
}

fn format_plain_detail(group: &CommandGroupSummary) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} \u{2014} {}", group.command, group.description);
    for example in &group.examples {
        out.push('\n');
        let _ = writeln!(out, "  # {}", example.description);
        let _ = writeln!(out, "  {}", example.cmd);
    }
    out
}

// =============================================================================
// Dispatcher
// =============================================================================

pub fn execute_examples(global: &GlobalOpts, args: &ExamplesArgs) -> Result<(), AppError> {
    let groups = all_examples();
    let is_plain = !global.output.json && !global.output.pretty;

    match &args.command {
        None => {
            if is_plain {
                print!("{}", format_plain_summary(&groups));
            } else {
                print_output(&groups, &global.output)?;
            }
        }
        Some(name) => {
            if let Some(g) = groups.into_iter().find(|g| g.command == *name) {
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

    #[test]
    fn all_examples_returns_expected_groups() {
        let groups = all_examples();
        let names: Vec<&str> = groups.iter().map(|g| g.command.as_str()).collect();
        assert!(names.contains(&"connect"));
        assert!(names.contains(&"tabs"));
        assert!(names.contains(&"navigate"));
        assert!(names.contains(&"page"));
        assert!(names.contains(&"dom"));
        assert!(names.contains(&"js"));
        assert!(names.contains(&"console"));
        assert!(names.contains(&"network"));
        assert!(names.contains(&"interact"));
        assert!(names.contains(&"form"));
        assert!(names.contains(&"emulate"));
        assert!(names.contains(&"perf"));
        assert!(names.contains(&"dialog"));
        assert!(names.contains(&"config"));
    }

    #[test]
    fn each_group_has_at_least_3_examples() {
        for group in all_examples() {
            assert!(
                group.examples.len() >= 3,
                "Group '{}' has only {} examples, expected at least 3",
                group.command,
                group.examples.len()
            );
        }
    }

    #[test]
    fn no_empty_fields() {
        for group in all_examples() {
            assert!(!group.command.is_empty());
            assert!(!group.description.is_empty());
            for example in &group.examples {
                assert!(
                    !example.cmd.is_empty(),
                    "Empty cmd in group '{}'",
                    group.command
                );
                assert!(
                    !example.description.is_empty(),
                    "Empty description in group '{}'",
                    group.command
                );
            }
        }
    }

    #[test]
    fn plain_summary_contains_all_groups() {
        let groups = all_examples();
        let output = format_plain_summary(&groups);
        for group in &groups {
            assert!(
                output.contains(&group.command),
                "Summary missing group '{}'",
                group.command
            );
        }
    }

    #[test]
    fn plain_summary_does_not_start_with_json() {
        let groups = all_examples();
        let output = format_plain_summary(&groups);
        assert!(!output.starts_with('['));
        assert!(!output.starts_with('{'));
    }

    #[test]
    fn plain_detail_contains_descriptions_and_commands() {
        let groups = all_examples();
        let group = groups.iter().find(|g| g.command == "navigate").unwrap();
        let output = format_plain_detail(group);
        assert!(output.contains("navigate"));
        for example in &group.examples {
            assert!(
                output.contains(&example.cmd),
                "Detail missing cmd: {}",
                example.cmd
            );
            assert!(
                output.contains(&example.description),
                "Detail missing description: {}",
                example.description
            );
        }
    }

    #[test]
    fn execute_examples_unknown_command_returns_error() {
        let global = GlobalOpts {
            port: None,
            host: "127.0.0.1".into(),
            ws_url: None,
            timeout: None,
            tab: None,
            auto_dismiss_dialogs: false,
            config: None,
            output: crate::cli::OutputFormat {
                json: false,
                pretty: false,
                plain: false,
            },
        };
        let args = ExamplesArgs {
            command: Some("nonexistent".into()),
        };
        let result = execute_examples(&global, &args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unknown command group"));
        assert!(err.message.contains("nonexistent"));
    }

    #[test]
    fn json_serialization_summary_has_expected_fields() {
        let groups = all_examples();
        let json = serde_json::to_string(&groups).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let arr = parsed.as_array().unwrap();
        assert!(!arr.is_empty());
        for entry in arr {
            assert!(entry.get("command").is_some(), "missing 'command' field");
            assert!(
                entry.get("description").is_some(),
                "missing 'description' field"
            );
            let examples = entry.get("examples").unwrap().as_array().unwrap();
            assert!(!examples.is_empty());
            for ex in examples {
                assert!(ex.get("cmd").is_some(), "missing 'cmd' field");
                assert!(
                    ex.get("description").is_some(),
                    "missing 'description' field"
                );
            }
        }
    }

    #[test]
    fn json_serialization_single_group_has_expected_fields() {
        let groups = all_examples();
        let navigate = groups.iter().find(|g| g.command == "navigate").unwrap();
        let json = serde_json::to_string(navigate).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.get("command").unwrap().as_str().unwrap(), "navigate");
        assert!(parsed.get("description").is_some());
        let examples = parsed.get("examples").unwrap().as_array().unwrap();
        assert!(examples.len() >= 3);
    }

    #[test]
    fn json_pretty_output_is_multiline() {
        let groups = all_examples();
        let json = serde_json::to_string_pretty(&groups).unwrap();
        assert!(json.lines().count() > 1, "pretty JSON should be multi-line");
        assert!(json.contains('\n'));
    }

    #[test]
    fn flags_field_omitted_when_none() {
        let entry = ExampleEntry {
            cmd: "chrome-cli test".into(),
            description: "A test".into(),
            flags: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            !json.contains("flags"),
            "flags field should be omitted when None"
        );
    }

    #[test]
    fn flags_field_present_when_some() {
        let entry = ExampleEntry {
            cmd: "chrome-cli test --flag".into(),
            description: "A test".into(),
            flags: Some(vec!["--flag".into()]),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(
            json.contains("flags"),
            "flags field should be present when Some"
        );
    }

    #[test]
    fn error_message_lists_all_available_groups() {
        let global = GlobalOpts {
            port: None,
            host: "127.0.0.1".into(),
            ws_url: None,
            timeout: None,
            tab: None,
            auto_dismiss_dialogs: false,
            config: None,
            output: crate::cli::OutputFormat {
                json: false,
                pretty: false,
                plain: false,
            },
        };
        let args = ExamplesArgs {
            command: Some("bogus".into()),
        };
        let err = execute_examples(&global, &args).unwrap_err();
        for group in all_examples() {
            assert!(
                err.message.contains(&group.command),
                "Error message should list '{}' as available",
                group.command
            );
        }
    }
}
