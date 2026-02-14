use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use chrome_cli::cdp::{CdpClient, CdpConfig};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{
    GlobalOpts, NetworkArgs, NetworkCommand, NetworkFollowArgs, NetworkGetArgs, NetworkListArgs,
};

// =============================================================================
// Output types
// =============================================================================

/// A network request summary for list mode.
#[derive(Clone, Debug, Serialize)]
pub struct NetworkRequestSummary {
    id: usize,
    method: String,
    url: String,
    status: Option<u16>,
    #[serde(rename = "type")]
    resource_type: String,
    size: Option<u64>,
    duration_ms: Option<f64>,
    timestamp: String,
}

/// Full detail of a single network request.
#[derive(Debug, Serialize)]
struct NetworkRequestDetail {
    id: usize,
    request: RequestInfo,
    response: ResponseInfo,
    timing: TimingInfo,
    #[serde(rename = "redirect_chain")]
    redirect_chain: Vec<RedirectEntry>,
    #[serde(rename = "type")]
    resource_type: String,
    size: Option<u64>,
    duration_ms: Option<f64>,
    timestamp: String,
}

/// Request section of a detailed network request.
#[derive(Debug, Serialize)]
struct RequestInfo {
    method: String,
    url: String,
    headers: serde_json::Value,
    body: Option<String>,
}

/// Response section of a detailed network request.
#[derive(Debug, Serialize)]
struct ResponseInfo {
    status: Option<u16>,
    status_text: String,
    headers: serde_json::Value,
    body: Option<String>,
    binary: bool,
    truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
}

/// Timing breakdown for a network request.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Serialize)]
struct TimingInfo {
    dns_ms: f64,
    connect_ms: f64,
    tls_ms: f64,
    ttfb_ms: f64,
    download_ms: f64,
}

/// A redirect hop entry.
#[derive(Clone, Debug, Serialize)]
struct RedirectEntry {
    url: String,
    status: u16,
}

/// A network request emitted by `network follow` (one JSON line per request).
#[derive(Debug, Serialize)]
struct NetworkStreamEvent {
    method: String,
    url: String,
    status: Option<u16>,
    #[serde(rename = "type")]
    resource_type: String,
    size: Option<u64>,
    duration_ms: Option<f64>,
    timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_headers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_headers: Option<serde_json::Value>,
}

/// Raw collected event data before correlation.
struct RawNetworkEvent {
    params: serde_json::Value,
    event_type: NetworkEventType,
    navigation_id: u32,
}

/// Types of network events we track.
enum NetworkEventType {
    RequestWillBeSent,
    ResponseReceived,
    LoadingFinished,
    LoadingFailed,
}

/// Builder for accumulating network request data from multiple CDP events.
struct NetworkRequestBuilder {
    cdp_request_id: String,
    assigned_id: usize,
    method: String,
    url: String,
    resource_type: String,
    timestamp: f64,
    request_headers: serde_json::Value,
    status: Option<u16>,
    status_text: String,
    response_headers: serde_json::Value,
    mime_type: Option<String>,
    encoded_data_length: Option<u64>,
    timing: Option<serde_json::Value>,
    redirect_chain: Vec<RedirectEntry>,
    completed: bool,
    failed: bool,
    error_text: Option<String>,
    navigation_id: u32,
    loading_finished_timestamp: Option<f64>,
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

fn print_list_plain(requests: &[NetworkRequestSummary]) {
    for req in requests {
        let status_str = req
            .status
            .map_or_else(|| "---".to_string(), |s| s.to_string());
        let size_str = req
            .size
            .map_or_else(|| "-".to_string(), |s| format!("{s}B"));
        let dur_str = req
            .duration_ms
            .map_or_else(|| "-".to_string(), |d| format!("{d:.1}ms"));
        println!(
            "{} {} {} {} {}",
            req.method, req.url, status_str, size_str, dur_str
        );
    }
}

fn print_detail_plain(detail: &NetworkRequestDetail) {
    println!("{} {}", detail.request.method, detail.request.url);
    let status_str = detail
        .response
        .status
        .map_or_else(|| "---".to_string(), |s| s.to_string());
    println!("  Status: {} {}", status_str, detail.response.status_text);
    println!("  Type: {}", detail.resource_type);
    println!("  Timestamp: {}", detail.timestamp);
    if let Some(size) = detail.size {
        println!("  Size: {size} bytes");
    }
    if let Some(dur) = detail.duration_ms {
        println!("  Duration: {dur:.1}ms");
    }
    println!(
        "  Timing: DNS={:.1}ms Connect={:.1}ms TLS={:.1}ms TTFB={:.1}ms Download={:.1}ms",
        detail.timing.dns_ms,
        detail.timing.connect_ms,
        detail.timing.tls_ms,
        detail.timing.ttfb_ms,
        detail.timing.download_ms,
    );
    if !detail.redirect_chain.is_empty() {
        println!("  Redirects:");
        for hop in &detail.redirect_chain {
            println!("    {} -> {}", hop.status, hop.url);
        }
    }
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
    let target = resolve_target(&conn.host, conn.port, global.tab.as_deref()).await?;

    let config = cdp_config(global);
    let client = CdpClient::connect(&conn.ws_url, config).await?;
    let session = client.create_session(&target.id).await?;
    let managed = ManagedSession::new(session);

    Ok((client, managed))
}

// =============================================================================
// Helpers
// =============================================================================

/// Maximum inline body size (matching MCP server limit).
const MAX_INLINE_BODY_SIZE: usize = 10_000;

/// Convert a CDP timestamp (seconds since epoch, floating point) to ISO 8601 string.
///
/// CDP `Network.requestWillBeSent` provides timestamps as seconds since epoch (not milliseconds).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::similar_names
)]
fn timestamp_to_iso(ts: f64) -> String {
    // CDP Network timestamps are in seconds since epoch (unlike Runtime which uses ms)
    let total_ms = (ts * 1000.0) as u64;
    let secs = total_ms / 1000;
    let ms_part = total_ms % 1000;

    // Civil date/time from epoch seconds (Howard Hinnant's algorithm)
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    let z = days_since_epoch as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}.{ms_part:03}Z")
}

/// Parse a status filter string. Supports exact ("404") or wildcard ("4xx").
fn parse_status_filter(status_str: &str) -> StatusFilter {
    let lower = status_str.to_lowercase();
    if lower.len() == 3 && lower.ends_with("xx") {
        if let Some(prefix_char) = lower.chars().next() {
            if let Some(digit) = prefix_char.to_digit(10) {
                #[allow(clippy::cast_possible_truncation)]
                let base = (digit as u16) * 100;
                return StatusFilter::Range(base, base + 99);
            }
        }
    }
    if let Ok(code) = status_str.parse::<u16>() {
        StatusFilter::Exact(code)
    } else {
        // Invalid filter, match nothing
        StatusFilter::Exact(0)
    }
}

/// Status code filter variant.
enum StatusFilter {
    Exact(u16),
    Range(u16, u16),
}

impl StatusFilter {
    fn matches(&self, code: u16) -> bool {
        match self {
            Self::Exact(target) => code == *target,
            Self::Range(low, high) => code >= *low && code <= *high,
        }
    }
}

/// Resolve `--type` into an optional type filter list.
fn resolve_type_filter(type_arg: Option<&str>) -> Option<Vec<String>> {
    type_arg.map(|types| types.split(',').map(|t| t.trim().to_lowercase()).collect())
}

/// Filter requests by resource type.
fn filter_by_type(
    requests: Vec<NetworkRequestSummary>,
    types: &[String],
) -> Vec<NetworkRequestSummary> {
    requests
        .into_iter()
        .filter(|r| types.iter().any(|t| t == &r.resource_type.to_lowercase()))
        .collect()
}

/// Filter requests by URL substring.
fn filter_by_url(
    requests: Vec<NetworkRequestSummary>,
    pattern: &str,
) -> Vec<NetworkRequestSummary> {
    requests
        .into_iter()
        .filter(|r| r.url.contains(pattern))
        .collect()
}

/// Filter requests by HTTP status code.
fn filter_by_status(
    requests: Vec<NetworkRequestSummary>,
    status_filter: &StatusFilter,
) -> Vec<NetworkRequestSummary> {
    requests
        .into_iter()
        .filter(|r| r.status.is_some_and(|s| status_filter.matches(s)))
        .collect()
}

/// Filter requests by HTTP method (case-insensitive).
fn filter_by_method(
    requests: Vec<NetworkRequestSummary>,
    method: &str,
) -> Vec<NetworkRequestSummary> {
    let upper = method.to_uppercase();
    requests
        .into_iter()
        .filter(|r| r.method.to_uppercase() == upper)
        .collect()
}

/// Apply pagination (limit + page offset).
fn paginate(
    requests: Vec<NetworkRequestSummary>,
    limit: usize,
    page: usize,
) -> Vec<NetworkRequestSummary> {
    let offset = page * limit;
    requests.into_iter().skip(offset).take(limit).collect()
}

/// Extract timing info from CDP `response.timing` object.
fn extract_timing(timing: &serde_json::Value) -> TimingInfo {
    let dns_start = timing["dnsStart"].as_f64().unwrap_or(-1.0);
    let dns_end = timing["dnsEnd"].as_f64().unwrap_or(-1.0);
    let connect_start = timing["connectStart"].as_f64().unwrap_or(-1.0);
    let connect_end = timing["connectEnd"].as_f64().unwrap_or(-1.0);
    let ssl_start = timing["sslStart"].as_f64().unwrap_or(-1.0);
    let ssl_end = timing["sslEnd"].as_f64().unwrap_or(-1.0);
    let send_end = timing["sendEnd"].as_f64().unwrap_or(-1.0);
    let receive_headers_end = timing["receiveHeadersEnd"].as_f64().unwrap_or(-1.0);

    let dns_ms = if dns_start >= 0.0 && dns_end >= 0.0 {
        dns_end - dns_start
    } else {
        0.0
    };
    let connect_ms = if connect_start >= 0.0 && connect_end >= 0.0 {
        connect_end - connect_start
    } else {
        0.0
    };
    let tls_ms = if ssl_start >= 0.0 && ssl_end >= 0.0 {
        ssl_end - ssl_start
    } else {
        0.0
    };
    let ttfb_ms = if send_end >= 0.0 && receive_headers_end >= 0.0 {
        receive_headers_end - send_end
    } else {
        0.0
    };

    TimingInfo {
        dns_ms,
        connect_ms,
        tls_ms,
        ttfb_ms,
        download_ms: 0.0, // Calculated separately from loading finished
    }
}

/// Check if a MIME type represents a binary resource.
fn is_binary_mime(mime: &str) -> bool {
    let lower = mime.to_lowercase();
    lower.starts_with("image/")
        || lower.starts_with("audio/")
        || lower.starts_with("video/")
        || lower.starts_with("application/octet-stream")
        || lower.starts_with("application/zip")
        || lower.starts_with("application/gzip")
        || lower.starts_with("application/pdf")
        || lower.starts_with("font/")
        || lower.starts_with("application/wasm")
}

/// Save body content to a file.
fn save_body_to_file(path: &Path, content: &str) -> Result<(), AppError> {
    std::fs::write(path, content).map_err(|e| AppError {
        message: format!("Failed to write to {}: {e}", path.display()),
        code: ExitCode::GeneralError,
    })
}

/// Save binary body (base64 decoded) to a file.
fn save_binary_body_to_file(path: &Path, base64_content: &str) -> Result<(), AppError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_content)
        .map_err(|e| AppError {
            message: format!("Failed to decode base64 body: {e}"),
            code: ExitCode::GeneralError,
        })?;
    std::fs::write(path, bytes).map_err(|e| AppError {
        message: format!("Failed to write to {}: {e}", path.display()),
        code: ExitCode::GeneralError,
    })
}

// =============================================================================
// Event collection and correlation
// =============================================================================

/// Collect raw network events from CDP subscriptions with a 100ms idle timeout.
#[allow(clippy::too_many_lines)]
async fn collect_and_correlate(
    managed: &mut ManagedSession,
    include_preserved: bool,
) -> Result<(Vec<NetworkRequestBuilder>, u32), AppError> {
    // Enable required domains
    managed.ensure_domain("Network").await?;
    managed.ensure_domain("Page").await?;

    // Subscribe to all needed events
    let mut request_rx = managed
        .subscribe("Network.requestWillBeSent")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Network.requestWillBeSent: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let mut response_rx = managed
        .subscribe("Network.responseReceived")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Network.responseReceived: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let mut finished_rx = managed
        .subscribe("Network.loadingFinished")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Network.loadingFinished: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let mut failed_rx = managed
        .subscribe("Network.loadingFailed")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Network.loadingFailed: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let mut nav_rx = managed
        .subscribe("Page.frameNavigated")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Page.frameNavigated: {e}"),
            code: ExitCode::GeneralError,
        })?;

    // Drain events with 100ms idle timeout
    let mut raw_events: Vec<RawNetworkEvent> = Vec::new();
    let mut current_nav_id: u32 = 0;
    let drain_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(100);

    loop {
        let remaining = drain_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        tokio::select! {
            event = request_rx.recv() => {
                match event {
                    Some(ev) => raw_events.push(RawNetworkEvent {
                        params: ev.params,
                        event_type: NetworkEventType::RequestWillBeSent,
                        navigation_id: current_nav_id,
                    }),
                    None => break,
                }
            }
            event = response_rx.recv() => {
                match event {
                    Some(ev) => raw_events.push(RawNetworkEvent {
                        params: ev.params,
                        event_type: NetworkEventType::ResponseReceived,
                        navigation_id: current_nav_id,
                    }),
                    None => break,
                }
            }
            event = finished_rx.recv() => {
                match event {
                    Some(ev) => raw_events.push(RawNetworkEvent {
                        params: ev.params,
                        event_type: NetworkEventType::LoadingFinished,
                        navigation_id: current_nav_id,
                    }),
                    None => break,
                }
            }
            event = failed_rx.recv() => {
                match event {
                    Some(ev) => raw_events.push(RawNetworkEvent {
                        params: ev.params,
                        event_type: NetworkEventType::LoadingFailed,
                        navigation_id: current_nav_id,
                    }),
                    None => break,
                }
            }
            event = nav_rx.recv() => {
                match event {
                    Some(_) => current_nav_id += 1,
                    None => break,
                }
            }
            () = tokio::time::sleep(remaining) => break,
        }
    }

    // Correlate events into builders
    let mut builders: HashMap<String, NetworkRequestBuilder> = HashMap::new();
    let mut next_id: usize = 0;

    for event in &raw_events {
        let request_id = event.params["requestId"].as_str().unwrap_or("").to_string();
        if request_id.is_empty() {
            continue;
        }

        match event.event_type {
            NetworkEventType::RequestWillBeSent => {
                // Check if this is a redirect (existing entry for same requestId)
                if let Some(existing) = builders.get_mut(&request_id) {
                    // Record redirect hop
                    let redirect_status = event.params["redirectResponse"]["status"]
                        .as_u64()
                        .unwrap_or(0);
                    let redirect_url = existing.url.clone();
                    #[allow(clippy::cast_possible_truncation)]
                    existing.redirect_chain.push(RedirectEntry {
                        url: redirect_url,
                        status: redirect_status as u16,
                    });
                    // Update the builder with new URL/method
                    existing.url = event.params["request"]["url"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    existing.method = event.params["request"]["method"]
                        .as_str()
                        .unwrap_or("GET")
                        .to_string();
                    existing.request_headers = event.params["request"]["headers"].clone();
                } else {
                    let builder = NetworkRequestBuilder {
                        cdp_request_id: request_id.clone(),
                        assigned_id: next_id,
                        method: event.params["request"]["method"]
                            .as_str()
                            .unwrap_or("GET")
                            .to_string(),
                        url: event.params["request"]["url"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        resource_type: event.params["type"]
                            .as_str()
                            .unwrap_or("Other")
                            .to_lowercase(),
                        timestamp: event.params["timestamp"].as_f64().unwrap_or(0.0),
                        request_headers: event.params["request"]["headers"].clone(),
                        status: None,
                        status_text: String::new(),
                        response_headers: serde_json::Value::Null,
                        mime_type: None,
                        encoded_data_length: None,
                        timing: None,
                        redirect_chain: Vec::new(),
                        completed: false,
                        failed: false,
                        error_text: None,
                        navigation_id: event.navigation_id,
                        loading_finished_timestamp: None,
                    };
                    builders.insert(request_id, builder);
                    next_id += 1;
                }
            }
            NetworkEventType::ResponseReceived => {
                if let Some(builder) = builders.get_mut(&request_id) {
                    #[allow(clippy::cast_possible_truncation)]
                    let status = event.params["response"]["status"]
                        .as_u64()
                        .map(|s| s as u16);
                    builder.status = status;
                    builder.status_text = event.params["response"]["statusText"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    builder.response_headers = event.params["response"]["headers"].clone();
                    builder.mime_type = event.params["response"]["mimeType"]
                        .as_str()
                        .map(String::from);
                    builder.timing = Some(event.params["response"]["timing"].clone());
                }
            }
            NetworkEventType::LoadingFinished => {
                if let Some(builder) = builders.get_mut(&request_id) {
                    builder.completed = true;
                    builder.encoded_data_length = event.params["encodedDataLength"].as_u64();
                    builder.loading_finished_timestamp = event.params["timestamp"].as_f64();
                }
            }
            NetworkEventType::LoadingFailed => {
                if let Some(builder) = builders.get_mut(&request_id) {
                    builder.failed = true;
                    builder.error_text = event.params["errorText"].as_str().map(String::from);
                }
            }
        }
    }

    // Filter by navigation if needed
    let builders_vec: Vec<NetworkRequestBuilder> = if include_preserved {
        builders.into_values().collect()
    } else {
        builders
            .into_values()
            .filter(|b| b.navigation_id == current_nav_id)
            .collect()
    };

    Ok((builders_vec, current_nav_id))
}

/// Convert a builder into a summary for list output.
fn builder_to_summary(builder: &NetworkRequestBuilder) -> NetworkRequestSummary {
    let duration_ms = builder
        .loading_finished_timestamp
        .map(|end_ts| (end_ts - builder.timestamp) * 1000.0);

    NetworkRequestSummary {
        id: builder.assigned_id,
        method: builder.method.clone(),
        url: builder.url.clone(),
        status: builder.status,
        resource_type: builder.resource_type.clone(),
        size: builder.encoded_data_length,
        duration_ms,
        timestamp: timestamp_to_iso(builder.timestamp),
    }
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `network` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_network(global: &GlobalOpts, args: &NetworkArgs) -> Result<(), AppError> {
    match &args.command {
        NetworkCommand::List(list_args) => execute_list(global, list_args).await,
        NetworkCommand::Get(get_args) => execute_get(global, get_args).await,
        NetworkCommand::Follow(follow_args) => execute_follow(global, follow_args).await,
    }
}

// =============================================================================
// List
// =============================================================================

async fn execute_list(global: &GlobalOpts, args: &NetworkListArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    let (builders, _nav_id) = collect_and_correlate(&mut managed, args.include_preserved).await?;

    // Convert to summaries and sort by assigned_id
    let mut requests: Vec<NetworkRequestSummary> =
        builders.iter().map(builder_to_summary).collect();
    requests.sort_by_key(|r| r.id);

    // Apply filters
    if let Some(ref types) = resolve_type_filter(args.r#type.as_deref()) {
        requests = filter_by_type(requests, types);
    }
    if let Some(ref url_pattern) = args.url {
        requests = filter_by_url(requests, url_pattern);
    }
    if let Some(ref status_str) = args.status {
        let status_filter = parse_status_filter(status_str);
        requests = filter_by_status(requests, &status_filter);
    }
    if let Some(ref method) = args.method {
        requests = filter_by_method(requests, method);
    }

    // Paginate
    requests = paginate(requests, args.limit, args.page);

    // Output
    if global.output.plain {
        print_list_plain(&requests);
        return Ok(());
    }
    print_output(&requests, &global.output)
}

// =============================================================================
// Get
// =============================================================================

#[allow(clippy::too_many_lines)]
async fn execute_get(global: &GlobalOpts, args: &NetworkGetArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    let (builders, _nav_id) = collect_and_correlate(&mut managed, true).await?;

    // Find builder by assigned numeric ID
    #[allow(clippy::cast_possible_truncation)]
    let target_id = args.req_id as usize;
    let builder = builders
        .iter()
        .find(|b| b.assigned_id == target_id)
        .ok_or_else(|| AppError {
            message: format!("Network request {target_id} not found"),
            code: ExitCode::GeneralError,
        })?;

    // Fetch request body for POST/PUT
    let request_body =
        if builder.method == "POST" || builder.method == "PUT" || builder.method == "PATCH" {
            match managed
                .send_command(
                    "Network.getRequestPostData",
                    Some(serde_json::json!({ "requestId": builder.cdp_request_id })),
                )
                .await
            {
                Ok(result) => result["postData"].as_str().map(String::from),
                Err(_) => None,
            }
        } else {
            None
        };

    // Fetch response body
    let (response_body, is_binary, is_truncated) = match managed
        .send_command(
            "Network.getResponseBody",
            Some(serde_json::json!({ "requestId": builder.cdp_request_id })),
        )
        .await
    {
        Ok(result) => {
            let base64_encoded = result["base64Encoded"].as_bool().unwrap_or(false);
            let body_str = result["body"].as_str().unwrap_or("");

            if base64_encoded {
                // Binary content — save to file if requested, don't inline
                if let Some(ref save_path) = args.save_response {
                    save_binary_body_to_file(save_path, body_str)?;
                }
                (None, true, false)
            } else if body_str.len() > MAX_INLINE_BODY_SIZE {
                // Save full body to file if requested
                if let Some(ref save_path) = args.save_response {
                    save_body_to_file(save_path, body_str)?;
                }
                let truncated = body_str[..MAX_INLINE_BODY_SIZE].to_string();
                (Some(truncated), false, true)
            } else {
                if let Some(ref save_path) = args.save_response {
                    save_body_to_file(save_path, body_str)?;
                }
                (Some(body_str.to_string()), false, false)
            }
        }
        Err(_) => (None, false, false),
    };

    // Save request body if requested
    if let Some(ref save_path) = args.save_request {
        if let Some(ref body) = request_body {
            save_body_to_file(save_path, body)?;
        }
    }

    // Build timing info
    let timing = builder.timing.as_ref().map_or_else(
        || TimingInfo {
            dns_ms: 0.0,
            connect_ms: 0.0,
            tls_ms: 0.0,
            ttfb_ms: 0.0,
            download_ms: 0.0,
        },
        |t| {
            let mut ti = extract_timing(t);
            // Calculate download time from timing + loading finished
            if let Some(end_ts) = builder.loading_finished_timestamp {
                let request_time = t["requestTime"].as_f64().unwrap_or(0.0);
                let receive_headers_end = t["receiveHeadersEnd"].as_f64().unwrap_or(0.0);
                if request_time > 0.0 && receive_headers_end > 0.0 {
                    let headers_done = request_time + receive_headers_end / 1000.0;
                    ti.download_ms = (end_ts - headers_done) * 1000.0;
                    if ti.download_ms < 0.0 {
                        ti.download_ms = 0.0;
                    }
                }
            }
            ti
        },
    );

    let duration_ms = builder
        .loading_finished_timestamp
        .map(|end_ts| (end_ts - builder.timestamp) * 1000.0);

    let mime_for_binary_check = builder.mime_type.as_deref().unwrap_or("");
    let binary = is_binary || is_binary_mime(mime_for_binary_check);

    let detail = NetworkRequestDetail {
        id: builder.assigned_id,
        request: RequestInfo {
            method: builder.method.clone(),
            url: builder.url.clone(),
            headers: builder.request_headers.clone(),
            body: request_body,
        },
        response: ResponseInfo {
            status: builder.status,
            status_text: builder.status_text.clone(),
            headers: builder.response_headers.clone(),
            body: if binary { None } else { response_body },
            binary,
            truncated: is_truncated,
            mime_type: builder.mime_type.clone(),
        },
        timing,
        redirect_chain: builder.redirect_chain.clone(),
        resource_type: builder.resource_type.clone(),
        size: builder.encoded_data_length,
        duration_ms,
        timestamp: timestamp_to_iso(builder.timestamp),
    };

    if global.output.plain {
        print_detail_plain(&detail);
        return Ok(());
    }
    print_output(&detail, &global.output)
}

// =============================================================================
// Follow: streaming mode
// =============================================================================

#[allow(clippy::too_many_lines)]
async fn execute_follow(global: &GlobalOpts, args: &NetworkFollowArgs) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;

    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Enable required domains
    managed.ensure_domain("Network").await?;

    // Subscribe to network events
    let mut request_rx = managed
        .subscribe("Network.requestWillBeSent")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Network.requestWillBeSent: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let mut response_rx = managed
        .subscribe("Network.responseReceived")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Network.responseReceived: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let mut finished_rx = managed
        .subscribe("Network.loadingFinished")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Network.loadingFinished: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let mut failed_rx = managed
        .subscribe("Network.loadingFailed")
        .await
        .map_err(|e| AppError {
            message: format!("Failed to subscribe to Network.loadingFailed: {e}"),
            code: ExitCode::GeneralError,
        })?;

    let type_filter = resolve_type_filter(args.r#type.as_deref());
    let url_filter = args.url.as_deref();
    let method_filter = args.method.as_deref().map(str::to_uppercase);

    let timeout_duration = args.timeout.map(Duration::from_millis);
    let deadline = timeout_duration.map(|d| tokio::time::Instant::now() + d);

    // In-flight request tracking for correlation
    let mut in_flight: HashMap<String, InFlightRequest> = HashMap::new();

    loop {
        tokio::select! {
            event = request_rx.recv() => {
                match event {
                    Some(ev) => {
                        let request_id = ev.params["requestId"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();
                        if request_id.is_empty() {
                            continue;
                        }
                        in_flight.insert(request_id, InFlightRequest {
                            method: ev.params["request"]["method"]
                                .as_str()
                                .unwrap_or("GET")
                                .to_string(),
                            url: ev.params["request"]["url"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                            resource_type: ev.params["type"]
                                .as_str()
                                .unwrap_or("other")
                                .to_lowercase(),
                            timestamp: ev.params["timestamp"].as_f64().unwrap_or(0.0),
                            request_headers: ev.params["request"]["headers"].clone(),
                            response_headers: serde_json::Value::Null,
                            status: None,
                        });
                    }
                    None => {
                        return Err(AppError {
                            message: "CDP connection closed".to_string(),
                            code: ExitCode::ConnectionError,
                        });
                    }
                }
            }
            event = response_rx.recv() => {
                match event {
                    Some(ev) => {
                        let request_id = ev.params["requestId"]
                            .as_str()
                            .unwrap_or("");
                        if let Some(req) = in_flight.get_mut(request_id) {
                            #[allow(clippy::cast_possible_truncation)]
                            let status = ev.params["response"]["status"]
                                .as_u64()
                                .map(|s| s as u16);
                            req.status = status;
                            req.response_headers = ev.params["response"]["headers"].clone();
                        }
                    }
                    None => {
                        return Err(AppError {
                            message: "CDP connection closed".to_string(),
                            code: ExitCode::ConnectionError,
                        });
                    }
                }
            }
            event = finished_rx.recv() => {
                match event {
                    Some(ev) => {
                        let request_id = ev.params["requestId"]
                            .as_str()
                            .unwrap_or("");
                        let size = ev.params["encodedDataLength"].as_u64();
                        let end_timestamp = ev.params["timestamp"].as_f64();

                        if let Some(req) = in_flight.remove(request_id) {
                            emit_stream_event(
                                &req, size, end_timestamp, type_filter.as_deref(),
                                url_filter, method_filter.as_deref(), args.verbose,
                            );
                        }
                    }
                    None => {
                        return Err(AppError {
                            message: "CDP connection closed".to_string(),
                            code: ExitCode::ConnectionError,
                        });
                    }
                }
            }
            event = failed_rx.recv() => {
                match event {
                    Some(ev) => {
                        let request_id = ev.params["requestId"]
                            .as_str()
                            .unwrap_or("");
                        if let Some(req) = in_flight.remove(request_id) {
                            emit_stream_event(
                                &req, None, None, type_filter.as_deref(),
                                url_filter, method_filter.as_deref(), args.verbose,
                            );
                        }
                    }
                    None => {
                        return Err(AppError {
                            message: "CDP connection closed".to_string(),
                            code: ExitCode::ConnectionError,
                        });
                    }
                }
            }
            () = async {
                if let Some(d) = deadline {
                    tokio::time::sleep_until(d).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                // Timeout expired
                break;
            }
            _ = tokio::signal::ctrl_c() => {
                // Ctrl+C
                break;
            }
        }
    }

    Ok(())
}

/// In-flight request state for follow mode correlation.
struct InFlightRequest {
    method: String,
    url: String,
    resource_type: String,
    timestamp: f64,
    request_headers: serde_json::Value,
    response_headers: serde_json::Value,
    status: Option<u16>,
}

/// Emit a stream event for a completed request (if it passes filters).
fn emit_stream_event(
    req: &InFlightRequest,
    size: Option<u64>,
    end_timestamp: Option<f64>,
    type_filter: Option<&[String]>,
    url_filter: Option<&str>,
    method_filter: Option<&str>,
    verbose: bool,
) {
    // Apply filters
    if let Some(types) = type_filter {
        if !types.iter().any(|t| t == &req.resource_type.to_lowercase()) {
            return;
        }
    }
    if let Some(pattern) = url_filter {
        if !req.url.contains(pattern) {
            return;
        }
    }
    if let Some(method) = method_filter {
        if req.method.to_uppercase() != method {
            return;
        }
    }

    let duration_ms = end_timestamp.map(|end| (end - req.timestamp) * 1000.0);

    let event = NetworkStreamEvent {
        method: req.method.clone(),
        url: req.url.clone(),
        status: req.status,
        resource_type: req.resource_type.clone(),
        size,
        duration_ms,
        timestamp: timestamp_to_iso(req.timestamp),
        request_headers: if verbose {
            Some(req.request_headers.clone())
        } else {
            None
        },
        response_headers: if verbose {
            Some(req.response_headers.clone())
        } else {
            None
        },
    };

    let json = serde_json::to_string(&event).unwrap_or_default();
    println!("{json}");
    let _ = std::io::stdout().flush();
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // NetworkRequestSummary serialization
    // =========================================================================

    #[test]
    fn network_request_summary_serialization() {
        let req = NetworkRequestSummary {
            id: 0,
            method: "GET".to_string(),
            url: "https://example.com/api/data".to_string(),
            status: Some(200),
            resource_type: "xhr".to_string(),
            size: Some(1234),
            duration_ms: Some(45.2),
            timestamp: "2026-02-14T12:00:00.000Z".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert_eq!(json["id"], 0);
        assert_eq!(json["method"], "GET");
        assert_eq!(json["url"], "https://example.com/api/data");
        assert_eq!(json["status"], 200);
        assert_eq!(json["type"], "xhr");
        assert_eq!(json["size"], 1234);
        assert_eq!(json["timestamp"], "2026-02-14T12:00:00.000Z");
        // Verify "type" field, not "resource_type"
        assert!(json.get("resource_type").is_none());
    }

    #[test]
    fn network_request_summary_null_fields() {
        let req = NetworkRequestSummary {
            id: 1,
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            status: None,
            resource_type: "document".to_string(),
            size: None,
            duration_ms: None,
            timestamp: String::new(),
        };
        let json: serde_json::Value = serde_json::to_value(&req).unwrap();
        assert!(json["status"].is_null());
        assert!(json["size"].is_null());
        assert!(json["duration_ms"].is_null());
    }

    // =========================================================================
    // NetworkRequestDetail serialization
    // =========================================================================

    #[test]
    fn network_request_detail_serialization() {
        let detail = NetworkRequestDetail {
            id: 1,
            request: RequestInfo {
                method: "POST".to_string(),
                url: "https://example.com/api".to_string(),
                headers: serde_json::json!({"Content-Type": "application/json"}),
                body: Some("{\"key\":\"value\"}".to_string()),
            },
            response: ResponseInfo {
                status: Some(200),
                status_text: "OK".to_string(),
                headers: serde_json::json!({"Content-Type": "application/json"}),
                body: Some("{\"result\":\"ok\"}".to_string()),
                binary: false,
                truncated: false,
                mime_type: Some("application/json".to_string()),
            },
            timing: TimingInfo {
                dns_ms: 5.0,
                connect_ms: 10.0,
                tls_ms: 15.0,
                ttfb_ms: 50.0,
                download_ms: 20.0,
            },
            redirect_chain: vec![RedirectEntry {
                url: "http://example.com/api".to_string(),
                status: 301,
            }],
            resource_type: "xhr".to_string(),
            size: Some(1234),
            duration_ms: Some(100.2),
            timestamp: "2026-02-14T12:00:00.000Z".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&detail).unwrap();
        assert_eq!(json["id"], 1);
        assert_eq!(json["request"]["method"], "POST");
        assert_eq!(json["response"]["status"], 200);
        assert_eq!(json["response"]["binary"], false);
        assert_eq!(json["response"]["truncated"], false);
        assert_eq!(json["timing"]["dns_ms"], 5.0);
        assert_eq!(json["timing"]["ttfb_ms"], 50.0);
        assert_eq!(json["redirect_chain"][0]["status"], 301);
        assert_eq!(json["type"], "xhr");
    }

    // =========================================================================
    // NetworkStreamEvent serialization
    // =========================================================================

    #[test]
    fn stream_event_serialization() {
        let event = NetworkStreamEvent {
            method: "GET".to_string(),
            url: "https://example.com/api".to_string(),
            status: Some(200),
            resource_type: "xhr".to_string(),
            size: Some(1234),
            duration_ms: Some(45.2),
            timestamp: "2026-02-14T12:00:00.000Z".to_string(),
            request_headers: None,
            response_headers: None,
        };
        let json: serde_json::Value = serde_json::to_value(&event).unwrap();
        assert_eq!(json["method"], "GET");
        assert_eq!(json["status"], 200);
        assert_eq!(json["type"], "xhr");
        // Headers should be absent (not null) when None
        assert!(json.get("request_headers").is_none());
        assert!(json.get("response_headers").is_none());
    }

    #[test]
    fn stream_event_verbose_serialization() {
        let event = NetworkStreamEvent {
            method: "GET".to_string(),
            url: "https://example.com/api".to_string(),
            status: Some(200),
            resource_type: "xhr".to_string(),
            size: Some(1234),
            duration_ms: Some(45.2),
            timestamp: "2026-02-14T12:00:00.000Z".to_string(),
            request_headers: Some(serde_json::json!({"Accept": "*/*"})),
            response_headers: Some(serde_json::json!({"Content-Type": "application/json"})),
        };
        let json: serde_json::Value = serde_json::to_value(&event).unwrap();
        assert_eq!(json["request_headers"]["Accept"], "*/*");
        assert_eq!(json["response_headers"]["Content-Type"], "application/json");
    }

    // =========================================================================
    // filter_by_type
    // =========================================================================

    fn make_request(
        id: usize,
        method: &str,
        url: &str,
        status: Option<u16>,
        resource_type: &str,
    ) -> NetworkRequestSummary {
        NetworkRequestSummary {
            id,
            method: method.to_string(),
            url: url.to_string(),
            status,
            resource_type: resource_type.to_string(),
            size: None,
            duration_ms: None,
            timestamp: String::new(),
        }
    }

    #[test]
    fn filter_by_type_single() {
        let requests = vec![
            make_request(0, "GET", "https://a.com", Some(200), "xhr"),
            make_request(1, "GET", "https://b.com", Some(200), "document"),
            make_request(2, "GET", "https://c.com", Some(200), "xhr"),
        ];
        let filtered = filter_by_type(requests, &["xhr".to_string()]);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|r| r.resource_type == "xhr"));
    }

    #[test]
    fn filter_by_type_multiple() {
        let requests = vec![
            make_request(0, "GET", "https://a.com", Some(200), "xhr"),
            make_request(1, "GET", "https://b.com", Some(200), "document"),
            make_request(2, "GET", "https://c.com", Some(200), "fetch"),
        ];
        let filtered = filter_by_type(requests, &["xhr".to_string(), "fetch".to_string()]);
        assert_eq!(filtered.len(), 2);
    }

    // =========================================================================
    // filter_by_url
    // =========================================================================

    #[test]
    fn filter_by_url_substring() {
        let requests = vec![
            make_request(0, "GET", "https://api.example.com/data", Some(200), "xhr"),
            make_request(
                1,
                "GET",
                "https://cdn.example.com/image.png",
                Some(200),
                "image",
            ),
        ];
        let filtered = filter_by_url(requests, "api.example.com");
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].url.contains("api.example.com"));
    }

    #[test]
    fn filter_by_url_no_match() {
        let requests = vec![make_request(
            0,
            "GET",
            "https://example.com/page",
            Some(200),
            "document",
        )];
        let filtered = filter_by_url(requests, "api.nowhere.com");
        assert!(filtered.is_empty());
    }

    // =========================================================================
    // filter_by_status
    // =========================================================================

    #[test]
    fn filter_by_status_exact() {
        let requests = vec![
            make_request(0, "GET", "https://a.com", Some(200), "document"),
            make_request(1, "GET", "https://b.com", Some(404), "document"),
            make_request(2, "GET", "https://c.com", Some(500), "document"),
        ];
        let filter = parse_status_filter("404");
        let filtered = filter_by_status(requests, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status, Some(404));
    }

    #[test]
    fn filter_by_status_wildcard() {
        let requests = vec![
            make_request(0, "GET", "https://a.com", Some(200), "document"),
            make_request(1, "GET", "https://b.com", Some(400), "document"),
            make_request(2, "GET", "https://c.com", Some(404), "document"),
            make_request(3, "GET", "https://d.com", Some(500), "document"),
        ];
        let filter = parse_status_filter("4xx");
        let filtered = filter_by_status(requests, &filter);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|r| {
            let s = r.status.unwrap();
            (400..500).contains(&s)
        }));
    }

    #[test]
    fn filter_by_status_none_skipped() {
        let requests = vec![
            make_request(0, "GET", "https://a.com", None, "document"),
            make_request(1, "GET", "https://b.com", Some(200), "document"),
        ];
        let filter = parse_status_filter("200");
        let filtered = filter_by_status(requests, &filter);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status, Some(200));
    }

    // =========================================================================
    // filter_by_method
    // =========================================================================

    #[test]
    fn filter_by_method_case_insensitive() {
        let requests = vec![
            make_request(0, "GET", "https://a.com", Some(200), "document"),
            make_request(1, "POST", "https://b.com", Some(200), "xhr"),
            make_request(2, "GET", "https://c.com", Some(200), "document"),
        ];
        let filtered = filter_by_method(requests, "post");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].method, "POST");
    }

    // =========================================================================
    // paginate
    // =========================================================================

    fn make_requests(count: usize) -> Vec<NetworkRequestSummary> {
        (0..count)
            .map(|i| {
                make_request(
                    i,
                    "GET",
                    &format!("https://example.com/{i}"),
                    Some(200),
                    "document",
                )
            })
            .collect()
    }

    #[test]
    fn paginate_page_0() {
        let requests = make_requests(30);
        let result = paginate(requests, 10, 0);
        assert_eq!(result.len(), 10);
        assert_eq!(result[0].id, 0);
        assert_eq!(result[9].id, 9);
    }

    #[test]
    fn paginate_page_1() {
        let requests = make_requests(30);
        let result = paginate(requests, 10, 1);
        assert_eq!(result.len(), 10);
        assert_eq!(result[0].id, 10);
        assert_eq!(result[9].id, 19);
    }

    #[test]
    fn paginate_beyond_available() {
        let requests = make_requests(5);
        let result = paginate(requests, 10, 1);
        assert!(result.is_empty());
    }

    #[test]
    fn paginate_partial_last_page() {
        let requests = make_requests(15);
        let result = paginate(requests, 10, 1);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].id, 10);
    }

    // =========================================================================
    // parse_status_filter
    // =========================================================================

    #[test]
    fn parse_status_filter_exact_value() {
        let filter = parse_status_filter("404");
        assert!(filter.matches(404));
        assert!(!filter.matches(200));
    }

    #[test]
    fn parse_status_filter_wildcard_4xx() {
        let filter = parse_status_filter("4xx");
        assert!(filter.matches(400));
        assert!(filter.matches(404));
        assert!(filter.matches(499));
        assert!(!filter.matches(500));
        assert!(!filter.matches(200));
    }

    #[test]
    fn parse_status_filter_wildcard_5xx() {
        let filter = parse_status_filter("5xx");
        assert!(filter.matches(500));
        assert!(filter.matches(503));
        assert!(!filter.matches(400));
    }

    #[test]
    fn parse_status_filter_wildcard_2xx() {
        let filter = parse_status_filter("2xx");
        assert!(filter.matches(200));
        assert!(filter.matches(201));
        assert!(filter.matches(299));
        assert!(!filter.matches(300));
    }

    // =========================================================================
    // timestamp_to_iso
    // =========================================================================

    #[test]
    fn timestamp_to_iso_epoch_zero() {
        assert_eq!(timestamp_to_iso(0.0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn timestamp_to_iso_known_value() {
        // 2024-02-14T12:00:00.000Z = 1707912000 seconds since epoch
        assert_eq!(
            timestamp_to_iso(1_707_912_000.0),
            "2024-02-14T12:00:00.000Z"
        );
    }

    #[test]
    fn timestamp_to_iso_with_milliseconds() {
        // 2024-02-14T12:00:00.123Z = 1707912000.123 seconds since epoch
        assert_eq!(
            timestamp_to_iso(1_707_912_000.123),
            "2024-02-14T12:00:00.123Z"
        );
    }

    // =========================================================================
    // is_binary_mime
    // =========================================================================

    #[test]
    fn binary_mime_detection() {
        assert!(is_binary_mime("image/png"));
        assert!(is_binary_mime("image/jpeg"));
        assert!(is_binary_mime("audio/mpeg"));
        assert!(is_binary_mime("video/mp4"));
        assert!(is_binary_mime("application/octet-stream"));
        assert!(is_binary_mime("application/pdf"));
        assert!(is_binary_mime("font/woff2"));
        assert!(is_binary_mime("application/wasm"));
        assert!(!is_binary_mime("text/html"));
        assert!(!is_binary_mime("application/json"));
        assert!(!is_binary_mime("text/css"));
    }

    // =========================================================================
    // extract_timing
    // =========================================================================

    #[test]
    fn extract_timing_full() {
        let timing = serde_json::json!({
            "dnsStart": 0.0,
            "dnsEnd": 5.0,
            "connectStart": 5.0,
            "connectEnd": 15.0,
            "sslStart": 10.0,
            "sslEnd": 15.0,
            "sendEnd": 16.0,
            "receiveHeadersEnd": 66.0
        });
        let ti = extract_timing(&timing);
        assert!((ti.dns_ms - 5.0).abs() < f64::EPSILON);
        assert!((ti.connect_ms - 10.0).abs() < f64::EPSILON);
        assert!((ti.tls_ms - 5.0).abs() < f64::EPSILON);
        assert!((ti.ttfb_ms - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_timing_missing_fields() {
        let timing = serde_json::json!({});
        let ti = extract_timing(&timing);
        assert!((ti.dns_ms).abs() < f64::EPSILON);
        assert!((ti.connect_ms).abs() < f64::EPSILON);
        assert!((ti.tls_ms).abs() < f64::EPSILON);
        assert!((ti.ttfb_ms).abs() < f64::EPSILON);
    }

    // =========================================================================
    // body truncation logic
    // =========================================================================

    #[test]
    fn body_under_limit_not_truncated() {
        let body = "a".repeat(100);
        assert!(body.len() <= MAX_INLINE_BODY_SIZE);
    }

    #[test]
    fn body_over_limit_truncated() {
        let body = "a".repeat(MAX_INLINE_BODY_SIZE + 1000);
        let truncated = &body[..MAX_INLINE_BODY_SIZE];
        assert_eq!(truncated.len(), MAX_INLINE_BODY_SIZE);
    }

    // =========================================================================
    // resolve_type_filter
    // =========================================================================

    #[test]
    fn resolve_type_filter_none() {
        assert!(resolve_type_filter(None).is_none());
    }

    #[test]
    fn resolve_type_filter_single() {
        let result = resolve_type_filter(Some("xhr"));
        let types = result.unwrap();
        assert_eq!(types, vec!["xhr"]);
    }

    #[test]
    fn resolve_type_filter_multiple() {
        let result = resolve_type_filter(Some("xhr,fetch,document"));
        let types = result.unwrap();
        assert_eq!(types.len(), 3);
        assert!(types.contains(&"xhr".to_string()));
        assert!(types.contains(&"fetch".to_string()));
        assert!(types.contains(&"document".to_string()));
    }

    // =========================================================================
    // Plain text output (no panics)
    // =========================================================================

    #[test]
    fn plain_text_list_empty() {
        print_list_plain(&[]);
    }

    #[test]
    fn plain_text_list_requests() {
        let requests = vec![
            make_request(0, "GET", "https://example.com", Some(200), "document"),
            make_request(1, "POST", "https://api.example.com", Some(404), "xhr"),
        ];
        print_list_plain(&requests);
    }

    #[test]
    fn plain_text_detail() {
        let detail = NetworkRequestDetail {
            id: 0,
            request: RequestInfo {
                method: "GET".to_string(),
                url: "https://example.com".to_string(),
                headers: serde_json::json!({}),
                body: None,
            },
            response: ResponseInfo {
                status: Some(200),
                status_text: "OK".to_string(),
                headers: serde_json::json!({}),
                body: Some("hello".to_string()),
                binary: false,
                truncated: false,
                mime_type: Some("text/html".to_string()),
            },
            timing: TimingInfo {
                dns_ms: 1.0,
                connect_ms: 2.0,
                tls_ms: 3.0,
                ttfb_ms: 4.0,
                download_ms: 5.0,
            },
            redirect_chain: vec![],
            resource_type: "document".to_string(),
            size: Some(5),
            duration_ms: Some(15.0),
            timestamp: "2026-02-14T12:00:00.000Z".to_string(),
        };
        print_detail_plain(&detail);
    }

    // =========================================================================
    // RedirectEntry serialization
    // =========================================================================

    #[test]
    fn redirect_entry_serialization() {
        let entry = RedirectEntry {
            url: "http://example.com".to_string(),
            status: 301,
        };
        let json: serde_json::Value = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["url"], "http://example.com");
        assert_eq!(json["status"], 301);
    }
}
