use serde::Serialize;

use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, MediaArgs, MediaCommand, MediaSeekArgs, MediaTargetArgs};
use crate::output::{print_output, setup_session};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct MediaInfo {
    index: u32,
    tag: String,
    src: String,
    #[serde(rename = "currentSrc")]
    current_src: String,
    duration: Option<f64>,
    #[serde(rename = "currentTime")]
    current_time: f64,
    state: String,
    muted: bool,
    volume: f64,
    #[serde(rename = "loop")]
    loop_: bool,
    #[serde(rename = "readyState")]
    ready_state: u32,
}

// =============================================================================
// Output formatting
// =============================================================================

fn print_list_plain(items: &[MediaInfo]) {
    if items.is_empty() {
        println!("No media elements");
        return;
    }
    for item in items {
        let muted_label = if item.muted { " [muted]" } else { "" };
        println!(
            "[{}] {} — {} ({:.1}s / {}{muted_label})",
            item.index,
            item.tag,
            item.state,
            item.current_time,
            match item.duration {
                Some(d) => format!("{d:.1}s"),
                None => "unknown".into(),
            },
        );
    }
}

fn print_action_plain(item: &MediaInfo) {
    let muted_label = if item.muted { " [muted]" } else { "" };
    println!(
        "[{}] {} — {} ({:.1}s / {}{muted_label})",
        item.index,
        item.tag,
        item.state,
        item.current_time,
        match item.duration {
            Some(d) => format!("{d:.1}s"),
            None => "unknown".into(),
        },
    );
}

fn print_bulk_plain(items: &[MediaInfo]) {
    for item in items {
        print_action_plain(item);
    }
}

// =============================================================================
// JavaScript builders
// =============================================================================

/// JS to enumerate all audio/video elements and return their state.
fn build_list_js() -> String {
    r"JSON.stringify(Array.from(document.querySelectorAll('audio, video')).map((el, i) => ({
    index: i,
    tag: el.tagName.toLowerCase(),
    src: el.getAttribute('src') || '',
    currentSrc: el.currentSrc || '',
    duration: Number.isFinite(el.duration) ? el.duration : null,
    currentTime: el.currentTime,
    state: el.ended ? 'ended' : el.paused ? 'paused' : 'playing',
    muted: el.muted,
    volume: el.volume,
    loop: el.loop,
    readyState: el.readyState
})))"
        .to_string()
}

/// Build JS to perform an action on a single media element by index.
fn build_action_by_index_js(index: u32, action: &str) -> String {
    format!(
        r"(async () => {{
    const els = document.querySelectorAll('audio, video');
    if ({index} >= els.length) {{
        throw new Error('Media element at index {index} not found. Page has ' + els.length + ' media elements.');
    }}
    const el = els[{index}];
    {action}
    return JSON.stringify({{
        index: {index},
        tag: el.tagName.toLowerCase(),
        src: el.getAttribute('src') || '',
        currentSrc: el.currentSrc || '',
        duration: Number.isFinite(el.duration) ? el.duration : null,
        currentTime: el.currentTime,
        state: el.ended ? 'ended' : el.paused ? 'paused' : 'playing',
        muted: el.muted,
        volume: el.volume,
        loop: el.loop,
        readyState: el.readyState
    }});
}})()"
    )
}

/// Build JS to perform an action on a single media element by CSS selector.
fn build_action_by_selector_js(selector: &str, action: &str) -> String {
    let escaped = escape_js_string(selector);
    format!(
        r#"(async () => {{
    const el = document.querySelector('{escaped}');
    if (!el || (el.tagName !== 'AUDIO' && el.tagName !== 'VIDEO')) {{
        throw new Error("No media element matching selector '{escaped}' found.");
    }}
    const els = document.querySelectorAll('audio, video');
    let idx = 0;
    for (let i = 0; i < els.length; i++) {{ if (els[i] === el) {{ idx = i; break; }} }}
    {action}
    return JSON.stringify({{
        index: idx,
        tag: el.tagName.toLowerCase(),
        src: el.getAttribute('src') || '',
        currentSrc: el.currentSrc || '',
        duration: Number.isFinite(el.duration) ? el.duration : null,
        currentTime: el.currentTime,
        state: el.ended ? 'ended' : el.paused ? 'paused' : 'playing',
        muted: el.muted,
        volume: el.volume,
        loop: el.loop,
        readyState: el.readyState
    }});
}})()"#
    )
}

/// Build JS to perform an action on all media elements.
///
/// Uses `Promise.all` with async map to handle actions that return promises
/// (e.g., `await el.play()`). The outermost expression is a promise, so
/// callers must set `awaitPromise: true` on the CDP evaluate call.
fn build_bulk_action_js(action: &str) -> String {
    format!(
        r"Promise.all(Array.from(document.querySelectorAll('audio, video')).map(async (el, i) => {{
    {action}
    return {{
        index: i,
        tag: el.tagName.toLowerCase(),
        src: el.getAttribute('src') || '',
        currentSrc: el.currentSrc || '',
        duration: Number.isFinite(el.duration) ? el.duration : null,
        currentTime: el.currentTime,
        state: el.ended ? 'ended' : el.paused ? 'paused' : 'playing',
        muted: el.muted,
        volume: el.volume,
        loop: el.loop,
        readyState: el.readyState
    }};
}})).then(r => JSON.stringify(r))"
    )
}

/// Escape a string for safe inclusion in a JS single-quoted string literal.
fn escape_js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

/// Parse a target string into either a numeric index or a CSS selector.
enum MediaTarget {
    Index(u32),
    Selector(String),
}

fn parse_target(target: &str) -> MediaTarget {
    if let Some(selector) = target.strip_prefix("css:") {
        MediaTarget::Selector(selector.to_string())
    } else if let Ok(index) = target.parse::<u32>() {
        MediaTarget::Index(index)
    } else {
        // Treat as selector if it doesn't parse as an integer
        MediaTarget::Selector(target.to_string())
    }
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `media` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_media(global: &GlobalOpts, args: &MediaArgs) -> Result<(), AppError> {
    match &args.command {
        MediaCommand::List => execute_list(global, args.frame.as_deref()).await,
        MediaCommand::Play(target_args) => {
            execute_action(global, target_args, "play", args.frame.as_deref()).await
        }
        MediaCommand::Pause(target_args) => {
            execute_action(global, target_args, "pause", args.frame.as_deref()).await
        }
        MediaCommand::Seek(seek_args) => {
            execute_seek(global, seek_args, args.frame.as_deref()).await
        }
        MediaCommand::SeekEnd(target_args) => {
            execute_seek_end(global, target_args, args.frame.as_deref()).await
        }
    }
}

// =============================================================================
// List: enumerate all media elements
// =============================================================================

async fn execute_list(global: &GlobalOpts, frame: Option<&str>) -> Result<(), AppError> {
    let (client, mut managed) = setup_session(global).await?;

    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    let eff = if let Some(ref mut ctx) = frame_ctx {
        agentchrome::frame::frame_session_mut(ctx, &mut managed)
    } else {
        &mut managed
    };

    let js = build_list_js();
    let response = eff
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": js,
                "returnByValue": true
            })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("Runtime.evaluate failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let items = parse_media_list(&response)?;

    if global.output.plain {
        print_list_plain(&items);
        Ok(())
    } else {
        print_output(&items, &global.output)
    }
}

// =============================================================================
// Play / Pause: single or bulk
// =============================================================================

async fn execute_action(
    global: &GlobalOpts,
    args: &MediaTargetArgs,
    action_name: &str,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session(global).await?;

    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    let eff = if let Some(ref mut ctx) = frame_ctx {
        agentchrome::frame::frame_session_mut(ctx, &mut managed)
    } else {
        &mut managed
    };

    let js_action = match action_name {
        // play() returns a Promise; race it against a short timeout so we
        // don't block forever when the media is not yet buffered.
        "play" => "await Promise.race([el.play(), new Promise(r => setTimeout(r, 200))]);",
        "pause" => "el.pause();",
        _ => unreachable!(),
    };

    if args.all {
        let js = build_bulk_action_js(js_action);
        let response = evaluate_js(eff, &js).await?;
        let items = parse_media_list(&response)?;

        if global.output.plain {
            print_bulk_plain(&items);
            Ok(())
        } else {
            print_output(&items, &global.output)
        }
    } else {
        let target_str = args.target.as_deref().ok_or_else(|| AppError {
            message: format!(
                "media {action_name} requires a target (index or css:selector) or --all"
            ),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

        let target = parse_target(target_str);
        let js = match target {
            MediaTarget::Index(idx) => build_action_by_index_js(idx, js_action),
            MediaTarget::Selector(sel) => build_action_by_selector_js(&sel, js_action),
        };

        let response = evaluate_js(eff, &js).await?;
        let item = parse_single_media(&response)?;

        if global.output.plain {
            print_action_plain(&item);
            Ok(())
        } else {
            print_output(&item, &global.output)
        }
    }
}

// =============================================================================
// Seek: set currentTime to a specific value
// =============================================================================

async fn execute_seek(
    global: &GlobalOpts,
    args: &MediaSeekArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session(global).await?;

    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    let eff = if let Some(ref mut ctx) = frame_ctx {
        agentchrome::frame::frame_session_mut(ctx, &mut managed)
    } else {
        &mut managed
    };

    // Resolve the effective time from either positional or --time flag
    let effective_time = args.time_pos.or(args.time);

    if args.all {
        let time = effective_time.unwrap_or(0.0);
        let js_action = format!("el.currentTime = {time};");
        let js = build_bulk_action_js(&js_action);
        let response = evaluate_js(eff, &js).await?;
        let items = parse_media_list(&response)?;

        if global.output.plain {
            print_bulk_plain(&items);
            Ok(())
        } else {
            print_output(&items, &global.output)
        }
    } else {
        let target_str = args.target.as_deref().ok_or_else(|| AppError {
            message: "media seek requires a target (index or css:selector) or --all".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

        let time = effective_time.unwrap_or(0.0);
        let js_action = format!("el.currentTime = {time};");
        let target = parse_target(target_str);
        let js = match target {
            MediaTarget::Index(idx) => build_action_by_index_js(idx, &js_action),
            MediaTarget::Selector(sel) => build_action_by_selector_js(&sel, &js_action),
        };

        let response = evaluate_js(eff, &js).await?;
        let item = parse_single_media(&response)?;

        if global.output.plain {
            print_action_plain(&item);
            Ok(())
        } else {
            print_output(&item, &global.output)
        }
    }
}

// =============================================================================
// SeekEnd: set currentTime to duration
// =============================================================================

async fn execute_seek_end(
    global: &GlobalOpts,
    args: &MediaTargetArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session(global).await?;

    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    let eff = if let Some(ref mut ctx) = frame_ctx {
        agentchrome::frame::frame_session_mut(ctx, &mut managed)
    } else {
        &mut managed
    };

    if args.all {
        let bulk_action = "if (!Number.isFinite(el.duration)) { throw new Error('Media element at index ' + i + ' has no duration (NaN). Cannot seek to end.'); } el.currentTime = el.duration;";
        let js = build_bulk_action_js(bulk_action);
        let response = evaluate_js(eff, &js).await?;
        let items = parse_media_list(&response)?;

        if global.output.plain {
            print_bulk_plain(&items);
            Ok(())
        } else {
            print_output(&items, &global.output)
        }
    } else {
        let target_str = args.target.as_deref().ok_or_else(|| AppError {
            message: "media seek-end requires a target (index or css:selector) or --all".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

        let target = parse_target(target_str);
        let single_action = "if (!Number.isFinite(el.duration)) { throw new Error('Media element has no duration (NaN). Cannot seek to end.'); } el.currentTime = el.duration;";
        let js = match target {
            MediaTarget::Index(idx) => build_action_by_index_js(idx, single_action),
            MediaTarget::Selector(sel) => build_action_by_selector_js(&sel, single_action),
        };

        let response = evaluate_js(eff, &js).await?;
        let item = parse_single_media(&response)?;

        if global.output.plain {
            print_action_plain(&item);
            Ok(())
        } else {
            print_output(&item, &global.output)
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Evaluate JavaScript via `Runtime.evaluate` and check for exceptions.
///
/// Always sets `awaitPromise: true` because our action JS uses async IIFEs
/// and `Promise.all` for bulk operations.
async fn evaluate_js(
    managed: &mut agentchrome::connection::ManagedSession,
    expression: &str,
) -> Result<serde_json::Value, AppError> {
    let params = serde_json::json!({
        "expression": expression,
        "returnByValue": true,
        "awaitPromise": true,
    });

    let response = managed
        .send_command("Runtime.evaluate", Some(params))
        .await
        .map_err(|e| AppError {
            message: format!("Runtime.evaluate failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    // Check for JS exceptions
    if let Some(exception) = response.get("exceptionDetails") {
        let msg = exception["exception"]["description"]
            .as_str()
            .or_else(|| exception["text"].as_str())
            .unwrap_or("Unknown JavaScript error");
        return Err(AppError {
            message: msg.to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    Ok(response)
}

/// Parse a CDP `Runtime.evaluate` response that returns a JSON array string.
fn parse_media_list(response: &serde_json::Value) -> Result<Vec<MediaInfo>, AppError> {
    let json_str = response["result"]["value"].as_str().unwrap_or("[]");

    let items: Vec<serde_json::Value> = serde_json::from_str(json_str).map_err(|e| AppError {
        message: format!("Failed to parse media list: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    Ok(items.iter().map(parse_media_value).collect())
}

/// Parse a CDP `Runtime.evaluate` response that returns a single JSON object string.
fn parse_single_media(response: &serde_json::Value) -> Result<MediaInfo, AppError> {
    let json_str = response["result"]["value"]
        .as_str()
        .ok_or_else(|| AppError {
            message: "No result from media command".into(),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;

    let value: serde_json::Value = serde_json::from_str(json_str).map_err(|e| AppError {
        message: format!("Failed to parse media result: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

    Ok(parse_media_value(&value))
}

/// Convert a JSON value into a `MediaInfo` struct.
#[allow(clippy::cast_possible_truncation)]
fn parse_media_value(v: &serde_json::Value) -> MediaInfo {
    MediaInfo {
        index: v["index"].as_u64().unwrap_or(0) as u32,
        tag: v["tag"].as_str().unwrap_or("").to_string(),
        src: v["src"].as_str().unwrap_or("").to_string(),
        current_src: v["currentSrc"].as_str().unwrap_or("").to_string(),
        duration: v["duration"].as_f64(),
        current_time: v["currentTime"].as_f64().unwrap_or(0.0),
        state: v["state"].as_str().unwrap_or("unknown").to_string(),
        muted: v["muted"].as_bool().unwrap_or(false),
        volume: v["volume"].as_f64().unwrap_or(1.0),
        loop_: v["loop"].as_bool().unwrap_or(false),
        ready_state: v["readyState"].as_u64().unwrap_or(0) as u32,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_info_serialization() {
        let info = MediaInfo {
            index: 0,
            tag: "audio".into(),
            src: "narration.mp3".into(),
            current_src: "https://example.com/narration.mp3".into(),
            duration: Some(30.0),
            current_time: 0.0,
            state: "paused".into(),
            muted: false,
            volume: 1.0,
            loop_: false,
            ready_state: 4,
        };
        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert_eq!(json["index"], 0);
        assert_eq!(json["tag"], "audio");
        assert_eq!(json["src"], "narration.mp3");
        assert_eq!(json["currentSrc"], "https://example.com/narration.mp3");
        assert_eq!(json["duration"], 30.0);
        assert_eq!(json["currentTime"], 0.0);
        assert_eq!(json["state"], "paused");
        assert_eq!(json["muted"], false);
        assert_eq!(json["volume"], 1.0);
        assert_eq!(json["loop"], false);
        assert_eq!(json["readyState"], 4);
    }

    #[test]
    fn media_info_null_duration() {
        let info = MediaInfo {
            index: 0,
            tag: "audio".into(),
            src: String::new(),
            current_src: String::new(),
            duration: None,
            current_time: 0.0,
            state: "paused".into(),
            muted: false,
            volume: 1.0,
            loop_: false,
            ready_state: 0,
        };
        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert!(json["duration"].is_null());
    }

    #[test]
    fn media_info_loop_field_renamed() {
        let info = MediaInfo {
            index: 0,
            tag: "video".into(),
            src: String::new(),
            current_src: String::new(),
            duration: Some(60.0),
            current_time: 10.0,
            state: "playing".into(),
            muted: true,
            volume: 0.5,
            loop_: true,
            ready_state: 4,
        };
        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert_eq!(json["loop"], true);
        // Ensure the field is named "loop", not "loop_"
        assert!(json.get("loop_").is_none());
    }

    #[test]
    fn parse_target_index() {
        match parse_target("0") {
            MediaTarget::Index(0) => {}
            _ => panic!("Expected Index(0)"),
        }
        match parse_target("42") {
            MediaTarget::Index(42) => {}
            _ => panic!("Expected Index(42)"),
        }
    }

    #[test]
    fn parse_target_css_selector() {
        match parse_target("css:audio.narration") {
            MediaTarget::Selector(s) => assert_eq!(s, "audio.narration"),
            MediaTarget::Index(_) => panic!("Expected Selector"),
        }
    }

    #[test]
    fn parse_target_non_numeric_becomes_selector() {
        match parse_target("audio.narration") {
            MediaTarget::Selector(s) => assert_eq!(s, "audio.narration"),
            MediaTarget::Index(_) => panic!("Expected Selector"),
        }
    }

    #[test]
    fn escape_js_string_basic() {
        assert_eq!(escape_js_string("hello"), "hello");
        assert_eq!(escape_js_string("it's"), "it\\'s");
        assert_eq!(escape_js_string("a\\b"), "a\\\\b");
        assert_eq!(escape_js_string("line\nnew"), "line\\nnew");
    }

    #[test]
    fn build_list_js_is_valid() {
        let js = build_list_js();
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("audio, video"));
        assert!(js.contains("JSON.stringify"));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn parse_media_value_full() {
        let v = serde_json::json!({
            "index": 1,
            "tag": "video",
            "src": "intro.mp4",
            "currentSrc": "https://example.com/intro.mp4",
            "duration": 60.0,
            "currentTime": 10.5,
            "state": "playing",
            "muted": false,
            "volume": 0.8,
            "loop": true,
            "readyState": 4
        });
        let info = parse_media_value(&v);
        assert_eq!(info.index, 1);
        assert_eq!(info.tag, "video");
        assert_eq!(info.src, "intro.mp4");
        assert_eq!(info.current_src, "https://example.com/intro.mp4");
        assert_eq!(info.duration, Some(60.0));
        assert_eq!(info.current_time, 10.5);
        assert_eq!(info.state, "playing");
        assert!(!info.muted);
        assert!((info.volume - 0.8).abs() < f64::EPSILON);
        assert!(info.loop_);
        assert_eq!(info.ready_state, 4);
    }

    #[test]
    fn parse_media_value_null_duration() {
        let v = serde_json::json!({
            "index": 0,
            "tag": "audio",
            "src": "",
            "currentSrc": "",
            "duration": null,
            "currentTime": 0.0,
            "state": "paused",
            "muted": false,
            "volume": 1.0,
            "loop": false,
            "readyState": 0
        });
        let info = parse_media_value(&v);
        assert_eq!(info.duration, None);
    }

    #[test]
    fn parse_media_list_from_response() {
        let response = serde_json::json!({
            "result": {
                "type": "string",
                "value": r#"[{"index":0,"tag":"audio","src":"a.mp3","currentSrc":"a.mp3","duration":30.0,"currentTime":0.0,"state":"paused","muted":false,"volume":1.0,"loop":false,"readyState":4}]"#
            }
        });
        let items = parse_media_list(&response).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].tag, "audio");
        assert_eq!(items[0].duration, Some(30.0));
    }

    #[test]
    fn parse_media_list_empty() {
        let response = serde_json::json!({
            "result": {
                "type": "string",
                "value": "[]"
            }
        });
        let items = parse_media_list(&response).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn parse_single_media_from_response() {
        let response = serde_json::json!({
            "result": {
                "type": "string",
                "value": r#"{"index":0,"tag":"audio","src":"a.mp3","currentSrc":"a.mp3","duration":30.0,"currentTime":15.5,"state":"paused","muted":false,"volume":1.0,"loop":false,"readyState":4}"#
            }
        });
        let item = parse_single_media(&response).unwrap();
        assert_eq!(item.index, 0);
        assert_eq!(item.current_time, 15.5);
    }

    #[test]
    fn plain_text_list_empty() {
        // Just verify it doesn't panic
        print_list_plain(&[]);
    }

    #[test]
    fn plain_text_list_items() {
        let items = vec![
            MediaInfo {
                index: 0,
                tag: "audio".into(),
                src: String::new(),
                current_src: String::new(),
                duration: Some(30.0),
                current_time: 0.0,
                state: "paused".into(),
                muted: false,
                volume: 1.0,
                loop_: false,
                ready_state: 4,
            },
            MediaInfo {
                index: 1,
                tag: "video".into(),
                src: String::new(),
                current_src: String::new(),
                duration: None,
                current_time: 5.0,
                state: "playing".into(),
                muted: true,
                volume: 0.0,
                loop_: false,
                ready_state: 4,
            },
        ];
        // Just verify it doesn't panic
        print_list_plain(&items);
    }

    #[test]
    fn plain_text_action() {
        let item = MediaInfo {
            index: 0,
            tag: "audio".into(),
            src: String::new(),
            current_src: String::new(),
            duration: Some(30.0),
            current_time: 30.0,
            state: "ended".into(),
            muted: false,
            volume: 1.0,
            loop_: false,
            ready_state: 4,
        };
        // Just verify it doesn't panic
        print_action_plain(&item);
    }

    #[test]
    fn build_action_by_index_contains_index() {
        let js = build_action_by_index_js(3, "el.play();");
        assert!(js.contains('3'));
        assert!(js.contains("el.play()"));
    }

    #[test]
    fn build_action_by_selector_escapes_quotes() {
        let js = build_action_by_selector_js("audio.it's", "el.pause();");
        assert!(js.contains("audio.it\\'s"));
        assert!(js.contains("el.pause()"));
    }

    #[test]
    fn build_bulk_action_iterates_all() {
        let js = build_bulk_action_js("el.currentTime = el.duration;");
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("el.currentTime = el.duration;"));
    }
}
