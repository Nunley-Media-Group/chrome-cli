use std::path::Path;
use std::process::Command;

use serde::Serialize;
use serde_json::{Map, Value};

use agentchrome::connection::{resolve_connection, resolve_target};
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{AuditArgs, AuditCommand, AuditLighthouseArgs, GlobalOpts};

/// Valid Lighthouse category names.
const VALID_CATEGORIES: &[&str] = &[
    "performance",
    "accessibility",
    "best-practices",
    "seo",
    "pwa",
];

pub async fn execute_audit(global: &GlobalOpts, args: &AuditArgs) -> Result<(), AppError> {
    match &args.command {
        AuditCommand::Lighthouse(lh_args) => execute_lighthouse(global, lh_args).await,
    }
}

async fn execute_lighthouse(
    global: &GlobalOpts,
    args: &AuditLighthouseArgs,
) -> Result<(), AppError> {
    // --install-prereqs runs before resolve_connection: the install path does not
    // need an active Chrome session.
    if args.install_prereqs {
        return install_lighthouse_prereqs();
    }

    // 1. Resolve the Chrome connection to get the port.
    let conn = resolve_connection(&global.host, global.port, global.ws_url.as_deref()).await?;

    // 2. Determine the URL to audit.
    let url = if let Some(u) = &args.url {
        u.clone()
    } else {
        // No explicit URL — use the current page's URL.
        let target = resolve_target(
            &conn.host,
            conn.port,
            global.tab.as_deref(),
            global.page_id.as_deref(),
        )
        .await?;
        target.url
    };

    // 3. Find the lighthouse binary.
    find_lighthouse_binary()?;

    // 4. Validate --only categories.
    let categories = validate_categories(args.only.as_deref())?;

    // 5. Build and execute the lighthouse command.
    let mut cmd = std::process::Command::new("lighthouse");
    cmd.arg(&url)
        .arg("--port")
        .arg(conn.port.to_string())
        .arg("--output")
        .arg("json")
        .arg("--chrome-flags=--headless");

    if let Some(ref cats) = categories {
        let joined = cats.join(",");
        cmd.arg(format!("--only-categories={joined}"));
    }

    let output = cmd.output().map_err(|e| AppError {
        message: format!("failed to execute lighthouse: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError {
            message: format!("lighthouse exited with error: {}", stderr.trim()),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    // 6. Parse stdout JSON and extract scores.
    let raw_json: Value = serde_json::from_slice(&output.stdout).map_err(|e| AppError {
        message: format!("failed to parse lighthouse JSON output: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    let scores = extract_scores(&raw_json, categories.as_deref(), &url);

    // 7. Optionally write full report to file.
    if let Some(ref path) = args.output_file {
        write_report(path, &output.stdout)?;
    }

    // 8. Print scores JSON to stdout.
    let json = serde_json::to_string(&scores).map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    println!("{json}");

    Ok(())
}

/// Check that the `lighthouse` binary is available on PATH.
fn find_lighthouse_binary() -> Result<(), AppError> {
    if probe_version("lighthouse").is_some() {
        return Ok(());
    }
    Err(AppError {
        message: LIGHTHOUSE_NOT_FOUND_MESSAGE.to_string(),
        code: ExitCode::GeneralError,
        custom_json: None,
    })
}

/// The `lighthouse binary not found` error message.
///
/// Surfaces both the direct `npm install` hint and the `--install-prereqs`
/// self-service path. Kept as a single `AppError` so one invocation emits
/// exactly one JSON error object on stderr.
const LIGHTHOUSE_NOT_FOUND_MESSAGE: &str = "lighthouse binary not found. Install it with: npm install -g lighthouse\nOr run: agentchrome audit lighthouse --install-prereqs";

/// Probe a binary by running `<bin> --version`. Returns the trimmed stdout
/// on success, or `None` if the binary is not on PATH / exits non-zero.
fn probe_version(bin: &str) -> Option<String> {
    probe_version_with(&|| Command::new(bin))
}

/// Testable variant of `probe_version` — runs the `Command` produced by
/// the factory and returns the trimmed stdout on success.
fn probe_version_with(factory: &dyn Fn() -> Command) -> Option<String> {
    let mut cmd = factory();
    cmd.arg("--version");
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Result payload emitted on stdout after a successful `--install-prereqs` run.
#[derive(Serialize)]
struct InstallPrereqsResult {
    installed: &'static str,
    version: String,
}

fn install_lighthouse_prereqs() -> Result<(), AppError> {
    install_lighthouse_prereqs_with(&npm_factory)
}

/// Return a `Command` for invoking `npm`.
///
/// On Windows, `npm` ships as `npm.cmd` and `CreateProcess` does not honor
/// `PATHEXT`, so we probe the bare name once and fall back to `npm.cmd`.
fn npm_factory() -> Command {
    if cfg!(windows) && Command::new("npm").arg("--version").output().is_err() {
        return Command::new("npm.cmd");
    }
    Command::new("npm")
}

fn install_lighthouse_prereqs_with(npm_factory: &dyn Fn() -> Command) -> Result<(), AppError> {
    if probe_version_with(npm_factory).is_none() {
        return Err(AppError {
            message: "npm not found on PATH — install Node.js first".to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    let output = npm_factory()
        .args(["install", "-g", "lighthouse"])
        .output()
        .map_err(|e| AppError {
            message: format!("Failed to invoke npm: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stderr.is_empty() {
            format!("npm exited with status {}", output.status)
        } else {
            stderr
        };
        return Err(AppError {
            message: format!("Failed to install lighthouse: {detail}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    let version = probe_version("lighthouse").ok_or_else(|| AppError {
        message: "lighthouse installed but not on PATH — open a new shell and retry".to_string(),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    let payload = InstallPrereqsResult {
        installed: "lighthouse",
        version,
    };
    let json = serde_json::to_string(&payload).map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    println!("{json}");

    Ok(())
}

/// Validate the `--only` category filter.
///
/// Returns `None` if no filter was specified (all categories).
/// Returns `Some(vec)` with validated category names.
fn validate_categories(only: Option<&str>) -> Result<Option<Vec<String>>, AppError> {
    let Some(only) = only else {
        return Ok(None);
    };

    let cats: Vec<String> = only.split(',').map(|s| s.trim().to_string()).collect();

    for cat in &cats {
        if !VALID_CATEGORIES.contains(&cat.as_str()) {
            return Err(AppError {
                message: format!(
                    "invalid category '{cat}'. Valid categories: {}",
                    VALID_CATEGORIES.join(", ")
                ),
                code: ExitCode::GeneralError,
                custom_json: None,
            });
        }
    }

    Ok(Some(cats))
}

/// Extract category scores from the Lighthouse JSON output.
fn extract_scores(raw: &Value, categories: Option<&[String]>, url: &str) -> Value {
    let mut result = Map::new();
    result.insert("url".to_string(), Value::String(url.to_string()));

    let cats_to_check: Vec<&str> = match categories {
        Some(cats) => cats.iter().map(String::as_str).collect(),
        None => VALID_CATEGORIES.to_vec(),
    };

    let lh_categories = &raw["categories"];

    for cat in cats_to_check {
        let score = &lh_categories[cat]["score"];
        if let Some(n) = score.as_f64() {
            result.insert(
                cat.to_string(),
                Value::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| {
                    // NaN/Inf can't be represented — fall back to 0
                    serde_json::Number::from(0)
                })),
            );
        } else {
            result.insert(cat.to_string(), Value::Null);
        }
    }

    Value::Object(result)
}

/// Write the raw Lighthouse JSON report to a file.
fn write_report(path: &Path, data: &[u8]) -> Result<(), AppError> {
    std::fs::write(path, data).map_err(|e| AppError {
        message: format!("failed to write report to {}: {e}", path.display()),
        code: ExitCode::GeneralError,
        custom_json: None,
    })
}

// =============================================================================
// Script runner adapter
// =============================================================================

/// Run an `audit` command against an existing session and return a JSON value.
///
/// # Errors
///
/// Propagates `AppError` from the underlying audit logic.
#[allow(dead_code)]
pub async fn run_from_session(
    _managed: &mut agentchrome::connection::ManagedSession,
    global: &GlobalOpts,
    args: &AuditArgs,
) -> Result<serde_json::Value, AppError> {
    execute_audit(global, args).await?;
    Ok(serde_json::json!({"executed": true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_categories_valid() {
        let result = validate_categories(Some("performance,accessibility")).unwrap();
        assert_eq!(
            result,
            Some(vec!["performance".to_string(), "accessibility".to_string()])
        );
    }

    #[test]
    fn validate_categories_invalid() {
        let result = validate_categories(Some("performance,bogus"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("bogus"));
    }

    #[test]
    fn validate_categories_none() {
        let result = validate_categories(None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn validate_categories_all_valid() {
        let result =
            validate_categories(Some("performance,accessibility,best-practices,seo,pwa")).unwrap();
        assert_eq!(result.as_ref().unwrap().len(), 5);
    }

    #[test]
    fn validate_categories_trimmed() {
        let result = validate_categories(Some(" performance , seo ")).unwrap();
        assert_eq!(
            result,
            Some(vec!["performance".to_string(), "seo".to_string()])
        );
    }

    #[test]
    fn extract_scores_all_categories() {
        let raw = json!({
            "categories": {
                "performance": {"score": 0.95},
                "accessibility": {"score": 0.88},
                "best-practices": {"score": 1.0},
                "seo": {"score": 0.92},
                "pwa": {"score": 0.5}
            }
        });

        let scores = extract_scores(&raw, None, "https://example.com");
        let obj = scores.as_object().unwrap();

        assert_eq!(obj["url"], "https://example.com");
        assert_eq!(obj["performance"], 0.95);
        assert_eq!(obj["accessibility"], 0.88);
        assert_eq!(obj["best-practices"], 1.0);
        assert_eq!(obj["seo"], 0.92);
        assert_eq!(obj["pwa"], 0.5);
    }

    #[test]
    fn extract_scores_filtered() {
        let raw = json!({
            "categories": {
                "performance": {"score": 0.95},
                "accessibility": {"score": 0.88},
                "best-practices": {"score": 1.0},
                "seo": {"score": 0.92},
                "pwa": {"score": 0.5}
            }
        });

        let filter = vec!["performance".to_string(), "seo".to_string()];
        let scores = extract_scores(&raw, Some(&filter), "https://example.com");
        let obj = scores.as_object().unwrap();

        assert_eq!(obj.len(), 3); // url + 2 categories
        assert_eq!(obj["performance"], 0.95);
        assert_eq!(obj["seo"], 0.92);
        assert!(!obj.contains_key("accessibility"));
        assert!(!obj.contains_key("best-practices"));
        assert!(!obj.contains_key("pwa"));
    }

    #[test]
    fn extract_scores_null_score() {
        let raw = json!({
            "categories": {
                "performance": {"score": null},
                "accessibility": {"score": 0.88}
            }
        });

        let filter = vec!["performance".to_string(), "accessibility".to_string()];
        let scores = extract_scores(&raw, Some(&filter), "https://example.com");
        let obj = scores.as_object().unwrap();

        assert!(obj["performance"].is_null());
        assert_eq!(obj["accessibility"], 0.88);
    }

    #[test]
    fn lighthouse_not_found_message_mentions_both_paths() {
        assert!(LIGHTHOUSE_NOT_FOUND_MESSAGE.contains("npm install -g lighthouse"));
        assert!(LIGHTHOUSE_NOT_FOUND_MESSAGE.contains("--install-prereqs"));
    }

    #[test]
    fn install_prereqs_errors_when_npm_missing() {
        let npm = || Command::new("/nonexistent/definitely-not-npm-binary-xyz");
        let err = install_lighthouse_prereqs_with(&npm).unwrap_err();
        assert!(
            err.message.contains("npm not found on PATH"),
            "expected npm-missing error, got: {}",
            err.message
        );
        assert!(err.message.contains("Node.js"));
    }

    #[test]
    #[cfg(unix)]
    fn install_prereqs_errors_when_npm_install_fails() {
        // `sh -c SCRIPT $0 $1 $2...` — the factory builds `sh -c SCRIPT sh` so that
        // $0=sh and $1 is whatever probe/install appends. --version succeeds, install fails.
        let npm = || {
            let mut c = Command::new("sh");
            c.arg("-c")
                .arg(r#"case "$1" in --version) echo 10.0.0; exit 0;; install) echo "npm err" >&2; exit 1;; esac"#)
                .arg("sh");
            c
        };
        let err = install_lighthouse_prereqs_with(&npm).unwrap_err();
        assert!(
            err.message.contains("Failed to install lighthouse"),
            "got: {}",
            err.message
        );
        assert!(
            err.message.contains("npm err"),
            "expected stderr capture, got: {}",
            err.message
        );
    }

    #[test]
    fn extract_scores_missing_categories_key() {
        let raw = json!({});

        let scores = extract_scores(&raw, None, "https://example.com");
        let obj = scores.as_object().unwrap();

        assert_eq!(obj["url"], "https://example.com");
        // All categories should be null when the categories key is missing
        for cat in VALID_CATEGORIES {
            assert!(obj[*cat].is_null(), "expected null for {cat}");
        }
    }
}
