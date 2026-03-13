use std::time::Duration;

use serde::Serialize;

use agentchrome::cdp::{CdpClient, CdpConfig};
use agentchrome::connection::{ManagedSession, resolve_connection, resolve_target};
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{
    CookieArgs, CookieCommand, CookieDeleteArgs, CookieListArgs, CookieSetArgs, GlobalOpts,
};
use crate::emulate::apply_emulate_state;

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct CookieInfo {
    name: String,
    value: String,
    domain: String,
    path: String,
    expires: f64,
    #[serde(rename = "httpOnly")]
    http_only: bool,
    secure: bool,
    #[serde(rename = "sameSite")]
    same_site: String,
    size: u64,
}

#[derive(Serialize)]
struct SetResult {
    success: bool,
    name: String,
    domain: String,
}

#[derive(Serialize)]
struct DeleteResult {
    deleted: u64,
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

fn print_list_plain(cookies: &[CookieInfo]) {
    if cookies.is_empty() {
        println!("No cookies");
        return;
    }
    for c in cookies {
        println!("{}: {}", c.name, c.value);
    }
}

fn print_set_plain(result: &SetResult) {
    println!("Set cookie: {} (domain: {})", result.name, result.domain);
}

fn print_delete_plain(result: &DeleteResult) {
    println!("Deleted {} cookie(s)", result.deleted);
}

fn print_clear_plain(result: &DeleteResult) {
    println!("Cleared {} cookie(s)", result.deleted);
}

// =============================================================================
// Config helper
// =============================================================================

fn cdp_config(global: &GlobalOpts) -> CdpConfig {
    let mut config = CdpConfig::default();
    if let Some(timeout_ms) = global.timeout {
        config.command_timeout = Duration::from_millis(timeout_ms);
    }
    config
}

// =============================================================================
// Session setup
// =============================================================================

async fn setup_session(global: &GlobalOpts) -> Result<(CdpClient, ManagedSession), AppError> {
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;
    let target = resolve_target(
        &conn.host,
        conn.port,
        global.tab.as_deref(),
        global.page_id.as_deref(),
    )
    .await?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let mut managed = ManagedSession::new(session);
    apply_emulate_state(&mut managed).await?;
    managed.install_dialog_interceptors().await;

    Ok((client, managed))
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `cookie` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_cookie(global: &GlobalOpts, args: &CookieArgs) -> Result<(), AppError> {
    match &args.command {
        CookieCommand::List(list_args) => execute_list(global, list_args).await,
        CookieCommand::Set(set_args) => execute_set(global, set_args).await,
        CookieCommand::Delete(delete_args) => execute_delete(global, delete_args).await,
        CookieCommand::Clear => execute_clear(global).await,
    }
}

// =============================================================================
// List: get cookies
// =============================================================================

async fn execute_list(global: &GlobalOpts, args: &CookieListArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    managed.ensure_domain("Network").await?;

    let method = if args.all {
        "Network.getAllCookies"
    } else {
        "Network.getCookies"
    };

    let response = managed
        .send_command(method, None)
        .await
        .map_err(|e| AppError {
            message: format!("{method} failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let cookies = parse_cookies(&response);

    // Client-side domain filter
    let cookies: Vec<CookieInfo> = if let Some(ref domain) = args.domain {
        cookies
            .into_iter()
            .filter(|c| c.domain.contains(domain.as_str()))
            .collect()
    } else {
        cookies
    };

    if global.output.plain {
        print_list_plain(&cookies);
        Ok(())
    } else {
        print_output(&cookies, &global.output)
    }
}

// =============================================================================
// Set: create/update a cookie
// =============================================================================

async fn execute_set(global: &GlobalOpts, args: &CookieSetArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    managed.ensure_domain("Network").await?;

    let mut params = serde_json::json!({
        "name": args.name,
        "value": args.value,
        "path": args.path,
    });

    if let Some(ref domain) = args.domain {
        params["domain"] = serde_json::Value::String(domain.clone());
    }
    if args.secure {
        params["secure"] = serde_json::Value::Bool(true);
    }
    if args.http_only {
        params["httpOnly"] = serde_json::Value::Bool(true);
    }
    if let Some(ref same_site) = args.same_site {
        params["sameSite"] = serde_json::Value::String(same_site.clone());
    }
    if let Some(expires) = args.expires {
        params["expires"] = serde_json::json!(expires);
    }

    let response = managed
        .send_command("Network.setCookie", Some(params))
        .await
        .map_err(|e| AppError {
            message: format!("Network.setCookie failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let success = response["success"].as_bool().unwrap_or(false);
    if !success {
        return Err(AppError {
            message: "Network.setCookie returned success: false — cookie was not set".to_string(),
            code: ExitCode::ProtocolError,
            custom_json: None,
        });
    }

    let result = SetResult {
        success: true,
        name: args.name.clone(),
        domain: args.domain.clone().unwrap_or_default(),
    };

    if global.output.plain {
        print_set_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// Delete: remove a specific cookie
// =============================================================================

async fn execute_delete(global: &GlobalOpts, args: &CookieDeleteArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    managed.ensure_domain("Network").await?;

    let mut params = serde_json::json!({ "name": args.name });
    if let Some(ref domain) = args.domain {
        params["domain"] = serde_json::Value::String(domain.clone());
    }

    managed
        .send_command("Network.deleteCookies", Some(params))
        .await
        .map_err(|e| AppError {
            message: format!("Network.deleteCookies failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let result = DeleteResult { deleted: 1 };

    if global.output.plain {
        print_delete_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// Clear: remove all cookies
// =============================================================================

async fn execute_clear(global: &GlobalOpts) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    managed.ensure_domain("Network").await?;

    // Count existing cookies before clearing
    let response = managed
        .send_command("Network.getAllCookies", None)
        .await
        .map_err(|e| AppError {
            message: format!("Network.getAllCookies failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let count = response["cookies"]
        .as_array()
        .map_or(0, |arr| arr.len() as u64);

    // Clear all cookies
    managed
        .send_command("Network.clearBrowserCookies", None)
        .await
        .map_err(|e| AppError {
            message: format!("Network.clearBrowserCookies failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let result = DeleteResult { deleted: count };

    if global.output.plain {
        print_clear_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn parse_cookies(response: &serde_json::Value) -> Vec<CookieInfo> {
    let Some(cookies) = response["cookies"].as_array() else {
        return Vec::new();
    };

    cookies
        .iter()
        .map(|c| CookieInfo {
            name: c["name"].as_str().unwrap_or("").to_string(),
            value: c["value"].as_str().unwrap_or("").to_string(),
            domain: c["domain"].as_str().unwrap_or("").to_string(),
            path: c["path"].as_str().unwrap_or("/").to_string(),
            expires: c["expires"].as_f64().unwrap_or(0.0),
            http_only: c["httpOnly"].as_bool().unwrap_or(false),
            secure: c["secure"].as_bool().unwrap_or(false),
            same_site: c["sameSite"].as_str().unwrap_or("").to_string(),
            size: c["size"].as_u64().unwrap_or(0),
        })
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_info_serialization() {
        let cookie = CookieInfo {
            name: "session_id".into(),
            value: "abc123".into(),
            domain: ".example.com".into(),
            path: "/".into(),
            expires: 1_735_689_600.0,
            http_only: true,
            secure: true,
            same_site: "Lax".into(),
            size: 22,
        };
        let json: serde_json::Value = serde_json::to_value(&cookie).unwrap();
        assert_eq!(json["name"], "session_id");
        assert_eq!(json["value"], "abc123");
        assert_eq!(json["domain"], ".example.com");
        assert_eq!(json["path"], "/");
        assert_eq!(json["httpOnly"], true);
        assert_eq!(json["secure"], true);
        assert_eq!(json["sameSite"], "Lax");
        assert_eq!(json["size"], 22);
    }

    #[test]
    fn cookie_info_serialization_minimal() {
        let cookie = CookieInfo {
            name: "test".into(),
            value: "val".into(),
            domain: "".into(),
            path: "/".into(),
            expires: 0.0,
            http_only: false,
            secure: false,
            same_site: "".into(),
            size: 0,
        };
        let json: serde_json::Value = serde_json::to_value(&cookie).unwrap();
        assert_eq!(json["name"], "test");
        assert_eq!(json["httpOnly"], false);
        assert_eq!(json["secure"], false);
    }

    #[test]
    fn set_result_serialization() {
        let result = SetResult {
            success: true,
            name: "session_id".into(),
            domain: "example.com".into(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["name"], "session_id");
        assert_eq!(json["domain"], "example.com");
    }

    #[test]
    fn delete_result_serialization() {
        let result = DeleteResult { deleted: 1 };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["deleted"], 1);
    }

    #[test]
    fn delete_result_clear_count() {
        let result = DeleteResult { deleted: 5 };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["deleted"], 5);
    }

    #[test]
    fn parse_cookies_from_cdp_response() {
        let response = serde_json::json!({
            "cookies": [
                {
                    "name": "session_id",
                    "value": "abc123",
                    "domain": ".example.com",
                    "path": "/",
                    "expires": 1735689600.0,
                    "size": 22,
                    "httpOnly": true,
                    "secure": true,
                    "session": false,
                    "sameSite": "Lax"
                },
                {
                    "name": "prefs",
                    "value": "dark",
                    "domain": "example.com",
                    "path": "/settings",
                    "expires": 0.0,
                    "size": 9,
                    "httpOnly": false,
                    "secure": false,
                    "session": true,
                    "sameSite": ""
                }
            ]
        });

        let cookies = parse_cookies(&response);
        assert_eq!(cookies.len(), 2);
        assert_eq!(cookies[0].name, "session_id");
        assert!(cookies[0].http_only);
        assert!(cookies[0].secure);
        assert_eq!(cookies[0].same_site, "Lax");
        assert_eq!(cookies[1].name, "prefs");
        assert!(!cookies[1].http_only);
    }

    #[test]
    fn parse_cookies_empty_response() {
        let response = serde_json::json!({ "cookies": [] });
        let cookies = parse_cookies(&response);
        assert!(cookies.is_empty());
    }

    #[test]
    fn parse_cookies_missing_field() {
        let response = serde_json::json!({});
        let cookies = parse_cookies(&response);
        assert!(cookies.is_empty());
    }

    #[test]
    fn plain_text_list_empty() {
        // Just verify it doesn't panic
        print_list_plain(&[]);
    }

    #[test]
    fn plain_text_set() {
        let result = SetResult {
            success: true,
            name: "test".into(),
            domain: "example.com".into(),
        };
        // Just verify it doesn't panic
        print_set_plain(&result);
    }

    #[test]
    fn plain_text_delete() {
        let result = DeleteResult { deleted: 1 };
        // Just verify it doesn't panic
        print_delete_plain(&result);
    }

    #[test]
    fn plain_text_clear() {
        let result = DeleteResult { deleted: 5 };
        // Just verify it doesn't panic
        print_clear_plain(&result);
    }
}
