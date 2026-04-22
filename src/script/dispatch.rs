/// Command dispatch table for the script runner.
///
/// Routes `Step.cmd` argv slices to the appropriate command module by
/// re-parsing them through the full clap tree and calling a thin adapter.
///
/// Each adapter calls the underlying command module's logic and returns
/// `serde_json::Value` instead of printing to stdout.
use agentchrome::cdp::CdpClient;
use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use clap::Parser as _;

use crate::cli::{Cli, Command, GlobalOpts};
use crate::script::context::VarContext;

// =============================================================================
// Known subcommand set (for dry-run validation)
// =============================================================================

/// Subcommands that the script dispatcher supports.
pub const KNOWN_SUBCOMMANDS: &[&str] = &[
    "navigate", "page", "js", "form", "interact", "tabs", "console", "dialog", "dom", "network",
    "media", "emulate", "perf", "cookie",
];

/// Return true if `name` is a known dispatchable subcommand.
#[must_use]
pub fn is_known_subcommand(name: &str) -> bool {
    KNOWN_SUBCOMMANDS.contains(&name)
}

// =============================================================================
// Dispatch entry point
// =============================================================================

/// Invoke a command from an argv slice and return its JSON output.
///
/// `argv` should be the raw `cmd` array from a script step (e.g.
/// `["navigate", "https://example.com"]`). The dispatcher prepends
/// `"agentchrome"` to synthesize a full invocation and uses clap's
/// `try_parse_from` to route it.
///
/// The `client` and `session` are the *already-connected* CDP connection.
/// No new connection is established for each step.
///
/// # Errors
///
/// Returns `AppError` for unknown subcommands, clap parse failures, or
/// errors from the underlying command module.
pub async fn invoke(
    argv: &[String],
    _ctx: &VarContext,
    client: &CdpClient,
    session: &mut ManagedSession,
    global: &GlobalOpts,
) -> Result<serde_json::Value, AppError> {
    let subcommand = argv.first().map(String::as_str).ok_or_else(|| AppError {
        message: "script step 'cmd' array is empty".into(),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    if !is_known_subcommand(subcommand) {
        return Err(AppError {
            message: format!(
                "unknown subcommand in script: '{subcommand}'. \
                 Known commands: {}",
                KNOWN_SUBCOMMANDS.join(", ")
            ),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    // Build the full argv for clap: ["agentchrome", <subcommand>, <args...>]
    let mut full_argv: Vec<String> = Vec::with_capacity(argv.len() + 1);
    full_argv.push("agentchrome".to_string());
    full_argv.extend_from_slice(argv);

    // Parse through the full clap tree to get strongly-typed args
    let cli = Cli::try_parse_from(&full_argv).map_err(|e| AppError {
        message: format!("script step parse error for '{subcommand}': {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    // Dispatch to the right adapter
    dispatch_command(&cli.command, client, session, global).await
}

/// Dispatch to the appropriate command adapter.
async fn dispatch_command(
    command: &Command,
    client: &CdpClient,
    session: &mut ManagedSession,
    global: &GlobalOpts,
) -> Result<serde_json::Value, AppError> {
    match command {
        Command::Navigate(args) => crate::navigate::run_from_session(session, global, args).await,
        Command::Page(args) => crate::page::run_from_session(session, global, args).await,
        Command::Js(args) => crate::js::run_from_session(session, global, args).await,
        Command::Form(args) => crate::form::run_from_session(session, global, args).await,
        Command::Interact(args) => crate::interact::run_from_session(session, global, args).await,
        Command::Tabs(args) => crate::tabs::run_from_session(client, session, global, args).await,
        Command::Console(args) => crate::console::run_from_session(session, global, args).await,
        Command::Dialog(args) => crate::dialog::run_from_session(session, global, args).await,
        Command::Dom(args) => crate::dom::run_from_session(session, global, args).await,
        Command::Network(args) => crate::network::run_from_session(session, global, args).await,
        Command::Media(args) => crate::media::run_from_session(session, global, args).await,
        Command::Emulate(args) => crate::emulate::run_from_session(session, global, args).await,
        Command::Perf(args) => crate::perf::run_from_session(session, global, args).await,
        Command::Cookie(args) => crate::cookie::run_from_session(session, global, args).await,
        _ => Err(AppError {
            message: "this command is not supported inside scripts".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        }),
    }
}

// =============================================================================
// Unit tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_subcommand_navigate() {
        assert!(is_known_subcommand("navigate"));
    }

    #[test]
    fn unknown_subcommand_rejected() {
        assert!(!is_known_subcommand("connect"));
        assert!(!is_known_subcommand("script"));
        assert!(!is_known_subcommand("nonexistent"));
    }
}
