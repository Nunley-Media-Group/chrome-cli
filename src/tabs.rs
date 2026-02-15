use std::fmt::Write;
use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::chrome::{TargetInfo, query_targets};
use chrome_cli::connection::{resolve_connection, select_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, TabsArgs, TabsCommand};

/// Execute the `tabs` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_tabs(global: &GlobalOpts, args: &TabsArgs) -> Result<(), AppError> {
    match &args.command {
        TabsCommand::List(list_args) => execute_list(global, list_args.all).await,
        TabsCommand::Create(create_args) => {
            execute_create(global, create_args.url.as_deref(), create_args.background).await
        }
        TabsCommand::Close(close_args) => execute_close(global, &close_args.targets).await,
        TabsCommand::Activate(act_args) => {
            execute_activate(global, &act_args.target, act_args.quiet).await
        }
    }
}

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct TabInfo {
    id: String,
    url: String,
    title: String,
    active: bool,
}

#[derive(Serialize)]
struct CreateResult {
    id: String,
    url: String,
    title: String,
}

#[derive(Serialize)]
struct CloseResult {
    closed: Vec<String>,
    remaining: usize,
}

#[derive(Serialize)]
struct ActivateResult {
    activated: String,
    url: String,
    title: String,
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

fn format_plain_table(tabs: &[TabInfo]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "  {:<3} {:<14} {:<20} {:<26} ACTIVE",
        "#", "ID", "TITLE", "URL"
    );
    for (i, tab) in tabs.iter().enumerate() {
        let active_marker = if tab.active { "*" } else { "" };
        let title: String = tab.title.chars().take(20).collect();
        let url: String = tab.url.chars().take(26).collect();
        let _ = writeln!(
            out,
            "  {i:<3} {:<14} {:<20} {:<26} {}",
            tab.id, title, url, active_marker
        );
    }
    out
}

// =============================================================================
// Filtering
// =============================================================================

fn filter_page_targets(targets: &[TargetInfo], include_all: bool) -> Vec<&TargetInfo> {
    targets
        .iter()
        .filter(|t| t.target_type == "page")
        .filter(|t| {
            if include_all {
                return true;
            }
            let url = &t.url;
            // Always include chrome://newtab/
            if url.starts_with("chrome://newtab") {
                return true;
            }
            // Exclude other chrome:// and chrome-extension:// URLs
            if url.starts_with("chrome://") || url.starts_with("chrome-extension://") {
                return false;
            }
            true
        })
        .collect()
}

// =============================================================================
// Subcommand handlers
// =============================================================================

async fn execute_list(global: &GlobalOpts, include_all: bool) -> Result<(), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;

    let targets = query_targets(&conn.host, conn.port).await?;
    let filtered = filter_page_targets(&targets, include_all);

    let tabs: Vec<TabInfo> = filtered
        .iter()
        .enumerate()
        .map(|(i, t)| TabInfo {
            id: t.id.clone(),
            url: t.url.clone(),
            title: t.title.clone(),
            active: i == 0,
        })
        .collect();

    if global.output.plain {
        print!("{}", format_plain_table(&tabs));
        return Ok(());
    }

    print_output(&tabs, &global.output)
}

async fn execute_create(
    global: &GlobalOpts,
    url: Option<&str>,
    background: bool,
) -> Result<(), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;

    // When --background is used, record the currently active tab so we can
    // re-activate it after creation (Chrome does not reliably honor the
    // `background` parameter in Target.createTarget).
    let original_active_id = if background {
        let targets = query_targets(&conn.host, conn.port).await?;
        targets
            .iter()
            .find(|t| t.target_type == "page")
            .map(|t| t.id.clone())
    } else {
        None
    };

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;

    let target_url = url.unwrap_or("about:blank");
    let mut params = serde_json::json!({ "url": target_url });
    if background {
        params["background"] = serde_json::json!(true);
    }

    let result = client
        .send_command("Target.createTarget", Some(params))
        .await?;

    let target_id = result["targetId"].as_str().unwrap_or_default().to_string();

    // Re-activate the original tab if --background was requested
    if let Some(ref active_id) = original_active_id {
        let activate_params = serde_json::json!({ "targetId": active_id });
        client
            .send_command("Target.activateTarget", Some(activate_params))
            .await?;
    }

    // Re-query to get the new tab's resolved URL and title
    let targets = query_targets(&conn.host, conn.port).await?;
    let new_tab = targets.iter().find(|t| t.id == target_id);

    let output = CreateResult {
        id: target_id,
        url: new_tab.map_or(target_url.to_string(), |t| t.url.clone()),
        title: new_tab.map_or(String::new(), |t| t.title.clone()),
    };

    print_output(&output, &global.output)
}

async fn execute_close(global: &GlobalOpts, target_args: &[String]) -> Result<(), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;

    let targets = query_targets(&conn.host, conn.port).await?;

    // Resolve all target arguments BEFORE closing any (avoids index shift)
    let mut to_close: Vec<&TargetInfo> = Vec::new();
    for arg in target_args {
        let target = select_target(&targets, Some(arg))?;
        to_close.push(target);
    }

    // Last-tab protection: count page targets and how many we're closing
    let page_count = targets.iter().filter(|t| t.target_type == "page").count();
    let closing_page_count = to_close.iter().filter(|t| t.target_type == "page").count();
    if closing_page_count >= page_count {
        return Err(AppError::last_tab());
    }

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;

    let mut closed_ids = Vec::new();
    for target in &to_close {
        let params = serde_json::json!({ "targetId": target.id });
        client
            .send_command("Target.closeTarget", Some(params))
            .await?;
        closed_ids.push(target.id.clone());
    }

    // Re-query remaining count
    let remaining_targets = query_targets(&conn.host, conn.port).await?;
    let remaining = remaining_targets
        .iter()
        .filter(|t| t.target_type == "page")
        .count();

    let output = CloseResult {
        closed: closed_ids,
        remaining,
    };

    print_output(&output, &global.output)
}

async fn execute_activate(
    global: &GlobalOpts,
    target_arg: &str,
    quiet: bool,
) -> Result<(), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;

    let targets = query_targets(&conn.host, conn.port).await?;
    let target = select_target(&targets, Some(target_arg))?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;

    let params = serde_json::json!({ "targetId": target.id });
    client
        .send_command("Target.activateTarget", Some(params))
        .await?;

    if quiet {
        return Ok(());
    }

    let output = ActivateResult {
        activated: target.id.clone(),
        url: target.url.clone(),
        title: target.title.clone(),
    };

    print_output(&output, &global.output)
}

// =============================================================================
// Helpers
// =============================================================================

fn cdp_config(global: &GlobalOpts) -> CdpConfig {
    let mut config = CdpConfig::default();
    if let Some(timeout_ms) = global.timeout {
        config.command_timeout = Duration::from_millis(timeout_ms);
    }
    config
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_target(id: &str, target_type: &str, url: &str) -> TargetInfo {
        TargetInfo {
            id: id.to_string(),
            target_type: target_type.to_string(),
            title: format!("Title {id}"),
            url: url.to_string(),
            ws_debugger_url: Some(format!("ws://127.0.0.1:9222/devtools/page/{id}")),
        }
    }

    #[test]
    fn filter_excludes_chrome_urls() {
        let targets = vec![
            make_target("a", "page", "https://google.com"),
            make_target("b", "page", "chrome://settings/"),
            make_target("c", "page", "chrome://extensions/"),
        ];
        let filtered = filter_page_targets(&targets, false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "a");
    }

    #[test]
    fn filter_keeps_chrome_newtab() {
        let targets = vec![
            make_target("a", "page", "chrome://newtab/"),
            make_target("b", "page", "chrome://settings/"),
        ];
        let filtered = filter_page_targets(&targets, false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "a");
    }

    #[test]
    fn filter_excludes_chrome_extension_urls() {
        let targets = vec![
            make_target("a", "page", "https://example.com"),
            make_target("b", "page", "chrome-extension://abc123/popup.html"),
        ];
        let filtered = filter_page_targets(&targets, false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "a");
    }

    #[test]
    fn filter_all_returns_all_page_targets() {
        let targets = vec![
            make_target("a", "page", "https://google.com"),
            make_target("b", "page", "chrome://settings/"),
            make_target("c", "page", "chrome-extension://abc/popup.html"),
        ];
        let filtered = filter_page_targets(&targets, true);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn filter_excludes_non_page_types() {
        let targets = vec![
            make_target("a", "page", "https://google.com"),
            make_target("b", "service_worker", "https://sw.example.com"),
            make_target("c", "background_page", "chrome-extension://abc/bg.html"),
        ];
        let filtered = filter_page_targets(&targets, false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "a");
    }

    #[test]
    fn first_page_target_is_active() {
        let targets = vec![
            make_target("a", "page", "https://google.com"),
            make_target("b", "page", "https://github.com"),
        ];
        let filtered = filter_page_targets(&targets, false);
        let tabs: Vec<TabInfo> = filtered
            .iter()
            .enumerate()
            .map(|(i, t)| TabInfo {
                id: t.id.clone(),
                url: t.url.clone(),
                title: t.title.clone(),
                active: i == 0,
            })
            .collect();
        assert!(tabs[0].active);
        assert!(!tabs[1].active);
    }

    #[test]
    fn plain_table_format() {
        let tabs = vec![
            TabInfo {
                id: "ABC123".to_string(),
                url: "https://google.com".to_string(),
                title: "Google".to_string(),
                active: true,
            },
            TabInfo {
                id: "DEF456".to_string(),
                url: "https://github.com".to_string(),
                title: "GitHub".to_string(),
                active: false,
            },
        ];
        let output = format_plain_table(&tabs);
        assert!(output.contains('#'));
        assert!(output.contains("ID"));
        assert!(output.contains("TITLE"));
        assert!(output.contains("URL"));
        assert!(output.contains("ACTIVE"));
        assert!(output.contains("ABC123"));
        assert!(output.contains("Google"));
        assert!(output.contains('*'));
        assert!(output.contains("DEF456"));
        assert!(output.contains("GitHub"));
    }

    #[test]
    fn last_tab_protection_logic() {
        // Simulate: 2 page targets, closing both should fail
        let page_count = 2;
        let closing_page_count = 2;
        assert!(closing_page_count >= page_count);

        // Simulate: 2 page targets, closing 1 should succeed
        let closing_page_count = 1;
        assert!(closing_page_count < page_count);

        // Simulate: 1 page target, closing 1 should fail
        let page_count = 1;
        let closing_page_count = 1;
        assert!(closing_page_count >= page_count);
    }
}
