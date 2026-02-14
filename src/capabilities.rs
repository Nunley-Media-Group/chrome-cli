use clap::CommandFactory;
use serde::Serialize;

use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{CapabilitiesArgs, Cli, GlobalOpts};

// =============================================================================
// Output types — the manifest schema
// =============================================================================

#[derive(Serialize)]
pub struct CapabilitiesManifest {
    name: String,
    version: String,
    commands: Vec<CommandDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    global_flags: Option<Vec<FlagDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_codes: Option<Vec<ExitCodeDescriptor>>,
}

#[derive(Serialize)]
pub struct CommandDescriptor {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subcommands: Option<Vec<SubcommandDescriptor>>,
}

#[derive(Serialize)]
pub struct SubcommandDescriptor {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<ArgDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<FlagDescriptor>>,
}

#[derive(Serialize)]
pub struct ArgDescriptor {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    required: bool,
    description: String,
}

#[derive(Serialize)]
pub struct FlagDescriptor {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<String>>,
    description: String,
}

#[derive(Serialize)]
pub struct ExitCodeDescriptor {
    code: u8,
    name: String,
    description: String,
}

// =============================================================================
// Clap tree walking — the core introspection logic
// =============================================================================

/// Build the full capabilities manifest from the clap command tree.
pub fn build_manifest(cmd: &clap::Command, compact: bool) -> CapabilitiesManifest {
    let commands: Vec<CommandDescriptor> = cmd
        .get_subcommands()
        .filter(|s| !s.is_hide_set())
        .map(|s| visit_command(s, compact))
        .collect();

    CapabilitiesManifest {
        name: cmd.get_name().to_string(),
        version: cmd.get_version().unwrap_or("unknown").to_string(),
        commands,
        global_flags: if compact {
            None
        } else {
            Some(global_flags(cmd))
        },
        exit_codes: if compact { None } else { Some(exit_codes()) },
    }
}

/// Visit a top-level command (e.g., `navigate`, `tabs`, `connect`).
fn visit_command(cmd: &clap::Command, compact: bool) -> CommandDescriptor {
    let description = cmd
        .get_about()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();

    if compact {
        return CommandDescriptor {
            name: cmd.get_name().to_string(),
            description,
            subcommands: None,
        };
    }

    let subs: Vec<&clap::Command> = cmd.get_subcommands().filter(|s| !s.is_hide_set()).collect();

    let parent_name = cmd.get_name();

    let mut subcommands = Vec::new();

    // If the command has positional args at its own level (hybrid like navigate),
    // create an implicit subcommand for the direct usage.
    let has_positional = cmd
        .get_arguments()
        .any(|a| a.is_positional() && !is_internal_arg(a));
    if has_positional {
        subcommands.push(visit_subcommand(parent_name, cmd));
    }

    // Add explicit subcommands
    for sub in &subs {
        subcommands.push(visit_subcommand(parent_name, sub));
    }

    // If no subcommands and no positional args, treat the command's flags as a
    // single implicit subcommand (flat commands like `connect`).
    if subcommands.is_empty() {
        let flags = extract_flags(cmd);
        if !flags.is_empty() {
            subcommands.push(SubcommandDescriptor {
                name: cmd.get_name().to_string(),
                description: description.clone(),
                args: Some(Vec::new()),
                flags: Some(flags),
            });
        }
    }

    CommandDescriptor {
        name: cmd.get_name().to_string(),
        description,
        subcommands: if subcommands.is_empty() {
            None
        } else {
            Some(subcommands)
        },
    }
}

/// Visit a subcommand and extract its args and flags.
fn visit_subcommand(parent_name: &str, cmd: &clap::Command) -> SubcommandDescriptor {
    let name = if cmd.get_name() == parent_name {
        // Implicit subcommand — show positional args in the name
        let positionals: Vec<String> = cmd
            .get_arguments()
            .filter(|a| a.is_positional() && !is_internal_arg(a))
            .map(|a| format!("<{}>", a.get_id().as_str().to_uppercase()))
            .collect();
        if positionals.is_empty() {
            parent_name.to_string()
        } else {
            format!("{parent_name} {}", positionals.join(" "))
        }
    } else {
        format!("{parent_name} {}", cmd.get_name())
    };

    let description = cmd
        .get_about()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();

    SubcommandDescriptor {
        name,
        description,
        args: Some(extract_args(cmd)),
        flags: Some(extract_flags(cmd)),
    }
}

/// Extract positional arguments from a command.
fn extract_args(cmd: &clap::Command) -> Vec<ArgDescriptor> {
    cmd.get_arguments()
        .filter(|a| a.is_positional() && !is_internal_arg(a))
        .map(|a| {
            let type_name = infer_type_with_possible_values(a);
            ArgDescriptor {
                name: a.get_id().as_str().to_string(),
                type_name,
                required: a.is_required_set(),
                description: a
                    .get_help()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default(),
            }
        })
        .collect()
}

/// Extract flags (long options) from a command, excluding global flags.
fn extract_flags(cmd: &clap::Command) -> Vec<FlagDescriptor> {
    cmd.get_arguments()
        .filter(|a| !a.is_positional() && !is_internal_arg(a) && !a.is_global_set())
        .filter_map(|a| {
            let long = a.get_long()?;
            let name = format!("--{long}");
            let type_name = infer_type_with_possible_values(a);
            let values = if type_name == "enum" {
                Some(
                    a.get_possible_values()
                        .iter()
                        .filter(|v| !v.is_hide_set())
                        .map(|v| v.get_name().to_string())
                        .collect(),
                )
            } else {
                None
            };
            Some(FlagDescriptor {
                name,
                type_name,
                required: Some(a.is_required_set()),
                default: extract_default(a),
                values,
                description: a
                    .get_help()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Infer a display type, checking possible values for enums (but bool actions take priority).
fn infer_type_with_possible_values(arg: &clap::Arg) -> String {
    // Bool actions take priority — clap reports possible values ["true","false"] for SetTrue
    match arg.get_action() {
        clap::ArgAction::SetTrue | clap::ArgAction::SetFalse => return "bool".to_string(),
        _ => {}
    }

    let possible = arg.get_possible_values();
    if !possible.is_empty() {
        return "enum".to_string();
    }

    infer_type(arg)
}

/// Infer a display type from a clap `Arg` using heuristics.
fn infer_type(arg: &clap::Arg) -> String {
    // Check for bool (flag-style args with SetTrue/SetFalse)
    match arg.get_action() {
        clap::ArgAction::SetTrue | clap::ArgAction::SetFalse => return "bool".to_string(),
        clap::ArgAction::Count => return "integer".to_string(),
        _ => {}
    }

    // Check for multiple values
    if let Some(num_args) = arg.get_num_args() {
        if num_args.max_values() > 1 {
            return "array".to_string();
        }
    }

    // Infer from value name
    let value_names: Vec<&str> = arg
        .get_value_names()
        .map(|names| names.iter().map(clap::builder::Str::as_str).collect())
        .unwrap_or_default();

    let id = arg.get_id().as_str().to_uppercase();

    let all_names: Vec<&str> = value_names
        .iter()
        .copied()
        .chain(std::iter::once(id.as_str()))
        .collect();

    for name in &all_names {
        let upper = name.to_uppercase();
        if matches!(
            upper.as_str(),
            "PORT"
                | "TIMEOUT"
                | "LIMIT"
                | "PAGE"
                | "QUALITY"
                | "REPEAT"
                | "AMOUNT"
                | "DELAY"
                | "CPU"
                | "X"
                | "Y"
                | "REQ_ID"
                | "MSG_ID"
                | "MAX_SIZE"
        ) {
            return "integer".to_string();
        }
        if matches!(upper.as_str(), "PATH" | "FILE" | "DIR") {
            return "path".to_string();
        }
    }

    "string".to_string()
}

/// Extract default values from a clap `Arg`.
fn extract_default(arg: &clap::Arg) -> Option<serde_json::Value> {
    let defaults = arg.get_default_values();
    if defaults.is_empty() {
        return None;
    }

    let val = defaults[0].to_str().unwrap_or("");

    // Try integer
    if let Ok(n) = val.parse::<i64>() {
        return Some(serde_json::Value::Number(n.into()));
    }

    // Try float
    if let Ok(n) = val.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(n) {
            return Some(serde_json::Value::Number(num));
        }
    }

    // Try bool
    if val == "true" {
        return Some(serde_json::Value::Bool(true));
    }
    if val == "false" {
        return Some(serde_json::Value::Bool(false));
    }

    Some(serde_json::Value::String(val.to_string()))
}

/// Extract global flags from the root command.
fn global_flags(cmd: &clap::Command) -> Vec<FlagDescriptor> {
    cmd.get_arguments()
        .filter(|a| a.is_global_set() && !is_internal_arg(a))
        .filter_map(|a| {
            let long = a.get_long()?;
            let name = format!("--{long}");
            let type_name = infer_type_with_possible_values(a);
            let values = if type_name == "enum" {
                Some(
                    a.get_possible_values()
                        .iter()
                        .filter(|v| !v.is_hide_set())
                        .map(|v| v.get_name().to_string())
                        .collect(),
                )
            } else {
                None
            };
            Some(FlagDescriptor {
                name,
                type_name,
                required: None,
                default: extract_default(a),
                values,
                description: a
                    .get_help()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Return static exit code documentation from the `ExitCode` enum.
fn exit_codes() -> Vec<ExitCodeDescriptor> {
    vec![
        ExitCodeDescriptor {
            code: 0,
            name: "Success".into(),
            description: "Command completed successfully".into(),
        },
        ExitCodeDescriptor {
            code: 1,
            name: "GeneralError".into(),
            description: "Invalid arguments or internal failure".into(),
        },
        ExitCodeDescriptor {
            code: 2,
            name: "ConnectionError".into(),
            description: "Chrome not running or session expired".into(),
        },
        ExitCodeDescriptor {
            code: 3,
            name: "TargetError".into(),
            description: "Tab not found or no page targets".into(),
        },
        ExitCodeDescriptor {
            code: 4,
            name: "TimeoutError".into(),
            description: "Navigation or trace timeout".into(),
        },
        ExitCodeDescriptor {
            code: 5,
            name: "ProtocolError".into(),
            description: "CDP protocol failure".into(),
        },
    ]
}

/// Check if an argument is an internal clap argument (like `help` or `version`).
fn is_internal_arg(arg: &clap::Arg) -> bool {
    let id = arg.get_id().as_str();
    matches!(id, "help" | "version")
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
    })?;
    println!("{json}");
    Ok(())
}

// =============================================================================
// Dispatcher
// =============================================================================

pub fn execute_capabilities(global: &GlobalOpts, args: &CapabilitiesArgs) -> Result<(), AppError> {
    let cmd = Cli::command();
    let mut manifest = build_manifest(&cmd, args.compact);

    // Filter to a specific command if requested
    if let Some(ref name) = args.command {
        let available: Vec<String> = manifest.commands.iter().map(|c| c.name.clone()).collect();

        let matching: Vec<CommandDescriptor> = manifest
            .commands
            .into_iter()
            .filter(|c| c.name == *name)
            .collect();

        if matching.is_empty() {
            return Err(AppError {
                message: format!(
                    "Unknown command: '{name}'. Available: {}",
                    available.join(", ")
                ),
                code: ExitCode::GeneralError,
            });
        }

        manifest.commands = matching;
    }

    print_output(&manifest, &global.output)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn root_cmd() -> clap::Command {
        Cli::command()
    }

    #[test]
    fn manifest_has_correct_name_and_version() {
        let manifest = build_manifest(&root_cmd(), false);
        assert_eq!(manifest.name, "chrome-cli");
        assert!(!manifest.version.is_empty());
    }

    #[test]
    fn manifest_covers_all_commands() {
        let cmd = root_cmd();
        let manifest = build_manifest(&cmd, false);
        let expected_names: HashSet<String> = cmd
            .get_subcommands()
            .filter(|s| !s.is_hide_set())
            .map(|s| s.get_name().to_string())
            .collect();
        let manifest_names: HashSet<String> =
            manifest.commands.iter().map(|c| c.name.clone()).collect();
        assert_eq!(expected_names, manifest_names);
    }

    #[test]
    fn each_command_has_description() {
        let manifest = build_manifest(&root_cmd(), false);
        for cmd in &manifest.commands {
            assert!(
                !cmd.description.is_empty(),
                "Command '{}' has empty description",
                cmd.name
            );
        }
    }

    #[test]
    fn global_flags_include_known_flags() {
        let flags = global_flags(&root_cmd());
        let names: Vec<&str> = flags.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"--port"), "Missing --port");
        assert!(names.contains(&"--host"), "Missing --host");
        assert!(names.contains(&"--ws-url"), "Missing --ws-url");
        assert!(names.contains(&"--timeout"), "Missing --timeout");
        assert!(names.contains(&"--tab"), "Missing --tab");
        assert!(
            names.contains(&"--auto-dismiss-dialogs"),
            "Missing --auto-dismiss-dialogs"
        );
        assert!(names.contains(&"--config"), "Missing --config");
        assert!(names.contains(&"--json"), "Missing --json");
        assert!(names.contains(&"--pretty"), "Missing --pretty");
        assert!(names.contains(&"--plain"), "Missing --plain");
    }

    #[test]
    fn exit_codes_returns_all_six() {
        let codes = exit_codes();
        assert_eq!(codes.len(), 6);
        assert_eq!(codes[0].code, 0);
        assert_eq!(codes[0].name, "Success");
        assert_eq!(codes[5].code, 5);
        assert_eq!(codes[5].name, "ProtocolError");
    }

    #[test]
    fn infer_type_returns_bool_for_set_true() {
        let arg = clap::Arg::new("test")
            .long("test")
            .action(clap::ArgAction::SetTrue);
        assert_eq!(infer_type(&arg), "bool");
    }

    #[test]
    fn infer_type_returns_enum_for_possible_values() {
        // We test via extract_args/extract_flags that enum detection works by
        // checking the navigate command's --wait-until flag.
        let cmd = root_cmd();
        let nav = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "navigate")
            .expect("navigate command not found");

        let flags = extract_flags(nav);
        let wait_until = flags.iter().find(|f| f.name == "--wait-until");
        assert!(wait_until.is_some(), "Missing --wait-until flag");
        let wf = wait_until.unwrap();
        assert_eq!(wf.type_name, "enum");
        assert!(wf.values.is_some());
        let values = wf.values.as_ref().unwrap();
        assert!(values.contains(&"load".to_string()));
        assert!(values.contains(&"domcontentloaded".to_string()));
        assert!(values.contains(&"networkidle".to_string()));
        assert!(values.contains(&"none".to_string()));
    }

    #[test]
    fn compact_mode_omits_details() {
        let manifest = build_manifest(&root_cmd(), true);
        for cmd in &manifest.commands {
            assert!(
                cmd.subcommands.is_none(),
                "Compact mode should not include subcommands for '{}'",
                cmd.name
            );
        }
        assert!(manifest.global_flags.is_none());
        assert!(manifest.exit_codes.is_none());
    }

    #[test]
    fn command_filter_returns_single_command() {
        let cmd = root_cmd();
        let manifest = build_manifest(&cmd, false);
        let filtered: Vec<&CommandDescriptor> = manifest
            .commands
            .iter()
            .filter(|c| c.name == "navigate")
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "navigate");
    }

    #[test]
    fn commands_with_subcommands_have_populated_list() {
        let manifest = build_manifest(&root_cmd(), false);
        let tabs = manifest
            .commands
            .iter()
            .find(|c| c.name == "tabs")
            .expect("tabs command not found");
        assert!(tabs.subcommands.is_some());
        assert!(
            !tabs.subcommands.as_ref().unwrap().is_empty(),
            "tabs should have subcommands"
        );
    }

    #[test]
    fn execute_capabilities_unknown_command_returns_error() {
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
        let args = CapabilitiesArgs {
            command: Some("nonexistent".into()),
            compact: false,
        };
        let result = execute_capabilities(&global, &args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Unknown command"));
        assert!(err.message.contains("nonexistent"));
    }

    #[test]
    fn json_serialization_roundtrips() {
        let manifest = build_manifest(&root_cmd(), false);
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["name"], "chrome-cli");
        assert!(parsed["commands"].is_array());
        assert!(parsed["global_flags"].is_array());
        assert!(parsed["exit_codes"].is_array());
    }

    #[test]
    fn compact_json_has_no_global_flags_or_exit_codes() {
        let manifest = build_manifest(&root_cmd(), true);
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("global_flags").is_none());
        assert!(parsed.get("exit_codes").is_none());
    }
}
