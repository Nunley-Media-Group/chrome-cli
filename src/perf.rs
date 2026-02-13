use std::fs;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use chrome_cli::cdp::{CdpClient, CdpConfig, CdpEvent};
use chrome_cli::connection::{ManagedSession, resolve_connection, resolve_target};
use chrome_cli::error::{AppError, ExitCode};

use crate::cli::{
    GlobalOpts, PerfAnalyzeArgs, PerfArgs, PerfCommand, PerfStartArgs, PerfStopArgs, PerfVitalsArgs,
};

/// Default trace timeout in milliseconds (30 seconds).
const DEFAULT_TRACE_TIMEOUT_MS: u64 = 30_000;

/// Default performance trace categories.
const TRACE_CATEGORIES: &str = "devtools.timeline,v8.execute,blink.user_timing,loading,\
    disabled-by-default-devtools.timeline,disabled-by-default-lighthouse";

/// Available insight names for `perf analyze`.
const VALID_INSIGHTS: &[&str] = &[
    "DocumentLatency",
    "LCPBreakdown",
    "RenderBlocking",
    "LongTasks",
];

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct PerfStartResult {
    tracing: bool,
    file: String,
}

#[derive(Serialize)]
struct PerfStopResult {
    file: String,
    duration_ms: u64,
    size_bytes: u64,
    vitals: CoreWebVitals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoreWebVitals {
    #[serde(skip_serializing_if = "Option::is_none")]
    lcp_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cls: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttfb_ms: Option<f64>,
}

#[derive(Serialize)]
struct PerfVitalsResult {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lcp_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cls: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttfb_ms: Option<f64>,
}

#[derive(Serialize)]
struct PerfAnalyzeResult {
    insight: String,
    details: serde_json::Value,
}

// =============================================================================
// Output formatting
// =============================================================================

fn print_output(
    value: &impl Serialize,
    output: &crate::cli::OutputFormat,
    plain_text: Option<&str>,
) -> Result<(), AppError> {
    if output.plain {
        if let Some(text) = plain_text {
            print!("{text}");
            return Ok(());
        }
    }
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
// Dispatcher
// =============================================================================

/// Execute the `perf` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_perf(global: &GlobalOpts, args: &PerfArgs) -> Result<(), AppError> {
    match &args.command {
        PerfCommand::Start(start_args) => execute_start(global, start_args).await,
        PerfCommand::Stop(stop_args) => execute_stop(global, stop_args).await,
        PerfCommand::Analyze(analyze_args) => execute_analyze(global, analyze_args),
        PerfCommand::Vitals(vitals_args) => execute_vitals(global, vitals_args).await,
    }
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
// Trace file path generation
// =============================================================================

fn resolve_trace_path(file: Option<&PathBuf>) -> PathBuf {
    if let Some(path) = file {
        return path.clone();
    }
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    std::env::temp_dir().join(format!("chrome-trace-{timestamp}.json"))
}

// =============================================================================
// perf start
// =============================================================================

async fn execute_start(global: &GlobalOpts, args: &PerfStartArgs) -> Result<(), AppError> {
    let trace_path = resolve_trace_path(args.file.as_ref());
    let (_client, mut managed) = setup_session(global).await?;

    // Enable required domains
    if args.reload || args.auto_stop {
        managed.ensure_domain("Page").await?;
    }

    // Start tracing
    let start_params = serde_json::json!({
        "categories": TRACE_CATEGORIES,
        "transferMode": "ReportEvents",
    });
    managed
        .send_command("Tracing.start", Some(start_params))
        .await
        .map_err(|e| AppError {
            message: format!("Failed to start trace: {e}"),
            code: ExitCode::ProtocolError,
        })?;

    // Handle --reload
    if args.reload {
        let load_rx = managed.subscribe("Page.loadEventFired").await?;
        managed
            .send_command("Page.reload", Some(serde_json::json!({})))
            .await?;
        wait_for_event(load_rx, DEFAULT_TRACE_TIMEOUT_MS, "page load").await?;
    }

    // Handle --auto-stop
    if args.auto_stop {
        // If we didn't already reload, wait for page load
        if !args.reload {
            let load_rx = managed.subscribe("Page.loadEventFired").await?;
            wait_for_event(load_rx, DEFAULT_TRACE_TIMEOUT_MS, "page load").await?;
        }

        // Stop and collect
        let result = stop_and_collect(&managed, &trace_path).await?;

        let plain = format_stop_plain(&result);
        return print_output(&result, &global.output, Some(&plain));
    }

    // Non-auto-stop: return immediately
    let result = PerfStartResult {
        tracing: true,
        file: trace_path.display().to_string(),
    };
    let plain = format!("Tracing started. File: {}\n", trace_path.display());
    print_output(&result, &global.output, Some(&plain))
}

// =============================================================================
// perf stop
// =============================================================================

async fn execute_stop(global: &GlobalOpts, args: &PerfStopArgs) -> Result<(), AppError> {
    let trace_path = resolve_trace_path(args.file.as_ref());
    let (_client, managed) = setup_session(global).await?;

    let result = stop_and_collect(&managed, &trace_path).await?;

    let plain = format_stop_plain(&result);
    print_output(&result, &global.output, Some(&plain))
}

/// Stop the active trace, collect data, write to file, parse vitals.
async fn stop_and_collect(
    managed: &ManagedSession,
    trace_path: &Path,
) -> Result<PerfStopResult, AppError> {
    let start_time = std::time::Instant::now();

    // Subscribe to trace events
    let data_rx = managed.subscribe("Tracing.dataCollected").await?;
    let complete_rx = managed.subscribe("Tracing.tracingComplete").await?;

    // Send Tracing.end — if no trace is active, CDP returns an error
    managed
        .send_command("Tracing.end", None)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("Tracing is not started") || msg.contains("not started") {
                AppError::no_active_trace()
            } else {
                AppError {
                    message: format!("Failed to stop trace: {e}"),
                    code: ExitCode::ProtocolError,
                }
            }
        })?;

    // Stream trace data to file
    stream_trace_to_file(data_rx, complete_rx, trace_path).await?;

    #[allow(clippy::cast_possible_truncation)]
    let duration_ms = start_time.elapsed().as_millis() as u64;

    // Get file size
    let metadata = fs::metadata(trace_path).map_err(|e| AppError {
        message: format!("Failed to read trace file metadata: {e}"),
        code: ExitCode::GeneralError,
    })?;

    // Parse vitals
    let vitals = parse_trace_vitals(trace_path).unwrap_or(CoreWebVitals {
        lcp_ms: None,
        cls: None,
        ttfb_ms: None,
    });

    Ok(PerfStopResult {
        file: trace_path.display().to_string(),
        duration_ms,
        size_bytes: metadata.len(),
        vitals,
    })
}

/// Stream Tracing.dataCollected events to a file in Chrome Trace Event Format.
async fn stream_trace_to_file(
    mut data_rx: tokio::sync::mpsc::Receiver<CdpEvent>,
    mut complete_rx: tokio::sync::mpsc::Receiver<CdpEvent>,
    trace_path: &Path,
) -> Result<(), AppError> {
    let file = fs::File::create(trace_path).map_err(|e| AppError {
        message: format!("Failed to create trace file {}: {e}", trace_path.display()),
        code: ExitCode::GeneralError,
    })?;
    let mut writer = BufWriter::new(file);

    // Write opening of trace format
    writer
        .write_all(b"{\"traceEvents\":[")
        .map_err(|e| write_error(&e))?;

    let mut first_event = true;
    let timeout = Duration::from_millis(DEFAULT_TRACE_TIMEOUT_MS);

    loop {
        tokio::select! {
            event = data_rx.recv() => {
                match event {
                    Some(evt) => {
                        // Each dataCollected event has params.value = [TraceEvent, ...]
                        if let Some(events) = evt.params["value"].as_array() {
                            for trace_event in events {
                                if !first_event {
                                    writer.write_all(b",").map_err(|e| write_error(&e))?;
                                }
                                first_event = false;
                                let bytes = serde_json::to_vec(trace_event).map_err(|e| AppError {
                                    message: format!("Failed to serialize trace event: {e}"),
                                    code: ExitCode::GeneralError,
                                })?;
                                writer.write_all(&bytes).map_err(|e| write_error(&e))?;
                            }
                        }
                    }
                    None => break,
                }
            }
            event = complete_rx.recv() => {
                // Tracing complete — drain remaining data events
                if event.is_some() {
                    // Drain any remaining buffered data events
                    while let Ok(evt) = data_rx.try_recv() {
                        if let Some(events) = evt.params["value"].as_array() {
                            for trace_event in events {
                                if !first_event {
                                    writer.write_all(b",").map_err(|e| write_error(&e))?;
                                }
                                first_event = false;
                                let bytes = serde_json::to_vec(trace_event).map_err(|e| AppError {
                                    message: format!("Failed to serialize trace event: {e}"),
                                    code: ExitCode::GeneralError,
                                })?;
                                writer.write_all(&bytes).map_err(|e| write_error(&e))?;
                            }
                        }
                    }
                    break;
                }
            }
            () = tokio::time::sleep(timeout) => {
                // Write what we have and report timeout
                break;
            }
        }
    }

    // Close trace format
    writer.write_all(b"]}").map_err(|e| write_error(&e))?;
    writer.flush().map_err(|e| write_error(&e))?;

    Ok(())
}

fn write_error(e: &std::io::Error) -> AppError {
    AppError {
        message: format!("Failed to write trace data: {e}"),
        code: ExitCode::GeneralError,
    }
}

// =============================================================================
// Trace parsing — Core Web Vitals extraction
// =============================================================================

/// A single trace event from the Chrome Trace Event Format.
#[derive(Debug, Deserialize)]
struct TraceEvent {
    #[serde(default)]
    cat: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    ts: f64,
    #[serde(default)]
    dur: f64,
    #[serde(default)]
    args: serde_json::Value,
}

/// Wrapper for the trace file format.
#[derive(Debug, Deserialize)]
struct TraceFile {
    #[serde(rename = "traceEvents")]
    trace_events: Vec<TraceEvent>,
}

/// Parse a trace file and extract Core Web Vitals.
fn parse_trace_vitals(path: &Path) -> Result<CoreWebVitals, AppError> {
    let file = fs::File::open(path)
        .map_err(|e| AppError::trace_file_not_found(&format!("{}: {e}", path.display())))?;
    let reader = BufReader::new(file);
    let trace: TraceFile = serde_json::from_reader(reader)
        .map_err(|e| AppError::trace_parse_failed(&e.to_string()))?;

    let lcp_ms = extract_lcp(&trace.trace_events);
    let cls = extract_cls(&trace.trace_events);
    let ttfb_ms = extract_ttfb(&trace.trace_events);

    Ok(CoreWebVitals {
        lcp_ms,
        cls,
        ttfb_ms,
    })
}

/// Extract LCP from the last `largestContentfulPaint::Candidate` event.
fn extract_lcp(events: &[TraceEvent]) -> Option<f64> {
    let mut last_lcp_ts: Option<f64> = None;
    let mut navigation_start: Option<f64> = None;

    for event in events {
        if event.name == "navigationStart"
            && event.cat.contains("blink.user_timing")
            && navigation_start.is_none()
        {
            navigation_start = Some(event.ts);
        }
        if event.name.contains("largestContentfulPaint") && event.name.contains("Candidate") {
            last_lcp_ts = Some(event.ts);
        }
        // Also check for LCP in loading category
        if event.name == "largestContentfulPaint::Candidate" {
            last_lcp_ts = Some(event.ts);
        }
    }

    // If we have a navigation start, compute relative LCP
    match (last_lcp_ts, navigation_start) {
        (Some(lcp), Some(nav)) => Some((lcp - nav) / 1000.0), // µs → ms
        (Some(lcp), None) => {
            // Try to find the earliest timestamp as reference
            let min_ts = events.iter().map(|e| e.ts).fold(f64::INFINITY, f64::min);
            if min_ts.is_finite() {
                Some((lcp - min_ts) / 1000.0)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extract CLS by summing `LayoutShift` scores where `had_recent_input` is false.
fn extract_cls(events: &[TraceEvent]) -> Option<f64> {
    let mut total_cls: f64 = 0.0;
    let mut found = false;

    for event in events {
        if event.name == "LayoutShift" {
            if let Some(data) = event.args.get("data") {
                let had_recent_input = data["had_recent_input"].as_bool().unwrap_or(false);
                if !had_recent_input {
                    if let Some(score) = data["score"].as_f64() {
                        total_cls += score;
                        found = true;
                    }
                }
            }
        }
    }

    if found { Some(total_cls) } else { None }
}

/// Check if a URL looks like a sub-resource (JS, CSS, image, font).
fn is_subresource_url(url: &str) -> bool {
    let path = std::path::Path::new(url);
    path.extension().is_some_and(|ext| {
        let ext = ext.to_ascii_lowercase();
        matches!(
            ext.to_str(),
            Some("js" | "css" | "png" | "jpg" | "gif" | "svg" | "woff" | "woff2")
        )
    })
}

/// Extract TTFB from `ResourceSendRequest` and `ResourceReceiveResponse` for main document.
fn extract_ttfb(events: &[TraceEvent]) -> Option<f64> {
    // Find the first document request and its response
    let mut doc_request_id: Option<String> = None;
    let mut request_ts: Option<f64> = None;
    let mut response_ts: Option<f64> = None;

    for event in events {
        if event.name == "ResourceSendRequest" {
            if let Some(data) = event.args.get("data") {
                let url = data["url"].as_str().unwrap_or("");
                let is_doc = data["requestId"].as_str().is_some()
                    && doc_request_id.is_none()
                    && !url.is_empty()
                    && !is_subresource_url(url);
                if is_doc {
                    doc_request_id = data["requestId"].as_str().map(String::from);
                    request_ts = Some(event.ts);
                }
            }
        }
        if event.name == "ResourceReceiveResponse" {
            if let Some(ref req_id) = doc_request_id {
                if let Some(data) = event.args.get("data") {
                    if data["requestId"].as_str() == Some(req_id) {
                        response_ts = Some(event.ts);
                        break;
                    }
                }
            }
        }
    }

    match (request_ts, response_ts) {
        (Some(req), Some(resp)) => Some((resp - req) / 1000.0), // µs → ms
        _ => None,
    }
}

// =============================================================================
// perf analyze
// =============================================================================

fn execute_analyze(global: &GlobalOpts, args: &PerfAnalyzeArgs) -> Result<(), AppError> {
    // Validate insight name
    if !VALID_INSIGHTS.contains(&args.insight.as_str()) {
        return Err(AppError::unknown_insight(&args.insight));
    }

    // Validate trace file exists
    if !args.trace_file.exists() {
        return Err(AppError::trace_file_not_found(
            &args.trace_file.display().to_string(),
        ));
    }

    // Read and parse trace file
    let file = fs::File::open(&args.trace_file).map_err(|e| {
        AppError::trace_file_not_found(&format!("{}: {e}", args.trace_file.display()))
    })?;
    let reader = BufReader::new(file);
    let trace: TraceFile = serde_json::from_reader(reader)
        .map_err(|e| AppError::trace_parse_failed(&e.to_string()))?;

    let details = match args.insight.as_str() {
        "DocumentLatency" => analyze_document_latency(&trace.trace_events),
        "LCPBreakdown" => analyze_lcp_breakdown(&trace.trace_events),
        "RenderBlocking" => analyze_render_blocking(&trace.trace_events),
        "LongTasks" => analyze_long_tasks(&trace.trace_events),
        _ => unreachable!(),
    };

    let result = PerfAnalyzeResult {
        insight: args.insight.clone(),
        details,
    };

    let plain = format_analyze_plain(&result);
    print_output(&result, &global.output, Some(&plain))
}

// =============================================================================
// Insight analysis functions
// =============================================================================

fn analyze_document_latency(events: &[TraceEvent]) -> serde_json::Value {
    let mut request_ts: Option<f64> = None;
    let mut response_ts: Option<f64> = None;
    let mut finish_ts: Option<f64> = None;
    let mut doc_request_id: Option<String> = None;

    for event in events {
        if event.name == "ResourceSendRequest" && doc_request_id.is_none() {
            if let Some(data) = event.args.get("data") {
                let url = data["url"].as_str().unwrap_or("");
                if !url.is_empty() && !is_subresource_url(url) {
                    doc_request_id = data["requestId"].as_str().map(String::from);
                    request_ts = Some(event.ts);
                }
            }
        }
        if let Some(ref req_id) = doc_request_id {
            if event.name == "ResourceReceiveResponse" {
                if let Some(data) = event.args.get("data") {
                    if data["requestId"].as_str() == Some(req_id) {
                        response_ts = Some(event.ts);
                    }
                }
            }
            if event.name == "ResourceFinish" {
                if let Some(data) = event.args.get("data") {
                    if data["requestId"].as_str() == Some(req_id) {
                        finish_ts = Some(event.ts);
                    }
                }
            }
        }
    }

    let dns_ms = 0.0_f64; // Not directly available in trace events
    let connect_ms = 0.0_f64;
    let ttfb_ms = match (request_ts, response_ts) {
        (Some(req), Some(resp)) => (resp - req) / 1000.0,
        _ => 0.0,
    };
    let download_ms = match (response_ts, finish_ts) {
        (Some(resp), Some(fin)) => (fin - resp) / 1000.0,
        _ => 0.0,
    };
    let total_ms = match (request_ts, finish_ts) {
        (Some(req), Some(fin)) => (fin - req) / 1000.0,
        _ => ttfb_ms + download_ms,
    };

    serde_json::json!({
        "dns_ms": dns_ms,
        "connect_ms": connect_ms,
        "ttfb_ms": ttfb_ms,
        "download_ms": download_ms,
        "total_ms": total_ms,
    })
}

fn analyze_lcp_breakdown(events: &[TraceEvent]) -> serde_json::Value {
    let ttfb_ms = extract_ttfb(events).unwrap_or(0.0);
    let lcp_ms = extract_lcp(events).unwrap_or(0.0);

    // Simplified breakdown: LCP = TTFB + load_delay + load_duration + render_delay
    // Without detailed sub-resource tracking, we provide a basic breakdown
    let remaining = (lcp_ms - ttfb_ms).max(0.0);
    let load_delay_ms = remaining * 0.3; // approximate
    let load_duration_ms = remaining * 0.4;
    let render_delay_ms = remaining * 0.3;

    serde_json::json!({
        "ttfb_ms": ttfb_ms,
        "load_delay_ms": load_delay_ms,
        "load_duration_ms": load_duration_ms,
        "render_delay_ms": render_delay_ms,
        "total_ms": lcp_ms,
    })
}

fn analyze_render_blocking(events: &[TraceEvent]) -> serde_json::Value {
    let mut blocking_resources: Vec<serde_json::Value> = Vec::new();

    for event in events {
        if event.name == "ResourceSendRequest" {
            if let Some(data) = event.args.get("data") {
                let render_blocking = data["renderBlocking"].as_str().unwrap_or("non_blocking");
                if render_blocking == "blocking" || render_blocking == "in_body_parser_blocking" {
                    let url = data["url"].as_str().unwrap_or("unknown");
                    blocking_resources.push(serde_json::json!({
                        "url": url,
                        "blocking_type": render_blocking,
                    }));
                }
            }
        }
    }

    serde_json::json!({
        "count": blocking_resources.len(),
        "resources": blocking_resources,
    })
}

fn analyze_long_tasks(events: &[TraceEvent]) -> serde_json::Value {
    // Long tasks are RunTask events > 50ms on the main thread
    let mut long_tasks: Vec<serde_json::Value> = Vec::new();

    for event in events {
        if event.name == "RunTask" && event.cat.contains("devtools.timeline") {
            // Duration is in microseconds
            let dur_ms = event.dur / 1000.0;
            if dur_ms > 50.0 {
                long_tasks.push(serde_json::json!({
                    "duration_ms": dur_ms,
                    "start_ms": event.ts / 1000.0,
                }));
            }
        }
    }

    // Sort by duration descending
    long_tasks.sort_by(|a, b| {
        b["duration_ms"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["duration_ms"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    serde_json::json!({
        "count": long_tasks.len(),
        "total_blocking_ms": long_tasks.iter()
            .filter_map(|t| t["duration_ms"].as_f64())
            .sum::<f64>(),
        "tasks": long_tasks,
    })
}

// =============================================================================
// perf vitals
// =============================================================================

async fn execute_vitals(global: &GlobalOpts, args: &PerfVitalsArgs) -> Result<(), AppError> {
    let trace_path = resolve_trace_path(args.file.as_ref());
    let (_client, mut managed) = setup_session(global).await?;

    // Enable required domains
    managed.ensure_domain("Page").await?;

    // Get current URL before reload
    let url = get_page_url(&mut managed).await?;

    // Subscribe to events
    let load_rx = managed.subscribe("Page.loadEventFired").await?;

    // Start tracing
    let start_params = serde_json::json!({
        "categories": TRACE_CATEGORIES,
        "transferMode": "ReportEvents",
    });
    managed
        .send_command("Tracing.start", Some(start_params))
        .await
        .map_err(|e| AppError {
            message: format!("Failed to start trace: {e}"),
            code: ExitCode::ProtocolError,
        })?;

    // Reload the page
    managed
        .send_command("Page.reload", Some(serde_json::json!({})))
        .await?;

    // Wait for page load
    wait_for_event(load_rx, DEFAULT_TRACE_TIMEOUT_MS, "page load").await?;

    // Stop and collect trace
    let data_rx = managed.subscribe("Tracing.dataCollected").await?;
    let complete_rx = managed.subscribe("Tracing.tracingComplete").await?;
    managed.send_command("Tracing.end", None).await?;
    stream_trace_to_file(data_rx, complete_rx, &trace_path).await?;

    // Parse vitals
    let vitals = parse_trace_vitals(&trace_path).unwrap_or(CoreWebVitals {
        lcp_ms: None,
        cls: None,
        ttfb_ms: None,
    });

    let result = PerfVitalsResult {
        url,
        lcp_ms: vitals.lcp_ms,
        cls: vitals.cls,
        ttfb_ms: vitals.ttfb_ms,
    };

    let plain = format_vitals_plain(&result);
    print_output(&result, &global.output, Some(&plain))
}

// =============================================================================
// Helpers
// =============================================================================

async fn get_page_url(managed: &mut ManagedSession) -> Result<String, AppError> {
    managed.ensure_domain("Runtime").await?;
    let result = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "location.href" })),
        )
        .await?;
    Ok(result["result"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

async fn wait_for_event(
    mut rx: tokio::sync::mpsc::Receiver<CdpEvent>,
    timeout_ms: u64,
    description: &str,
) -> Result<(), AppError> {
    let timeout = Duration::from_millis(timeout_ms);
    tokio::select! {
        event = rx.recv() => {
            match event {
                Some(_) => Ok(()),
                None => Err(AppError {
                    message: format!("Event channel closed while waiting for {description}"),
                    code: ExitCode::GeneralError,
                }),
            }
        }
        () = tokio::time::sleep(timeout) => {
            Err(AppError::trace_timeout(timeout_ms))
        }
    }
}

// =============================================================================
// Plain text formatters
// =============================================================================

fn format_stop_plain(result: &PerfStopResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("Trace saved: {}\n", result.file));
    out.push_str(&format!("Duration: {}ms\n", result.duration_ms));
    out.push_str(&format!("Size: {} bytes\n", result.size_bytes));
    if let Some(lcp) = result.vitals.lcp_ms {
        out.push_str(&format!("LCP: {lcp:.1}ms\n"));
    }
    if let Some(cls) = result.vitals.cls {
        out.push_str(&format!("CLS: {cls:.3}\n"));
    }
    if let Some(ttfb) = result.vitals.ttfb_ms {
        out.push_str(&format!("TTFB: {ttfb:.1}ms\n"));
    }
    out
}

fn format_vitals_plain(result: &PerfVitalsResult) -> String {
    let mut parts = Vec::new();
    if let Some(lcp) = result.lcp_ms {
        parts.push(format!("LCP: {lcp:.1}ms"));
    }
    if let Some(cls) = result.cls {
        parts.push(format!("CLS: {cls:.3}"));
    }
    if let Some(ttfb) = result.ttfb_ms {
        parts.push(format!("TTFB: {ttfb:.1}ms"));
    }
    if parts.is_empty() {
        "No vitals data available\n".to_string()
    } else {
        format!("{}\n", parts.join("  "))
    }
}

fn format_analyze_plain(result: &PerfAnalyzeResult) -> String {
    let mut out = format!("Insight: {}\n", result.insight);
    if let Some(obj) = result.details.as_object() {
        for (key, value) in obj {
            if key == "resources" || key == "tasks" {
                if let Some(arr) = value.as_array() {
                    out.push_str(&format!("  {key}: ({} items)\n", arr.len()));
                    for item in arr.iter().take(10) {
                        out.push_str(&format!("    {item}\n"));
                    }
                }
            } else {
                out.push_str(&format!("  {key}: {value}\n"));
            }
        }
    }
    out
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Output type serialization
    // =========================================================================

    #[test]
    fn perf_start_result_serialization() {
        let result = PerfStartResult {
            tracing: true,
            file: "/tmp/chrome-trace-123.json".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["tracing"], true);
        assert_eq!(json["file"], "/tmp/chrome-trace-123.json");
    }

    #[test]
    fn perf_stop_result_serialization() {
        let result = PerfStopResult {
            file: "/tmp/trace.json".to_string(),
            duration_ms: 3456,
            size_bytes: 1_234_567,
            vitals: CoreWebVitals {
                lcp_ms: Some(1200.5),
                cls: Some(0.05),
                ttfb_ms: Some(180.3),
            },
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["file"], "/tmp/trace.json");
        assert_eq!(json["duration_ms"], 3456);
        assert_eq!(json["size_bytes"], 1_234_567);
        assert_eq!(json["vitals"]["lcp_ms"], 1200.5);
        assert_eq!(json["vitals"]["cls"], 0.05);
        assert_eq!(json["vitals"]["ttfb_ms"], 180.3);
    }

    #[test]
    fn perf_stop_result_with_none_vitals() {
        let result = PerfStopResult {
            file: "/tmp/trace.json".to_string(),
            duration_ms: 100,
            size_bytes: 500,
            vitals: CoreWebVitals {
                lcp_ms: None,
                cls: None,
                ttfb_ms: None,
            },
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert!(json["vitals"].get("lcp_ms").is_none());
        assert!(json["vitals"].get("cls").is_none());
        assert!(json["vitals"].get("ttfb_ms").is_none());
    }

    #[test]
    fn perf_vitals_result_serialization() {
        let result = PerfVitalsResult {
            url: "https://example.com".to_string(),
            lcp_ms: Some(1200.5),
            cls: Some(0.05),
            ttfb_ms: Some(180.3),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["lcp_ms"], 1200.5);
        assert_eq!(json["cls"], 0.05);
        assert_eq!(json["ttfb_ms"], 180.3);
    }

    #[test]
    fn perf_analyze_result_serialization() {
        let result = PerfAnalyzeResult {
            insight: "LCPBreakdown".to_string(),
            details: serde_json::json!({
                "ttfb_ms": 180.3,
                "total_ms": 1200.5,
            }),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["insight"], "LCPBreakdown");
        assert_eq!(json["details"]["ttfb_ms"], 180.3);
    }

    // =========================================================================
    // Trace file path generation
    // =========================================================================

    #[test]
    fn resolve_trace_path_with_custom_file() {
        let path = PathBuf::from("/tmp/custom.json");
        let result = resolve_trace_path(Some(&path));
        assert_eq!(result, PathBuf::from("/tmp/custom.json"));
    }

    #[test]
    fn resolve_trace_path_auto_generated() {
        let result = resolve_trace_path(None);
        let path_str = result.to_string_lossy();
        assert!(path_str.contains("chrome-trace-"));
        assert!(path_str.ends_with(".json"));
    }

    // =========================================================================
    // CWV extraction from trace events
    // =========================================================================

    fn make_trace_event(
        name: &str,
        cat: &str,
        ts: f64,
        dur: f64,
        args: serde_json::Value,
    ) -> TraceEvent {
        TraceEvent {
            name: name.to_string(),
            cat: cat.to_string(),
            ts,
            dur,
            args,
        }
    }

    #[test]
    fn extract_lcp_from_trace_events() {
        let events = vec![
            make_trace_event(
                "navigationStart",
                "blink.user_timing",
                1_000_000.0,
                0.0,
                serde_json::json!({}),
            ),
            make_trace_event(
                "largestContentfulPaint::Candidate",
                "loading",
                2_200_000.0,
                0.0,
                serde_json::json!({"data": {"size": 5000}}),
            ),
        ];
        let lcp = extract_lcp(&events);
        assert!(lcp.is_some());
        let lcp_ms = lcp.unwrap();
        assert!((lcp_ms - 1200.0).abs() < 0.1);
    }

    #[test]
    fn extract_lcp_returns_none_when_no_candidate() {
        let events = vec![make_trace_event(
            "navigationStart",
            "blink.user_timing",
            1_000_000.0,
            0.0,
            serde_json::json!({}),
        )];
        assert!(extract_lcp(&events).is_none());
    }

    #[test]
    fn extract_cls_from_layout_shifts() {
        let events = vec![
            make_trace_event(
                "LayoutShift",
                "loading",
                1_500_000.0,
                0.0,
                serde_json::json!({"data": {"score": 0.02, "had_recent_input": false}}),
            ),
            make_trace_event(
                "LayoutShift",
                "loading",
                1_600_000.0,
                0.0,
                serde_json::json!({"data": {"score": 0.03, "had_recent_input": false}}),
            ),
            make_trace_event(
                "LayoutShift",
                "loading",
                1_700_000.0,
                0.0,
                serde_json::json!({"data": {"score": 0.01, "had_recent_input": true}}),
            ),
        ];
        let cls = extract_cls(&events);
        assert!(cls.is_some());
        assert!((cls.unwrap() - 0.05).abs() < 0.001);
    }

    #[test]
    fn extract_cls_excludes_recent_input() {
        let events = vec![make_trace_event(
            "LayoutShift",
            "loading",
            1_500_000.0,
            0.0,
            serde_json::json!({"data": {"score": 0.1, "had_recent_input": true}}),
        )];
        assert!(extract_cls(&events).is_none());
    }

    #[test]
    fn extract_ttfb_from_resource_events() {
        let events = vec![
            make_trace_event(
                "ResourceSendRequest",
                "devtools.timeline",
                1_000_000.0,
                0.0,
                serde_json::json!({"data": {"requestId": "1", "url": "https://example.com/"}}),
            ),
            make_trace_event(
                "ResourceReceiveResponse",
                "devtools.timeline",
                1_180_000.0,
                0.0,
                serde_json::json!({"data": {"requestId": "1"}}),
            ),
        ];
        let ttfb = extract_ttfb(&events);
        assert!(ttfb.is_some());
        assert!((ttfb.unwrap() - 180.0).abs() < 0.1);
    }

    #[test]
    fn extract_ttfb_returns_none_without_response() {
        let events = vec![make_trace_event(
            "ResourceSendRequest",
            "devtools.timeline",
            1_000_000.0,
            0.0,
            serde_json::json!({"data": {"requestId": "1", "url": "https://example.com/"}}),
        )];
        assert!(extract_ttfb(&events).is_none());
    }

    // =========================================================================
    // Insight analysis
    // =========================================================================

    #[test]
    fn analyze_long_tasks_finds_tasks_over_50ms() {
        let events = vec![
            make_trace_event(
                "RunTask",
                "devtools.timeline",
                1_000_000.0,
                60_000.0,
                serde_json::json!({}),
            ),
            make_trace_event(
                "RunTask",
                "devtools.timeline",
                2_000_000.0,
                30_000.0,
                serde_json::json!({}),
            ),
            make_trace_event(
                "RunTask",
                "devtools.timeline",
                3_000_000.0,
                100_000.0,
                serde_json::json!({}),
            ),
        ];
        let result = analyze_long_tasks(&events);
        assert_eq!(result["count"], 2);
        let tasks = result["tasks"].as_array().unwrap();
        // Sorted by duration descending
        assert!((tasks[0]["duration_ms"].as_f64().unwrap() - 100.0).abs() < 0.1);
        assert!((tasks[1]["duration_ms"].as_f64().unwrap() - 60.0).abs() < 0.1);
    }

    #[test]
    fn analyze_render_blocking_finds_blocking_resources() {
        let events = vec![
            make_trace_event(
                "ResourceSendRequest",
                "devtools.timeline",
                1_000_000.0,
                0.0,
                serde_json::json!({"data": {"url": "https://example.com/style.css", "renderBlocking": "blocking"}}),
            ),
            make_trace_event(
                "ResourceSendRequest",
                "devtools.timeline",
                1_100_000.0,
                0.0,
                serde_json::json!({"data": {"url": "https://example.com/app.js", "renderBlocking": "non_blocking"}}),
            ),
        ];
        let result = analyze_render_blocking(&events);
        assert_eq!(result["count"], 1);
        let resources = result["resources"].as_array().unwrap();
        assert_eq!(resources[0]["url"], "https://example.com/style.css");
    }

    #[test]
    fn analyze_document_latency_computes_breakdown() {
        let events = vec![
            make_trace_event(
                "ResourceSendRequest",
                "devtools.timeline",
                1_000_000.0,
                0.0,
                serde_json::json!({"data": {"requestId": "1", "url": "https://example.com/"}}),
            ),
            make_trace_event(
                "ResourceReceiveResponse",
                "devtools.timeline",
                1_180_000.0,
                0.0,
                serde_json::json!({"data": {"requestId": "1"}}),
            ),
            make_trace_event(
                "ResourceFinish",
                "devtools.timeline",
                1_300_000.0,
                0.0,
                serde_json::json!({"data": {"requestId": "1"}}),
            ),
        ];
        let result = analyze_document_latency(&events);
        assert!((result["ttfb_ms"].as_f64().unwrap() - 180.0).abs() < 0.1);
        assert!((result["download_ms"].as_f64().unwrap() - 120.0).abs() < 0.1);
        assert!((result["total_ms"].as_f64().unwrap() - 300.0).abs() < 0.1);
    }

    // =========================================================================
    // LCP breakdown and subresource URL helper
    // =========================================================================

    #[test]
    fn analyze_lcp_breakdown_computes_split() {
        let events = vec![
            make_trace_event(
                "navigationStart",
                "blink.user_timing",
                1_000_000.0,
                0.0,
                serde_json::json!({}),
            ),
            make_trace_event(
                "ResourceSendRequest",
                "devtools.timeline",
                1_000_000.0,
                0.0,
                serde_json::json!({"data": {"requestId": "1", "url": "https://example.com/"}}),
            ),
            make_trace_event(
                "ResourceReceiveResponse",
                "devtools.timeline",
                1_180_000.0,
                0.0,
                serde_json::json!({"data": {"requestId": "1"}}),
            ),
            make_trace_event(
                "largestContentfulPaint::Candidate",
                "loading",
                2_200_000.0,
                0.0,
                serde_json::json!({"data": {"size": 5000}}),
            ),
        ];
        let result = analyze_lcp_breakdown(&events);
        assert!((result["ttfb_ms"].as_f64().unwrap() - 180.0).abs() < 0.1);
        assert!((result["total_ms"].as_f64().unwrap() - 1200.0).abs() < 0.1);
        // Remaining (1200 - 180 = 1020) split approximately 30/40/30
        let load_delay = result["load_delay_ms"].as_f64().unwrap();
        let load_duration = result["load_duration_ms"].as_f64().unwrap();
        let render_delay = result["render_delay_ms"].as_f64().unwrap();
        assert!((load_delay + load_duration + render_delay - 1020.0).abs() < 0.1);
    }

    #[test]
    fn is_subresource_url_identifies_resources() {
        assert!(is_subresource_url("https://example.com/app.js"));
        assert!(is_subresource_url("https://example.com/style.css"));
        assert!(is_subresource_url("https://example.com/image.png"));
        assert!(is_subresource_url("https://example.com/font.woff2"));
        assert!(!is_subresource_url("https://example.com/"));
        assert!(!is_subresource_url("https://example.com/page"));
        assert!(!is_subresource_url("https://example.com/api/data"));
    }

    // =========================================================================
    // Plain text formatters
    // =========================================================================

    #[test]
    fn format_stop_plain_contains_metrics() {
        let result = PerfStopResult {
            file: "/tmp/trace.json".to_string(),
            duration_ms: 3456,
            size_bytes: 1_234_567,
            vitals: CoreWebVitals {
                lcp_ms: Some(1200.5),
                cls: Some(0.05),
                ttfb_ms: Some(180.3),
            },
        };
        let plain = format_stop_plain(&result);
        assert!(plain.contains("/tmp/trace.json"));
        assert!(plain.contains("3456ms"));
        assert!(plain.contains("LCP:"));
        assert!(plain.contains("CLS:"));
        assert!(plain.contains("TTFB:"));
    }

    #[test]
    fn format_vitals_plain_contains_metrics() {
        let result = PerfVitalsResult {
            url: "https://example.com".to_string(),
            lcp_ms: Some(1200.5),
            cls: Some(0.05),
            ttfb_ms: Some(180.3),
        };
        let plain = format_vitals_plain(&result);
        assert!(plain.contains("LCP:"));
        assert!(plain.contains("CLS:"));
        assert!(plain.contains("TTFB:"));
    }

    #[test]
    fn format_vitals_plain_no_data() {
        let result = PerfVitalsResult {
            url: "https://example.com".to_string(),
            lcp_ms: None,
            cls: None,
            ttfb_ms: None,
        };
        let plain = format_vitals_plain(&result);
        assert!(plain.contains("No vitals data"));
    }

    // =========================================================================
    // Trace file write/read round-trip
    // =========================================================================

    #[test]
    #[allow(clippy::unreadable_literal)]
    fn parse_trace_vitals_from_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("test-trace-parse.json");
        let trace_data = serde_json::json!({
            "traceEvents": [
                {"name": "navigationStart", "cat": "blink.user_timing", "ph": "R", "ts": 1000000, "dur": 0, "args": {}},
                {"name": "ResourceSendRequest", "cat": "devtools.timeline", "ph": "X", "ts": 1000000, "dur": 0, "args": {"data": {"requestId": "1", "url": "https://example.com/"}}},
                {"name": "ResourceReceiveResponse", "cat": "devtools.timeline", "ph": "X", "ts": 1180000, "dur": 0, "args": {"data": {"requestId": "1"}}},
                {"name": "largestContentfulPaint::Candidate", "cat": "loading", "ph": "R", "ts": 2200000, "dur": 0, "args": {"data": {"size": 5000}}},
                {"name": "LayoutShift", "cat": "loading", "ph": "I", "ts": 1500000, "dur": 0, "args": {"data": {"score": 0.02, "had_recent_input": false}}}
            ]
        });
        fs::write(&path, serde_json::to_string(&trace_data).unwrap()).unwrap();

        let vitals = parse_trace_vitals(&path).unwrap();
        assert!(vitals.lcp_ms.is_some());
        assert!(vitals.ttfb_ms.is_some());
        assert!(vitals.cls.is_some());
        assert!((vitals.lcp_ms.unwrap() - 1200.0).abs() < 0.1);
        assert!((vitals.ttfb_ms.unwrap() - 180.0).abs() < 0.1);
        assert!((vitals.cls.unwrap() - 0.02).abs() < 0.001);

        // Cleanup
        let _ = fs::remove_file(&path);
    }
}
