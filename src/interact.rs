use std::time::Duration;

use serde::Serialize;

use agentchrome::connection::ManagedSession;
use agentchrome::error::AppError;

use crate::cli::{
    ClickArgs, ClickAtArgs, DragArgs, DragAtArgs, GlobalOpts, HoverArgs, InteractArgs,
    InteractCommand, KeyArgs, MouseButton, MouseDownAtArgs, MouseUpAtArgs, ScrollArgs,
    ScrollDirection, TypeArgs, WaitUntil,
};
use crate::coord_helpers::{frame_viewport_offset, resolve_element_box};
use crate::navigate::{DEFAULT_NAVIGATE_TIMEOUT_MS, wait_for_event, wait_for_network_idle};
use crate::output::{print_output, setup_session_with_interceptors};
use crate::snapshot;
use agentchrome::coords::{CoordValue, resolve_relative_coords};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
struct Coords {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct DragTargets {
    from: String,
    to: String,
}

#[derive(Serialize)]
struct ClickResult {
    clicked: String,
    url: String,
    navigated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    double_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    right_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ClickAtResult {
    clicked_at: Coords,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    navigated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    double_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    right_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct HoverResult {
    hovered: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct DragResult {
    dragged: DragTargets,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct DragAtCoords {
    from: Coords,
    to: Coords,
}

#[derive(Serialize)]
struct DragAtResult {
    dragged_at: DragAtCoords,
    #[serde(skip_serializing_if = "Option::is_none")]
    steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct MouseDownAtResult {
    mousedown_at: Coords,
    #[serde(skip_serializing_if = "Option::is_none")]
    button: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct MouseUpAtResult {
    mouseup_at: Coords,
    #[serde(skip_serializing_if = "Option::is_none")]
    button: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct TypeResult {
    typed: String,
    length: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct KeyResult {
    pressed: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct ScrollResult {
    scrolled: Coords,
    position: Coords,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<serde_json::Value>,
}

// =============================================================================
// Output formatting
// =============================================================================

fn print_click_plain(result: &ClickResult) {
    if result.double_click == Some(true) {
        println!("Double-clicked {}", result.clicked);
    } else if result.right_click == Some(true) {
        println!("Right-clicked {}", result.clicked);
    } else {
        println!("Clicked {}", result.clicked);
    }
}

fn print_click_at_plain(result: &ClickAtResult) {
    if result.double_click == Some(true) {
        println!(
            "Double-clicked at ({}, {})",
            result.clicked_at.x, result.clicked_at.y
        );
    } else if result.right_click == Some(true) {
        println!(
            "Right-clicked at ({}, {})",
            result.clicked_at.x, result.clicked_at.y
        );
    } else {
        println!(
            "Clicked at ({}, {})",
            result.clicked_at.x, result.clicked_at.y
        );
    }
}

fn print_hover_plain(result: &HoverResult) {
    println!("Hovered {}", result.hovered);
}

fn print_drag_plain(result: &DragResult) {
    println!("Dragged {} to {}", result.dragged.from, result.dragged.to);
}

fn print_drag_at_plain(result: &DragAtResult) {
    println!(
        "Dragged from ({}, {}) to ({}, {})",
        result.dragged_at.from.x,
        result.dragged_at.from.y,
        result.dragged_at.to.x,
        result.dragged_at.to.y
    );
}

fn print_mousedown_at_plain(result: &MouseDownAtResult) {
    println!(
        "Mousedown at ({}, {})",
        result.mousedown_at.x, result.mousedown_at.y
    );
}

fn print_mouseup_at_plain(result: &MouseUpAtResult) {
    println!(
        "Mouseup at ({}, {})",
        result.mouseup_at.x, result.mouseup_at.y
    );
}

fn print_type_plain(result: &TypeResult) {
    println!("Typed {} characters", result.length);
}

fn print_key_plain(result: &KeyResult) {
    println!("Pressed {}", result.pressed);
}

fn print_scroll_plain(result: &ScrollResult, mode: &str) {
    match mode {
        "to-top" => println!(
            "Scrolled to top at ({}, {})",
            result.position.x, result.position.y
        ),
        "to-bottom" => println!(
            "Scrolled to bottom at ({}, {})",
            result.position.x, result.position.y
        ),
        "to-element" => println!(
            "Scrolled to element at ({}, {})",
            result.position.x, result.position.y
        ),
        "container" => println!(
            "Scrolled container by ({}, {})",
            result.scrolled.x, result.scrolled.y
        ),
        _ => {
            let dir = if result.scrolled.y > 0.0 {
                "down"
            } else if result.scrolled.y < 0.0 {
                "up"
            } else if result.scrolled.x > 0.0 {
                "right"
            } else if result.scrolled.x < 0.0 {
                "left"
            } else {
                "by"
            };
            let amount = result.scrolled.x.abs().max(result.scrolled.y.abs());
            println!(
                "Scrolled {dir} {amount}px to ({}, {})",
                result.position.x, result.position.y
            );
        }
    }
}

// =============================================================================
// Target resolution helpers
// =============================================================================

/// Resolve a target (UID or CSS selector) to a backend DOM node ID.
///
/// For UIDs: reads snapshot state and looks up the backendDOMNodeId.
/// For CSS selectors: queries the DOM and resolves the node.
async fn resolve_target_to_backend_node_id(
    session: &ManagedSession,
    target: &str,
) -> Result<i64, AppError> {
    if snapshot::is_uid(target) {
        // Read snapshot state
        let state = snapshot::read_snapshot_state()?.ok_or_else(AppError::no_snapshot_state)?;

        // Lookup UID in the map
        let backend_node_id = state
            .uid_map
            .get(target)
            .copied()
            .ok_or_else(|| AppError::uid_not_found(target))?;

        Ok(backend_node_id)
    } else if snapshot::is_css_selector(target) {
        // Strip 'css:' prefix
        let selector = &target[4..];

        // Get document root node ID
        let doc_response = session.send_command("DOM.getDocument", None).await?;
        let root_node_id = doc_response["root"]["nodeId"]
            .as_i64()
            .ok_or_else(|| AppError::element_not_found(selector))?;

        // Query selector
        let query_params = serde_json::json!({
            "nodeId": root_node_id,
            "selector": selector,
        });
        let query_response = session
            .send_command("DOM.querySelector", Some(query_params))
            .await?;

        let node_id = query_response["nodeId"].as_i64().unwrap_or(0);
        if node_id == 0 {
            return Err(AppError::element_not_found(selector));
        }

        // Get backend node ID via describeNode
        let describe_params = serde_json::json!({ "nodeId": node_id });
        let describe_response = session
            .send_command("DOM.describeNode", Some(describe_params))
            .await?;

        let backend_node_id = describe_response["node"]["backendNodeId"]
            .as_i64()
            .ok_or_else(|| AppError::element_not_found(selector))?;

        Ok(backend_node_id)
    } else {
        Err(AppError::element_not_found(target))
    }
}

/// Get the center coordinates of an element by its backend node ID.
///
/// Returns (x, y) coordinates of the element's center.
async fn get_element_center(
    session: &ManagedSession,
    backend_node_id: i64,
) -> Result<(f64, f64), AppError> {
    let params = serde_json::json!({ "backendNodeId": backend_node_id });
    let response = session
        .send_command("DOM.getBoxModel", Some(params))
        .await
        .map_err(|e| AppError::interaction_failed("get_element_center", &e.to_string()))?;

    let content = response["model"]["content"]
        .as_array()
        .ok_or_else(|| AppError::element_zero_size("element"))?;

    if content.len() < 8 {
        return Err(AppError::element_zero_size("element"));
    }

    // content is [x1, y1, x2, y2, x3, y3, x4, y4]
    // Center = ((x1 + x3) / 2, (y1 + y3) / 2)
    let x1 = content[0].as_f64().unwrap_or(0.0);
    let y1 = content[1].as_f64().unwrap_or(0.0);
    let x3 = content[4].as_f64().unwrap_or(0.0);
    let y3 = content[5].as_f64().unwrap_or(0.0);

    let center_x = (x1 + x3) / 2.0;
    let center_y = (y1 + y3) / 2.0;

    // Check for zero-size
    if (x3 - x1).abs() < 1.0 || (y3 - y1).abs() < 1.0 {
        return Err(AppError::element_zero_size("element"));
    }

    Ok((center_x, center_y))
}

/// Scroll an element into view if needed.
async fn scroll_into_view(session: &ManagedSession, backend_node_id: i64) -> Result<(), AppError> {
    let params = serde_json::json!({ "backendNodeId": backend_node_id });
    session
        .send_command("DOM.scrollIntoViewIfNeeded", Some(params))
        .await
        .map_err(|e| AppError::interaction_failed("scroll_into_view", &e.to_string()))?;
    Ok(())
}

/// High-level function to resolve a target to coordinates.
///
/// Steps:
/// 1. Resolve target to backend node ID
/// 2. Scroll element into view
/// 3. Get element center coordinates
async fn resolve_target_coords(
    session: &ManagedSession,
    target: &str,
) -> Result<(f64, f64), AppError> {
    let backend_node_id = resolve_target_to_backend_node_id(session, target).await?;
    scroll_into_view(session, backend_node_id).await?;
    get_element_center(session, backend_node_id).await
}

// =============================================================================
// Mouse dispatch helpers
// =============================================================================

/// Dispatch a click (or double-click, or right-click) at the given coordinates.
///
/// - `button`: "left" or "right"
/// - `click_count`: 1 for single click, 2 for double click
async fn dispatch_click(
    session: &mut ManagedSession,
    x: f64,
    y: f64,
    button: &str,
    click_count: u8,
) -> Result<(), AppError> {
    if click_count == 2 {
        // For double-click, send: press(1) → release(1) → press(2) → release(2)
        // First click
        let press_params = serde_json::json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": 1,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(press_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_press", &e.to_string()))?;

        let release_params = serde_json::json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": 1,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(release_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_release", &e.to_string()))?;

        // Second click
        let press_params = serde_json::json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": 2,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(press_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_press", &e.to_string()))?;

        let release_params = serde_json::json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": 2,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(release_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_release", &e.to_string()))?;
    } else {
        // Single click
        let press_params = serde_json::json!({
            "type": "mousePressed",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": click_count,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(press_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_press", &e.to_string()))?;

        let release_params = serde_json::json!({
            "type": "mouseReleased",
            "x": x,
            "y": y,
            "button": button,
            "clickCount": click_count,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(release_params))
            .await
            .map_err(|e| AppError::interaction_failed("mouse_release", &e.to_string()))?;
    }

    Ok(())
}

/// Dispatch a hover (mouse move) to the given coordinates.
async fn dispatch_hover(session: &mut ManagedSession, x: f64, y: f64) -> Result<(), AppError> {
    let params = serde_json::json!({
        "type": "mouseMoved",
        "x": x,
        "y": y,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(params))
        .await
        .map_err(|e| AppError::interaction_failed("mouse_move", &e.to_string()))?;
    Ok(())
}

/// Dispatch a drag operation from (`from_x`, `from_y`) to (`to_x`, `to_y`).
async fn dispatch_drag(
    session: &mut ManagedSession,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
) -> Result<(), AppError> {
    // Press at start position
    let press_params = serde_json::json!({
        "type": "mousePressed",
        "x": from_x,
        "y": from_y,
        "button": "left",
        "clickCount": 1,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(press_params))
        .await
        .map_err(|e| AppError::interaction_failed("drag_press", &e.to_string()))?;

    // Move to end position
    let move_params = serde_json::json!({
        "type": "mouseMoved",
        "x": to_x,
        "y": to_y,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(move_params))
        .await
        .map_err(|e| AppError::interaction_failed("drag_move", &e.to_string()))?;

    // Release at end position
    let release_params = serde_json::json!({
        "type": "mouseReleased",
        "x": to_x,
        "y": to_y,
        "button": "left",
        "clickCount": 1,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(release_params))
        .await
        .map_err(|e| AppError::interaction_failed("drag_release", &e.to_string()))?;

    Ok(())
}

/// Dispatch a single `mousePressed` event at the given coordinates.
async fn dispatch_mousedown(
    session: &mut ManagedSession,
    x: f64,
    y: f64,
    button: &str,
) -> Result<(), AppError> {
    let press_params = serde_json::json!({
        "type": "mousePressed",
        "x": x,
        "y": y,
        "button": button,
        "clickCount": 1,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(press_params))
        .await
        .map_err(|e| AppError::interaction_failed("mouse_press", &e.to_string()))?;
    Ok(())
}

/// Dispatch a single `mouseReleased` event at the given coordinates.
async fn dispatch_mouseup(
    session: &mut ManagedSession,
    x: f64,
    y: f64,
    button: &str,
) -> Result<(), AppError> {
    let release_params = serde_json::json!({
        "type": "mouseReleased",
        "x": x,
        "y": y,
        "button": button,
        "clickCount": 1,
    });
    session
        .send_command("Input.dispatchMouseEvent", Some(release_params))
        .await
        .map_err(|e| AppError::interaction_failed("mouse_release", &e.to_string()))?;
    Ok(())
}

/// Dispatch a drag with N intermediate mousemove steps via linear interpolation.
///
/// When `steps == 1`, this is functionally equivalent to `dispatch_drag`.
/// When `steps > 1`, linearly interpolates N evenly-spaced points between source and target.
async fn dispatch_drag_interpolated(
    session: &mut ManagedSession,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    steps: u32,
) -> Result<(), AppError> {
    // Press at start position
    dispatch_mousedown(session, from_x, from_y, "left").await?;

    // Interpolate intermediate moves
    let steps = steps.max(1);
    for i in 1..=steps {
        let t = f64::from(i) / f64::from(steps);
        let x = from_x + (to_x - from_x) * t;
        let y = from_y + (to_y - from_y) * t;
        let move_params = serde_json::json!({
            "type": "mouseMoved",
            "x": x,
            "y": y,
        });
        session
            .send_command("Input.dispatchMouseEvent", Some(move_params))
            .await
            .map_err(|e| AppError::interaction_failed("drag_move", &e.to_string()))?;
    }

    // Release at end position
    dispatch_mouseup(session, to_x, to_y, "left").await?;

    Ok(())
}

// =============================================================================
// Keyboard key validation and mapping
// =============================================================================

/// Modifier key names.
const MODIFIER_KEYS: &[&str] = &["Alt", "Control", "Meta", "Shift"];

/// All valid key names (non-modifier).
const VALID_KEYS: &[&str] = &[
    // Letters
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    // Digits
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    // Function keys
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "F13",
    "F14",
    "F15",
    "F16",
    "F17",
    "F18",
    "F19",
    "F20",
    "F21",
    "F22",
    "F23",
    "F24",
    // Navigation
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    // Editing
    "Backspace",
    "Delete",
    "Insert",
    "Tab",
    "Enter",
    "Escape",
    // Whitespace
    "Space",
    // Numpad
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "NumpadAdd",
    "NumpadSubtract",
    "NumpadMultiply",
    "NumpadDivide",
    "NumpadDecimal",
    "NumpadEnter",
    // Media
    "MediaPlayPause",
    "MediaStop",
    "MediaTrackNext",
    "MediaTrackPrevious",
    "AudioVolumeUp",
    "AudioVolumeDown",
    "AudioVolumeMute",
    // Symbols
    "Minus",
    "Equal",
    "BracketLeft",
    "BracketRight",
    "Backslash",
    "Semicolon",
    "Quote",
    "Backquote",
    "Comma",
    "Period",
    "Slash",
    // Lock keys
    "CapsLock",
    "NumLock",
    "ScrollLock",
    // Other
    "ContextMenu",
    "PrintScreen",
    "Pause",
];

/// Check if a key name is a modifier.
fn is_modifier(key: &str) -> bool {
    MODIFIER_KEYS.contains(&key)
}

/// Check if a key name is valid (either a modifier or a known key).
fn is_valid_key(key: &str) -> bool {
    is_modifier(key) || VALID_KEYS.contains(&key)
}

/// Parsed key combination.
#[derive(Debug)]
struct ParsedKey {
    /// The modifier bitmask (1=Alt, 2=Control, 4=Meta, 8=Shift).
    modifiers: u8,
    /// The primary (non-modifier) key name.
    key: String,
}

/// Parse a key combination string like "Control+A" or "Enter".
///
/// Validates all parts and checks for duplicate modifiers.
fn parse_key_combination(input: &str) -> Result<ParsedKey, AppError> {
    let parts: Vec<&str> = input.split('+').collect();
    let mut modifiers: u8 = 0;
    let mut seen_modifiers: Vec<&str> = Vec::new();
    let mut primary_key: Option<&str> = None;

    for part in &parts {
        if !is_valid_key(part) {
            return Err(AppError::invalid_key(part));
        }

        if is_modifier(part) {
            if seen_modifiers.contains(part) {
                return Err(AppError::duplicate_modifier(part));
            }
            seen_modifiers.push(part);
            match *part {
                "Alt" => modifiers |= 1,
                "Control" => modifiers |= 2,
                "Meta" => modifiers |= 4,
                "Shift" => modifiers |= 8,
                _ => {}
            }
        } else {
            primary_key = Some(part);
        }
    }

    // If there's no primary key and only modifiers, use the last modifier as the key
    let key = match primary_key {
        Some(k) => k.to_string(),
        None => {
            // All parts are modifiers — use the last one as primary
            (*parts.last().unwrap_or(&"")).to_string()
        }
    };

    Ok(ParsedKey { modifiers, key })
}

/// Get the CDP `key` value for a key name.
fn cdp_key_value(key: &str) -> &str {
    match key {
        "Enter" => "Enter",
        "Tab" => "Tab",
        "Escape" => "Escape",
        "Backspace" => "Backspace",
        "Delete" => "Delete",
        "Insert" => "Insert",
        "Space" => " ",
        "ArrowUp" => "ArrowUp",
        "ArrowDown" => "ArrowDown",
        "ArrowLeft" => "ArrowLeft",
        "ArrowRight" => "ArrowRight",
        "Home" => "Home",
        "End" => "End",
        "PageUp" => "PageUp",
        "PageDown" => "PageDown",
        "Alt" => "Alt",
        "Control" => "Control",
        "Meta" => "Meta",
        "Shift" => "Shift",
        "CapsLock" => "CapsLock",
        "NumLock" => "NumLock",
        "ScrollLock" => "ScrollLock",
        "ContextMenu" => "ContextMenu",
        "PrintScreen" => "PrintScreen",
        "Pause" => "Pause",
        _ if key.starts_with('F') && key.len() >= 2 => key,
        _ if key.starts_with("Numpad") => key,
        _ if key.starts_with("Media") || key.starts_with("Audio") => key,
        // Single character keys
        _ if key.len() == 1 => key,
        // Symbol key names
        "Minus" => "-",
        "Equal" => "=",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Backslash" => "\\",
        "Semicolon" => ";",
        "Quote" => "'",
        "Backquote" => "`",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        _ => key,
    }
}

/// Get the CDP `code` value for a key name.
fn cdp_key_code(key: &str) -> String {
    match key {
        // Letters
        k if k.len() == 1 && k.chars().next().unwrap().is_ascii_alphabetic() => {
            format!("Key{}", k.to_uppercase())
        }
        // Digits
        k if k.len() == 1 && k.chars().next().unwrap().is_ascii_digit() => {
            format!("Digit{k}")
        }
        "Enter" => "Enter".to_string(),
        "Tab" => "Tab".to_string(),
        "Escape" => "Escape".to_string(),
        "Backspace" => "Backspace".to_string(),
        "Delete" => "Delete".to_string(),
        "Insert" => "Insert".to_string(),
        "Space" => "Space".to_string(),
        "ArrowUp" => "ArrowUp".to_string(),
        "ArrowDown" => "ArrowDown".to_string(),
        "ArrowLeft" => "ArrowLeft".to_string(),
        "ArrowRight" => "ArrowRight".to_string(),
        "Home" => "Home".to_string(),
        "End" => "End".to_string(),
        "PageUp" => "PageUp".to_string(),
        "PageDown" => "PageDown".to_string(),
        "Alt" => "AltLeft".to_string(),
        "Control" => "ControlLeft".to_string(),
        "Meta" => "MetaLeft".to_string(),
        "Shift" => "ShiftLeft".to_string(),
        "CapsLock" => "CapsLock".to_string(),
        "NumLock" => "NumLock".to_string(),
        "ScrollLock" => "ScrollLock".to_string(),
        "ContextMenu" => "ContextMenu".to_string(),
        "PrintScreen" => "PrintScreen".to_string(),
        "Pause" => "Pause".to_string(),
        "Minus" => "Minus".to_string(),
        "Equal" => "Equal".to_string(),
        "BracketLeft" => "BracketLeft".to_string(),
        "BracketRight" => "BracketRight".to_string(),
        "Backslash" => "Backslash".to_string(),
        "Semicolon" => "Semicolon".to_string(),
        "Quote" => "Quote".to_string(),
        "Backquote" => "Backquote".to_string(),
        "Comma" => "Comma".to_string(),
        "Period" => "Period".to_string(),
        "Slash" => "Slash".to_string(),
        k if k.starts_with('F') => k.to_string(),
        k if k.starts_with("Numpad") => k.to_string(),
        k if k.starts_with("Media") || k.starts_with("Audio") => k.to_string(),
        _ => key.to_string(),
    }
}

/// Get the Windows virtual-key code for a key name.
///
/// CDP populates `KeyboardEvent.keyCode` / `event.which` from this value;
/// omitting it causes jQuery-style handlers that short-circuit on `which === 0`
/// to ignore the event.
fn windows_virtual_key_code(key: &str) -> i64 {
    match key {
        "Backspace" => 8,
        "Tab" => 9,
        "Enter" => 13,
        "Shift" => 16,
        "Control" => 17,
        "Alt" => 18,
        "Pause" => 19,
        "CapsLock" => 20,
        "Escape" => 27,
        "Space" => 32,
        "PageUp" => 33,
        "PageDown" => 34,
        "End" => 35,
        "Home" => 36,
        "ArrowLeft" => 37,
        "ArrowUp" => 38,
        "ArrowRight" => 39,
        "ArrowDown" => 40,
        "PrintScreen" => 44,
        "Insert" => 45,
        "Delete" => 46,
        "Meta" => 91,
        "ContextMenu" => 93,
        "NumLock" => 144,
        "ScrollLock" => 145,
        "Semicolon" => 186,
        "Equal" => 187,
        "Comma" => 188,
        "Minus" => 189,
        "Period" => 190,
        "Slash" => 191,
        "Backquote" => 192,
        "BracketLeft" => 219,
        "Backslash" => 220,
        "BracketRight" => 221,
        "Quote" => 222,
        k if k.len() == 1 => {
            let c = k.chars().next().unwrap();
            if c.is_ascii_alphabetic() {
                c.to_ascii_uppercase() as i64
            } else if c.is_ascii_digit() {
                c as i64
            } else {
                0
            }
        }
        k if k.starts_with('F') && k.len() >= 2 => k[1..]
            .parse::<i64>()
            .ok()
            .filter(|n| (1..=24).contains(n))
            .map_or(0, |n| 111 + n),
        _ => 0,
    }
}

/// Get the text a real keyboard would produce for `keyDown` on a printable key.
///
/// Returns `None` for non-printable keys (navigation, function keys, bare
/// modifiers). CDP uses `text` on `keyDown` to drive `keypress` and `input`
/// events on focused form elements.
fn key_text(key: &str, modifiers: u8) -> Option<String> {
    let shift = modifiers & 8 != 0;
    match key {
        "Enter" => Some("\r".to_string()),
        "Tab" => Some("\t".to_string()),
        "Space" => Some(" ".to_string()),
        "Escape" | "Backspace" | "Delete" | "Insert" | "ArrowUp" | "ArrowDown" | "ArrowLeft"
        | "ArrowRight" | "Home" | "End" | "PageUp" | "PageDown" | "Alt" | "Control" | "Meta"
        | "Shift" | "CapsLock" | "NumLock" | "ScrollLock" | "ContextMenu" | "PrintScreen"
        | "Pause" => None,
        "Minus" => Some(if shift { "_" } else { "-" }.to_string()),
        "Equal" => Some(if shift { "+" } else { "=" }.to_string()),
        "BracketLeft" => Some(if shift { "{" } else { "[" }.to_string()),
        "BracketRight" => Some(if shift { "}" } else { "]" }.to_string()),
        "Backslash" => Some(if shift { "|" } else { "\\" }.to_string()),
        "Semicolon" => Some(if shift { ":" } else { ";" }.to_string()),
        "Quote" => Some(if shift { "\"" } else { "'" }.to_string()),
        "Backquote" => Some(if shift { "~" } else { "`" }.to_string()),
        "Comma" => Some(if shift { "<" } else { "," }.to_string()),
        "Period" => Some(if shift { ">" } else { "." }.to_string()),
        "Slash" => Some(if shift { "?" } else { "/" }.to_string()),
        k if k.starts_with('F') && k.len() >= 2 && k[1..].chars().all(|c| c.is_ascii_digit()) => {
            None
        }
        k if k.starts_with("Numpad") => None,
        k if k.starts_with("Media") || k.starts_with("Audio") => None,
        k if k.len() == 1 => {
            let c = k.chars().next().unwrap();
            if c.is_ascii_lowercase() && shift {
                Some(c.to_ascii_uppercase().to_string())
            } else if c.is_ascii_digit() && shift {
                let shifted = match c {
                    '1' => '!',
                    '2' => '@',
                    '3' => '#',
                    '4' => '$',
                    '5' => '%',
                    '6' => '^',
                    '7' => '&',
                    '8' => '*',
                    '9' => '(',
                    '0' => ')',
                    _ => c,
                };
                Some(shifted.to_string())
            } else {
                Some(c.to_string())
            }
        }
        _ => None,
    }
}

// =============================================================================
// Keyboard dispatch helpers
// =============================================================================

/// Dispatch a single key press (keyDown + keyUp) via CDP Input.dispatchKeyEvent.
async fn dispatch_key_press(
    session: &mut ManagedSession,
    key: &str,
    modifiers: u8,
) -> Result<(), AppError> {
    let key_value = cdp_key_value(key);
    let code = cdp_key_code(key);
    let vk = windows_virtual_key_code(key);
    let text = key_text(key, modifiers);

    // keyDown
    let mut down_params = serde_json::json!({
        "type": "keyDown",
        "key": key_value,
        "code": code,
        "modifiers": modifiers,
        "windowsVirtualKeyCode": vk,
    });
    if let Some(t) = text {
        down_params["text"] = serde_json::Value::String(t);
    }
    session
        .send_command("Input.dispatchKeyEvent", Some(down_params))
        .await
        .map_err(|e| AppError::interaction_failed("key_down", &e.to_string()))?;

    // keyUp
    let up_params = serde_json::json!({
        "type": "keyUp",
        "key": key_value,
        "code": code,
        "modifiers": modifiers,
        "windowsVirtualKeyCode": vk,
    });
    session
        .send_command("Input.dispatchKeyEvent", Some(up_params))
        .await
        .map_err(|e| AppError::interaction_failed("key_up", &e.to_string()))?;

    Ok(())
}

/// Dispatch typing a single character via CDP Input.dispatchKeyEvent.
async fn dispatch_char(session: &mut ManagedSession, ch: char) -> Result<(), AppError> {
    let text = ch.to_string();

    let params = serde_json::json!({
        "type": "char",
        "text": text,
    });
    session
        .send_command("Input.dispatchKeyEvent", Some(params))
        .await
        .map_err(|e| AppError::interaction_failed("char", &e.to_string()))?;

    Ok(())
}

/// Send a single modifier key event (keyDown or keyUp).
async fn dispatch_modifier_event(
    session: &mut ManagedSession,
    event_type: &str,
    key: &str,
    code: &str,
    modifiers: u8,
) -> Result<(), AppError> {
    let params = serde_json::json!({
        "type": event_type,
        "key": key,
        "code": code,
        "modifiers": modifiers,
        "windowsVirtualKeyCode": windows_virtual_key_code(key),
    });
    let action = if event_type == "keyDown" {
        "modifier_down"
    } else {
        "modifier_up"
    };
    session
        .send_command("Input.dispatchKeyEvent", Some(params))
        .await
        .map_err(|e| AppError::interaction_failed(action, &e.to_string()))?;
    Ok(())
}

/// Modifier keys with their bitmask, CDP key name, and CDP code.
const MODIFIER_MAP: &[(u8, &str, &str)] = &[
    (1, "Alt", "AltLeft"),
    (2, "Control", "ControlLeft"),
    (4, "Meta", "MetaLeft"),
    (8, "Shift", "ShiftLeft"),
];

/// Dispatch a key combination: press modifiers, press key, release key, release modifiers.
async fn dispatch_key_combination(
    session: &mut ManagedSession,
    parsed: &ParsedKey,
) -> Result<(), AppError> {
    let modifiers = parsed.modifiers;

    // Press modifier keys down
    for &(bit, key, code) in MODIFIER_MAP {
        if modifiers & bit != 0 {
            dispatch_modifier_event(session, "keyDown", key, code, modifiers).await?;
        }
    }

    // Press the primary key
    dispatch_key_press(session, &parsed.key, modifiers).await?;

    // Release modifier keys (reverse order)
    for &(bit, key, code) in MODIFIER_MAP.iter().rev() {
        if modifiers & bit != 0 {
            dispatch_modifier_event(session, "keyUp", key, code, 0).await?;
        }
    }

    Ok(())
}

// =============================================================================
// URL helper
// =============================================================================

/// Get the current page URL via `Runtime.evaluate`.
async fn get_current_url(managed: &ManagedSession) -> Result<String, AppError> {
    let url_response = managed
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "window.location.href" })),
        )
        .await?;
    Ok(url_response["result"]["value"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

// =============================================================================
// Snapshot refresh helper
// =============================================================================

/// Take a fresh snapshot and write it to snapshot state.
///
/// Returns the snapshot tree as a JSON value.
async fn take_snapshot(
    session: &mut ManagedSession,
    url: &str,
    compact: bool,
) -> Result<serde_json::Value, AppError> {
    // Enable Accessibility domain
    session.ensure_domain("Accessibility").await?;

    // Get full AX tree
    let response = session
        .send_command("Accessibility.getFullAXTree", None)
        .await?;

    let nodes = response["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("missing nodes array"))?;

    // Build tree
    let build_result = snapshot::build_tree(nodes, false);

    // Write snapshot state
    let state = snapshot::SnapshotState {
        url: url.to_string(),
        timestamp: agentchrome::session::now_iso8601(),
        uid_map: build_result.uid_map,
        frame_index: None,
        frame_id: None,
        aggregate: false,
        frame_uid_ranges: Vec::new(),
        frame_ids: Vec::new(),
    };
    snapshot::write_snapshot_state(&state)?;

    // Apply compact filtering if requested
    let root = if compact {
        snapshot::compact_tree(&build_result.root)
    } else {
        build_result.root
    };

    // Serialize root node as JSON
    let snapshot_json = serde_json::to_value(&root)
        .map_err(|e| AppError::snapshot_failed(&format!("failed to serialize snapshot: {e}")))?;

    Ok(snapshot_json)
}

// =============================================================================
// Scroll helpers
// =============================================================================

/// Read the current page scroll position (scrollX, scrollY).
async fn get_scroll_position(session: &ManagedSession) -> Result<(f64, f64), AppError> {
    let response = session
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": "JSON.stringify({x: window.scrollX, y: window.scrollY})",
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| AppError::interaction_failed("get_scroll_position", &e.to_string()))?;

    let json_str = response["result"]["value"]
        .as_str()
        .unwrap_or(r#"{"x":0,"y":0}"#);
    let pos: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| AppError::interaction_failed("get_scroll_position", &e.to_string()))?;

    Ok((
        pos["x"].as_f64().unwrap_or(0.0),
        pos["y"].as_f64().unwrap_or(0.0),
    ))
}

/// Read the current viewport dimensions (innerWidth, innerHeight).
async fn get_viewport_dimensions(session: &ManagedSession) -> Result<(f64, f64), AppError> {
    let response = session
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": "JSON.stringify({w: window.innerWidth, h: window.innerHeight})",
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| AppError::interaction_failed("get_viewport_dimensions", &e.to_string()))?;

    let json_str = response["result"]["value"]
        .as_str()
        .unwrap_or(r#"{"w":1024,"h":768}"#);
    let dims: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| AppError::interaction_failed("get_viewport_dimensions", &e.to_string()))?;

    Ok((
        dims["w"].as_f64().unwrap_or(1024.0),
        dims["h"].as_f64().unwrap_or(768.0),
    ))
}

/// Scroll the page by a delta using `window.scrollBy()`.
async fn dispatch_page_scroll(
    session: &ManagedSession,
    dx: f64,
    dy: f64,
    smooth: bool,
) -> Result<(), AppError> {
    let behavior = if smooth { "smooth" } else { "instant" };
    let expr = format!("window.scrollBy({{left: {dx}, top: {dy}, behavior: '{behavior}'}})");
    session
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": expr })),
        )
        .await
        .map_err(|e| AppError::interaction_failed("page_scroll", &e.to_string()))?;
    Ok(())
}

/// Scroll the page to an absolute position using `window.scrollTo()`.
async fn dispatch_page_scroll_to(
    session: &ManagedSession,
    x: f64,
    y: f64,
    smooth: bool,
) -> Result<(), AppError> {
    let behavior = if smooth { "smooth" } else { "instant" };
    let expr = format!("window.scrollTo({{left: {x}, top: {y}, behavior: '{behavior}'}})");
    session
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": expr })),
        )
        .await
        .map_err(|e| AppError::interaction_failed("page_scroll_to", &e.to_string()))?;
    Ok(())
}

/// Resolve a backend node ID to a Runtime object ID via DOM.resolveNode.
async fn resolve_to_object_id(
    session: &ManagedSession,
    backend_node_id: i64,
) -> Result<String, AppError> {
    let resolve_params = serde_json::json!({ "backendNodeId": backend_node_id });
    let resolve_response = session
        .send_command("DOM.resolveNode", Some(resolve_params))
        .await
        .map_err(|e| AppError::interaction_failed("resolve_node", &e.to_string()))?;

    resolve_response["object"]["objectId"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| AppError::interaction_failed("resolve_node", "no objectId returned"))
}

/// Resolve a raw CSS selector (no `css:` prefix) to a backend node ID.
async fn resolve_selector_to_backend_node_id(
    session: &ManagedSession,
    selector: &str,
) -> Result<i64, AppError> {
    resolve_target_to_backend_node_id(session, &format!("css:{selector}")).await
}

/// Check whether an element is scrollable (has overflow content).
/// Returns `Ok(())` if scrollable, or an error if not.
async fn check_element_scrollable(
    session: &ManagedSession,
    backend_node_id: i64,
    descriptor: &str,
) -> Result<(), AppError> {
    let object_id = resolve_to_object_id(session, backend_node_id).await?;
    let call_params = serde_json::json!({
        "objectId": object_id,
        "functionDeclaration": "function() { return JSON.stringify({ sh: this.scrollHeight, ch: this.clientHeight, sw: this.scrollWidth, cw: this.clientWidth }); }",
        "arguments": [],
        "returnByValue": true,
    });
    let response = session
        .send_command("Runtime.callFunctionOn", Some(call_params))
        .await
        .map_err(|e| AppError::interaction_failed("check_scrollable", &e.to_string()))?;

    let json_str = response["result"]["value"]
        .as_str()
        .unwrap_or(r#"{"sh":0,"ch":0,"sw":0,"cw":0}"#);
    let dims: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| AppError::interaction_failed("check_scrollable", &e.to_string()))?;

    let sh = dims["sh"].as_f64().unwrap_or(0.0);
    let ch = dims["ch"].as_f64().unwrap_or(0.0);
    let sw = dims["sw"].as_f64().unwrap_or(0.0);
    let cw = dims["cw"].as_f64().unwrap_or(0.0);

    if sh > ch || sw > cw {
        Ok(())
    } else {
        Err(AppError::element_not_scrollable(descriptor))
    }
}

/// Scroll a container element by a delta.
async fn dispatch_container_scroll(
    session: &ManagedSession,
    backend_node_id: i64,
    dx: f64,
    dy: f64,
    smooth: bool,
) -> Result<(), AppError> {
    let object_id = resolve_to_object_id(session, backend_node_id).await?;
    let behavior = if smooth { "smooth" } else { "instant" };
    let func = format!(
        "function() {{ this.scrollBy({{left: {dx}, top: {dy}, behavior: '{behavior}'}}); }}"
    );
    let call_params = serde_json::json!({
        "objectId": object_id,
        "functionDeclaration": func,
        "arguments": [],
    });
    session
        .send_command("Runtime.callFunctionOn", Some(call_params))
        .await
        .map_err(|e| AppError::interaction_failed("container_scroll", &e.to_string()))?;
    Ok(())
}

/// Read a container element's scroll position.
async fn get_container_scroll_position(
    session: &ManagedSession,
    backend_node_id: i64,
) -> Result<(f64, f64), AppError> {
    let object_id = resolve_to_object_id(session, backend_node_id).await?;
    let call_params = serde_json::json!({
        "objectId": object_id,
        "functionDeclaration": "function() { return JSON.stringify({x: this.scrollLeft, y: this.scrollTop}); }",
        "arguments": [],
        "returnByValue": true,
    });
    let response = session
        .send_command("Runtime.callFunctionOn", Some(call_params))
        .await
        .map_err(|e| AppError::interaction_failed("get_container_scroll", &e.to_string()))?;

    let json_str = response["result"]["value"]
        .as_str()
        .unwrap_or(r#"{"x":0,"y":0}"#);
    let pos: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| AppError::interaction_failed("get_container_scroll", &e.to_string()))?;

    Ok((
        pos["x"].as_f64().unwrap_or(0.0),
        pos["y"].as_f64().unwrap_or(0.0),
    ))
}

/// Wait for a smooth page scroll to finish by polling position until stable.
async fn wait_for_smooth_page_scroll(session: &ManagedSession) -> Result<(), AppError> {
    let mut last_pos = get_scroll_position(session).await?;
    for _ in 0..15 {
        // 15 × 200ms = 3s timeout
        tokio::time::sleep(Duration::from_millis(200)).await;
        let current_pos = get_scroll_position(session).await?;
        if (current_pos.0 - last_pos.0).abs() < 1.0 && (current_pos.1 - last_pos.1).abs() < 1.0 {
            return Ok(());
        }
        last_pos = current_pos;
    }
    Ok(())
}

/// Wait for a smooth container scroll to finish by polling position until stable.
async fn wait_for_smooth_container_scroll(
    session: &ManagedSession,
    backend_node_id: i64,
) -> Result<(), AppError> {
    let mut last_pos = get_container_scroll_position(session, backend_node_id).await?;
    for _ in 0..15 {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let current_pos = get_container_scroll_position(session, backend_node_id).await?;
        if (current_pos.0 - last_pos.0).abs() < 1.0 && (current_pos.1 - last_pos.1).abs() < 1.0 {
            return Ok(());
        }
        last_pos = current_pos;
    }
    Ok(())
}

// =============================================================================
// Command implementations
// =============================================================================

/// Get the document scroll height.
async fn get_document_scroll_height(session: &ManagedSession) -> Result<f64, AppError> {
    let response = session
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": "document.documentElement.scrollHeight",
                "returnByValue": true,
            })),
        )
        .await
        .map_err(|e| AppError::interaction_failed("get_scroll_height", &e.to_string()))?;
    Ok(response["result"]["value"].as_f64().unwrap_or(0.0))
}

/// Compute scroll delta and position change, returning (before, after) positions.
fn compute_delta(before: (f64, f64), after: (f64, f64)) -> (f64, f64, f64, f64) {
    (after.0 - before.0, after.1 - before.1, after.0, after.1)
}

/// Execute the `interact scroll` command.
#[allow(clippy::too_many_lines)] // Five mutually-exclusive scroll modes in one match
async fn execute_scroll(
    global: &GlobalOpts,
    args: &ScrollArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    let scroll_element = args.to_element.as_deref();
    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, scroll_element).await?;

    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("Runtime").await?;
        eff_mut.ensure_domain("DOM").await?;
    }

    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    let mode_label;
    let (scrolled_x, scrolled_y, final_x, final_y) = if let Some(ref target) = args.to_element {
        mode_label = "to-element";
        let before = get_scroll_position(effective).await?;
        let backend_node_id = resolve_target_to_backend_node_id(effective, target).await?;
        scroll_into_view(effective, backend_node_id).await?;
        compute_delta(before, get_scroll_position(effective).await?)
    } else if args.to_top {
        mode_label = "to-top";
        let before = get_scroll_position(effective).await?;
        dispatch_page_scroll_to(effective, 0.0, 0.0, args.smooth).await?;
        if args.smooth {
            wait_for_smooth_page_scroll(effective).await?;
        }
        compute_delta(before, get_scroll_position(effective).await?)
    } else if args.to_bottom {
        mode_label = "to-bottom";
        let before = get_scroll_position(effective).await?;
        let height = get_document_scroll_height(effective).await?;
        dispatch_page_scroll_to(effective, 0.0, height, args.smooth).await?;
        if args.smooth {
            wait_for_smooth_page_scroll(effective).await?;
        }
        compute_delta(before, get_scroll_position(effective).await?)
    } else if args.selector.is_some() || args.uid.is_some() {
        let (cid, descriptor) = if let Some(ref sel) = args.selector {
            mode_label = "selector";
            (
                resolve_selector_to_backend_node_id(effective, sel).await?,
                sel.clone(),
            )
        } else {
            let uid = args
                .uid
                .as_ref()
                .ok_or_else(|| AppError::interaction_failed("scroll", "uid argument missing"))?;
            mode_label = "uid";
            (
                resolve_target_to_backend_node_id(effective, uid).await?,
                uid.clone(),
            )
        };
        check_element_scrollable(effective, cid, &descriptor).await?;
        let before = get_container_scroll_position(effective, cid).await?;
        let (vw, vh) = get_viewport_dimensions(effective).await?;
        let (dx, dy) = compute_scroll_delta(args.direction, args.amount, vw, vh);
        dispatch_container_scroll(effective, cid, dx, dy, args.smooth).await?;
        if args.smooth {
            wait_for_smooth_container_scroll(effective, cid).await?;
        }
        compute_delta(before, get_container_scroll_position(effective, cid).await?)
    } else if let Some(ref container_target) = args.container {
        mode_label = "container";
        let cid = resolve_target_to_backend_node_id(effective, container_target).await?;
        let before = get_container_scroll_position(effective, cid).await?;
        let (vw, vh) = get_viewport_dimensions(effective).await?;
        let (dx, dy) = compute_scroll_delta(args.direction, args.amount, vw, vh);
        dispatch_container_scroll(effective, cid, dx, dy, args.smooth).await?;
        if args.smooth {
            wait_for_smooth_container_scroll(effective, cid).await?;
        }
        compute_delta(before, get_container_scroll_position(effective, cid).await?)
    } else {
        mode_label = "direction";
        let before = get_scroll_position(effective).await?;
        let (vw, vh) = get_viewport_dimensions(effective).await?;
        let (dx, dy) = compute_scroll_delta(args.direction, args.amount, vw, vh);
        dispatch_page_scroll(effective, dx, dy, args.smooth).await?;
        if args.smooth {
            wait_for_smooth_page_scroll(effective).await?;
        }
        compute_delta(before, get_scroll_position(effective).await?)
    };

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let url = get_current_url(effective).await?;
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    let result = ScrollResult {
        scrolled: Coords {
            x: scrolled_x,
            y: scrolled_y,
        },
        position: Coords {
            x: final_x,
            y: final_y,
        },
        snapshot,
    };

    if global.output.plain {
        print_scroll_plain(&result, mode_label);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Compute scroll delta (dx, dy) from direction, optional amount, and viewport dimensions.
fn compute_scroll_delta(
    direction: ScrollDirection,
    amount: Option<u32>,
    viewport_width: f64,
    viewport_height: f64,
) -> (f64, f64) {
    match direction {
        ScrollDirection::Down => (0.0, amount.map_or(viewport_height, f64::from)),
        ScrollDirection::Up => (0.0, -amount.map_or(viewport_height, f64::from)),
        ScrollDirection::Right => (amount.map_or(viewport_width, f64::from), 0.0),
        ScrollDirection::Left => (-amount.map_or(viewport_width, f64::from), 0.0),
    }
}

/// Execute the `interact click` command.
async fn execute_click(
    global: &GlobalOpts,
    args: &ClickArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session_with_interceptors(global).await?;
    let (_dismiss, dialog_settle_rx) = if global.auto_dismiss_dialogs {
        let (handle, rx) = managed.spawn_auto_dismiss_with_settle().await?;
        (Some(handle), Some(rx))
    } else {
        (None, None)
    };

    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, Some(&args.target))
            .await?;

    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("DOM").await?;
    }

    managed.ensure_domain("Page").await?;

    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    // Resolve target coordinates via the effective (frame-scoped) session
    let (x, y) = resolve_target_coords(effective, &args.target).await?;

    // Determine button and click count
    let button = if args.right { "right" } else { "left" };
    let click_count = if args.double { 2 } else { 1 };

    // Get pre-click URL for comparison
    let pre_url = get_current_url(&managed).await?;

    let navigated;

    match args.wait_until {
        Some(WaitUntil::None) => {
            // Dispatch and return immediately — no grace period, no navigation check
            dispatch_click(&mut managed, x, y, button, click_count).await?;
            navigated = false;
        }
        Some(WaitUntil::Load) => {
            let wait_rx = managed.subscribe("Page.loadEventFired").await?;
            dispatch_click(&mut managed, x, y, button, click_count).await?;
            let timeout_ms = global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);
            wait_for_event(wait_rx, timeout_ms, "load").await?;
            navigated = true;
        }
        Some(WaitUntil::Domcontentloaded) => {
            let wait_rx = managed.subscribe("Page.domContentEventFired").await?;
            dispatch_click(&mut managed, x, y, button, click_count).await?;
            let timeout_ms = global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);
            wait_for_event(wait_rx, timeout_ms, "domcontentloaded").await?;
            navigated = true;
        }
        Some(WaitUntil::Networkidle) => {
            managed.ensure_domain("Network").await?;
            let req_rx = managed.subscribe("Network.requestWillBeSent").await?;
            let fin_rx = managed.subscribe("Network.loadingFinished").await?;
            let fail_rx = managed.subscribe("Network.loadingFailed").await?;
            dispatch_click(&mut managed, x, y, button, click_count).await?;
            let timeout_ms = global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);
            wait_for_network_idle(req_rx, fin_rx, fail_rx, timeout_ms).await?;
            let post_url = get_current_url(&managed).await?;
            navigated = post_url != pre_url;
        }
        None => {
            // Legacy behavior: 100ms grace period with non-blocking navigation check
            let mut nav_rx = managed.subscribe("Page.frameNavigated").await?;
            dispatch_click(&mut managed, x, y, button, click_count).await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
            navigated = nav_rx.try_recv().is_ok();
        }
    }

    if let Some(mut rx) = dialog_settle_rx {
        agentchrome::connection::ManagedSession::await_dialog_settle(&mut rx).await;
    }

    // Get current URL
    let url = get_current_url(&managed).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    // Build result
    let result = ClickResult {
        clicked: args.target.clone(),
        url,
        navigated,
        double_click: if args.double { Some(true) } else { None },
        right_click: if args.right { Some(true) } else { None },
        snapshot,
    };

    if global.output.plain {
        print_click_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Extract a pixel value from a `CoordValue`, returning an error if it is a percentage
/// and no `--relative-to` was provided.
fn extract_pixels_no_relative_to(v: CoordValue, axis: &str) -> Result<f64, AppError> {
    match v {
        CoordValue::Pixels(px) => Ok(px),
        CoordValue::Percent(_) => Err(AppError {
            message: format!("percentage coordinates require --relative-to (axis: {axis})"),
            code: agentchrome::error::ExitCode::GeneralError,
            custom_json: None,
        }),
    }
}

/// Execute the `interact click-at` command.
#[allow(clippy::too_many_lines)]
async fn execute_click_at(
    global: &GlobalOpts,
    args: &ClickAtArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    // Validate percentage coordinates before establishing a session so that the error message
    // references --relative-to even when Chrome is not reachable.
    if args.relative_to.is_none() {
        extract_pixels_no_relative_to(args.x, "x")?;
        extract_pixels_no_relative_to(args.y, "y")?;
    }

    let (client, mut managed) = setup_session_with_interceptors(global).await?;
    let (_dismiss, dialog_settle_rx) = if global.auto_dismiss_dialogs {
        let (handle, rx) = managed.spawn_auto_dismiss_with_settle().await?;
        (Some(handle), Some(rx))
    } else {
        (None, None)
    };

    // Resolve frame context for coordinate translation
    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    let (click_x, click_y, output_x, output_y) = if let Some(ref selector) = args.relative_to {
        // Ensure DOM domain is available
        {
            let eff_mut = if let Some(ref mut ctx) = frame_ctx {
                agentchrome::frame::frame_session_mut(ctx, &mut managed)
            } else {
                &mut managed
            };
            eff_mut.ensure_domain("DOM").await?;
        }
        // Resolve element bounding box and frame offset
        let element_box = resolve_element_box(&managed, frame_ctx.as_ref(), selector).await?;
        let frame_offset = if let Some(ref ctx) = frame_ctx {
            frame_viewport_offset(&managed, ctx).await?
        } else {
            (0.0, 0.0)
        };
        let (rx, ry) = resolve_relative_coords(args.x, args.y, element_box, frame_offset);
        (rx, ry, rx, ry)
    } else {
        // Existing absolute-coordinate path — unchanged behaviour
        let px = extract_pixels_no_relative_to(args.x, "x")?;
        let py = extract_pixels_no_relative_to(args.y, "y")?;
        let (offset_x, offset_y) = if let Some(ref ctx) = frame_ctx {
            frame_viewport_offset(&managed, ctx).await?
        } else {
            (0.0, 0.0)
        };
        (px + offset_x, py + offset_y, px, py)
    };

    // Determine button and click count
    let button = if args.right { "right" } else { "left" };
    let click_count = if args.double { 2 } else { 1 };

    let (url, navigated) = match args.wait_until {
        Some(WaitUntil::None) => {
            dispatch_click(&mut managed, click_x, click_y, button, click_count).await?;
            (None, None)
        }
        Some(WaitUntil::Load) => {
            managed.ensure_domain("Page").await?;
            let pre_url = get_current_url(&managed).await?;
            let wait_rx = managed.subscribe("Page.loadEventFired").await?;
            dispatch_click(&mut managed, click_x, click_y, button, click_count).await?;
            let timeout_ms = global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);
            wait_for_event(wait_rx, timeout_ms, "load").await?;
            let post_url = get_current_url(&managed).await?;
            let nav = post_url != pre_url;
            (Some(post_url), Some(nav))
        }
        Some(WaitUntil::Domcontentloaded) => {
            managed.ensure_domain("Page").await?;
            let pre_url = get_current_url(&managed).await?;
            let wait_rx = managed.subscribe("Page.domContentEventFired").await?;
            dispatch_click(&mut managed, click_x, click_y, button, click_count).await?;
            let timeout_ms = global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);
            wait_for_event(wait_rx, timeout_ms, "domcontentloaded").await?;
            let post_url = get_current_url(&managed).await?;
            let nav = post_url != pre_url;
            (Some(post_url), Some(nav))
        }
        Some(WaitUntil::Networkidle) => {
            managed.ensure_domain("Network").await?;
            let pre_url = get_current_url(&managed).await?;
            let req_rx = managed.subscribe("Network.requestWillBeSent").await?;
            let fin_rx = managed.subscribe("Network.loadingFinished").await?;
            let fail_rx = managed.subscribe("Network.loadingFailed").await?;
            dispatch_click(&mut managed, click_x, click_y, button, click_count).await?;
            let timeout_ms = global.timeout.unwrap_or(DEFAULT_NAVIGATE_TIMEOUT_MS);
            wait_for_network_idle(req_rx, fin_rx, fail_rx, timeout_ms).await?;
            let post_url = get_current_url(&managed).await?;
            let nav = post_url != pre_url;
            (Some(post_url), Some(nav))
        }
        None => {
            // Legacy behavior: dispatch click, no navigation check
            dispatch_click(&mut managed, click_x, click_y, button, click_count).await?;
            (None, None)
        }
    };

    if let Some(mut rx) = dialog_settle_rx {
        agentchrome::connection::ManagedSession::await_dialog_settle(&mut rx).await;
    }

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let snap_url = if let Some(ref u) = url {
            u.clone()
        } else {
            get_current_url(&managed).await?
        };
        Some(take_snapshot(&mut managed, &snap_url, args.compact).await?)
    } else {
        None
    };

    // Build result — `clicked_at` reports resolved page-global coords when --relative-to is
    // present; otherwise it echoes the raw input (existing behavior).
    let result = ClickAtResult {
        clicked_at: Coords {
            x: output_x,
            y: output_y,
        },
        url,
        navigated,
        double_click: if args.double { Some(true) } else { None },
        right_click: if args.right { Some(true) } else { None },
        snapshot,
    };

    if global.output.plain {
        print_click_at_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact hover` command.
async fn execute_hover(
    global: &GlobalOpts,
    args: &HoverArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, Some(&args.target))
            .await?;

    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("DOM").await?;
        eff_mut.ensure_domain("Runtime").await?;
    }

    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    // Resolve target coordinates via the effective session
    let (x, y) = resolve_target_coords(effective, &args.target).await?;

    // Dispatch hover (always on main page session)
    dispatch_hover(&mut managed, x, y).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let url = get_current_url(&managed).await?;
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    // Build result
    let result = HoverResult {
        hovered: args.target.clone(),
        snapshot,
    };

    if global.output.plain {
        print_hover_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact drag` command.
async fn execute_drag(
    global: &GlobalOpts,
    args: &DragArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, Some(&args.from))
            .await?;

    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("DOM").await?;
        eff_mut.ensure_domain("Runtime").await?;
    }

    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    // Resolve "from" target via effective session
    let from_backend_id = resolve_target_to_backend_node_id(effective, &args.from).await?;
    scroll_into_view(effective, from_backend_id).await?;
    let (from_x, from_y) = get_element_center(effective, from_backend_id).await?;

    // Resolve "to" target via effective session
    let (to_x, to_y) = resolve_target_coords(effective, &args.to).await?;

    // Dispatch drag (always on main page session)
    dispatch_drag(&mut managed, from_x, from_y, to_x, to_y).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let url = get_current_url(&managed).await?;
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    // Build result
    let result = DragResult {
        dragged: DragTargets {
            from: args.from.clone(),
            to: args.to.clone(),
        },
        snapshot,
    };

    if global.output.plain {
        print_drag_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Convert a `MouseButton` enum to the CDP button string.
fn mouse_button_to_cdp(button: Option<&MouseButton>) -> &'static str {
    match button {
        Some(MouseButton::Right) => "right",
        Some(MouseButton::Middle) => "middle",
        _ => "left",
    }
}

/// Return the button name for output (only when non-default).
fn mouse_button_for_output(button: Option<&MouseButton>) -> Option<String> {
    match button {
        Some(MouseButton::Right) => Some("right".to_string()),
        Some(MouseButton::Middle) => Some("middle".to_string()),
        _ => None,
    }
}

/// Execute the `interact drag-at` command.
#[allow(clippy::similar_names)]
async fn execute_drag_at(
    global: &GlobalOpts,
    args: &DragAtArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    // Validate percentage coordinates before establishing a session.
    if args.relative_to.is_none() {
        extract_pixels_no_relative_to(args.from_x, "from_x")?;
        extract_pixels_no_relative_to(args.from_y, "from_y")?;
        extract_pixels_no_relative_to(args.to_x, "to_x")?;
        extract_pixels_no_relative_to(args.to_y, "to_y")?;
    }

    let (client, mut managed) = setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Resolve frame context for coordinate translation
    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    let (from_x, from_y, to_x, to_y, out_from_x, out_from_y, out_to_x, out_to_y) =
        if let Some(ref selector) = args.relative_to {
            // Ensure DOM domain is available
            {
                let eff_mut = if let Some(ref mut ctx) = frame_ctx {
                    agentchrome::frame::frame_session_mut(ctx, &mut managed)
                } else {
                    &mut managed
                };
                eff_mut.ensure_domain("DOM").await?;
            }
            let element_box = resolve_element_box(&managed, frame_ctx.as_ref(), selector).await?;
            let frame_offset = if let Some(ref ctx) = frame_ctx {
                frame_viewport_offset(&managed, ctx).await?
            } else {
                (0.0, 0.0)
            };
            let (fx, fy) =
                resolve_relative_coords(args.from_x, args.from_y, element_box, frame_offset);
            let (tx, ty) = resolve_relative_coords(args.to_x, args.to_y, element_box, frame_offset);
            (fx, fy, tx, ty, fx, fy, tx, ty)
        } else {
            let from_px = extract_pixels_no_relative_to(args.from_x, "from_x")?;
            let from_py = extract_pixels_no_relative_to(args.from_y, "from_y")?;
            let to_px = extract_pixels_no_relative_to(args.to_x, "to_x")?;
            let to_py = extract_pixels_no_relative_to(args.to_y, "to_y")?;
            let (offset_x, offset_y) = if let Some(ref ctx) = frame_ctx {
                frame_viewport_offset(&managed, ctx).await?
            } else {
                (0.0, 0.0)
            };
            (
                from_px + offset_x,
                from_py + offset_y,
                to_px + offset_x,
                to_py + offset_y,
                from_px,
                from_py,
                to_px,
                to_py,
            )
        };

    // Dispatch the drag
    let steps = args.steps.unwrap_or(1);
    dispatch_drag_interpolated(&mut managed, from_x, from_y, to_x, to_y, steps).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let url = get_current_url(&managed).await?;
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    // Build result — from/to report resolved page-global coords when --relative-to is present
    let result = DragAtResult {
        dragged_at: DragAtCoords {
            from: Coords {
                x: out_from_x,
                y: out_from_y,
            },
            to: Coords {
                x: out_to_x,
                y: out_to_y,
            },
        },
        steps: if steps > 1 { Some(steps) } else { None },
        snapshot,
    };

    if global.output.plain {
        print_drag_at_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact mousedown-at` command.
async fn execute_mousedown_at(
    global: &GlobalOpts,
    args: &MouseDownAtArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    // Validate percentage coordinates before establishing a session.
    if args.relative_to.is_none() {
        extract_pixels_no_relative_to(args.x, "x")?;
        extract_pixels_no_relative_to(args.y, "y")?;
    }

    let (client, mut managed) = setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Resolve frame context for coordinate translation
    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    let (x, y, out_x, out_y) = if let Some(ref selector) = args.relative_to {
        {
            let eff_mut = if let Some(ref mut ctx) = frame_ctx {
                agentchrome::frame::frame_session_mut(ctx, &mut managed)
            } else {
                &mut managed
            };
            eff_mut.ensure_domain("DOM").await?;
        }
        let element_box = resolve_element_box(&managed, frame_ctx.as_ref(), selector).await?;
        let frame_offset = if let Some(ref ctx) = frame_ctx {
            frame_viewport_offset(&managed, ctx).await?
        } else {
            (0.0, 0.0)
        };
        let (rx, ry) = resolve_relative_coords(args.x, args.y, element_box, frame_offset);
        (rx, ry, rx, ry)
    } else {
        let px = extract_pixels_no_relative_to(args.x, "x")?;
        let py = extract_pixels_no_relative_to(args.y, "y")?;
        let (offset_x, offset_y) = if let Some(ref ctx) = frame_ctx {
            frame_viewport_offset(&managed, ctx).await?
        } else {
            (0.0, 0.0)
        };
        (px + offset_x, py + offset_y, px, py)
    };

    let button = mouse_button_to_cdp(args.button.as_ref());
    dispatch_mousedown(&mut managed, x, y, button).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let url = get_current_url(&managed).await?;
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    // Build result — mousedown_at reports resolved page-global coords when --relative-to is used
    let result = MouseDownAtResult {
        mousedown_at: Coords { x: out_x, y: out_y },
        button: mouse_button_for_output(args.button.as_ref()),
        snapshot,
    };

    if global.output.plain {
        print_mousedown_at_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact mouseup-at` command.
async fn execute_mouseup_at(
    global: &GlobalOpts,
    args: &MouseUpAtArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    // Validate percentage coordinates before establishing a session.
    if args.relative_to.is_none() {
        extract_pixels_no_relative_to(args.x, "x")?;
        extract_pixels_no_relative_to(args.y, "y")?;
    }

    let (client, mut managed) = setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Resolve frame context for coordinate translation
    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    let (x, y, out_x, out_y) = if let Some(ref selector) = args.relative_to {
        {
            let eff_mut = if let Some(ref mut ctx) = frame_ctx {
                agentchrome::frame::frame_session_mut(ctx, &mut managed)
            } else {
                &mut managed
            };
            eff_mut.ensure_domain("DOM").await?;
        }
        let element_box = resolve_element_box(&managed, frame_ctx.as_ref(), selector).await?;
        let frame_offset = if let Some(ref ctx) = frame_ctx {
            frame_viewport_offset(&managed, ctx).await?
        } else {
            (0.0, 0.0)
        };
        let (rx, ry) = resolve_relative_coords(args.x, args.y, element_box, frame_offset);
        (rx, ry, rx, ry)
    } else {
        let px = extract_pixels_no_relative_to(args.x, "x")?;
        let py = extract_pixels_no_relative_to(args.y, "y")?;
        let (offset_x, offset_y) = if let Some(ref ctx) = frame_ctx {
            frame_viewport_offset(&managed, ctx).await?
        } else {
            (0.0, 0.0)
        };
        (px + offset_x, py + offset_y, px, py)
    };

    let button = mouse_button_to_cdp(args.button.as_ref());
    dispatch_mouseup(&mut managed, x, y, button).await?;

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        let url = get_current_url(&managed).await?;
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    // Build result — mouseup_at reports resolved page-global coords when --relative-to is used
    let result = MouseUpAtResult {
        mouseup_at: Coords { x: out_x, y: out_y },
        button: mouse_button_for_output(args.button.as_ref()),
        snapshot,
    };

    if global.output.plain {
        print_mouseup_at_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact type` command.
async fn execute_type(
    global: &GlobalOpts,
    args: &TypeArgs,
    _frame: Option<&str>,
) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    let text = &args.text;
    let length = text.chars().count();

    // Type each character (keyboard events go to whichever element has focus,
    // which is typically the main page session regardless of --frame)
    for ch in text.chars() {
        dispatch_char(&mut managed, ch).await?;

        if args.delay > 0 {
            tokio::time::sleep(Duration::from_millis(args.delay)).await;
        }
    }

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        managed.ensure_domain("Runtime").await?;
        let url = get_current_url(&managed).await?;
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    let result = TypeResult {
        typed: text.clone(),
        length,
        snapshot,
    };

    if global.output.plain {
        print_type_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

/// Execute the `interact key` command.
async fn execute_key(
    global: &GlobalOpts,
    args: &KeyArgs,
    _frame: Option<&str>,
) -> Result<(), AppError> {
    // Validate the key combination before connecting to Chrome
    let parsed = parse_key_combination(&args.keys)?;

    let (_client, mut managed) = setup_session_with_interceptors(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Press the key combination (key events are always dispatched at the main page level)
    for _ in 0..args.repeat {
        if parsed.modifiers != 0 {
            dispatch_key_combination(&mut managed, &parsed).await?;
        } else {
            dispatch_key_press(&mut managed, &parsed.key, 0).await?;
        }
    }

    // Take snapshot if requested
    let snapshot = if args.include_snapshot {
        managed.ensure_domain("Runtime").await?;
        let url = get_current_url(&managed).await?;
        Some(take_snapshot(&mut managed, &url, args.compact).await?)
    } else {
        None
    };

    let result = KeyResult {
        pressed: args.keys.clone(),
        repeat: if args.repeat > 1 {
            Some(args.repeat)
        } else {
            None
        },
        snapshot,
    };

    if global.output.plain {
        print_key_plain(&result);
        Ok(())
    } else {
        print_output(&result, &global.output)
    }
}

// =============================================================================
// Dispatcher
// =============================================================================

/// Execute the `interact` subcommand group.
///
/// # Errors
///
/// Returns `AppError` if the subcommand fails.
pub async fn execute_interact(global: &GlobalOpts, args: &InteractArgs) -> Result<(), AppError> {
    let frame = args.frame.as_deref();
    match &args.command {
        InteractCommand::Click(click_args) => execute_click(global, click_args, frame).await,
        InteractCommand::ClickAt(click_at_args) => {
            execute_click_at(global, click_at_args, frame).await
        }
        InteractCommand::Hover(hover_args) => execute_hover(global, hover_args, frame).await,
        InteractCommand::Drag(drag_args) => execute_drag(global, drag_args, frame).await,
        InteractCommand::DragAt(drag_at_args) => execute_drag_at(global, drag_at_args, frame).await,
        InteractCommand::MouseDownAt(mousedown_args) => {
            execute_mousedown_at(global, mousedown_args, frame).await
        }
        InteractCommand::MouseUpAt(mouseup_args) => {
            execute_mouseup_at(global, mouseup_args, frame).await
        }
        InteractCommand::Type(type_args) => execute_type(global, type_args, frame).await,
        InteractCommand::Key(key_args) => execute_key(global, key_args, frame).await,
        InteractCommand::Scroll(scroll_args) => execute_scroll(global, scroll_args, frame).await,
    }
}

// =============================================================================
// Script runner adapter
// =============================================================================

/// Run an `interact` command against an existing session and return a JSON value.
///
/// # Errors
///
/// Propagates `AppError` from the underlying interact logic.
pub async fn run_from_session(
    _managed: &mut ManagedSession,
    global: &GlobalOpts,
    args: &InteractArgs,
) -> Result<serde_json::Value, AppError> {
    execute_interact(global, args).await?;
    Ok(serde_json::json!({"executed": true}))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_uid_valid() {
        assert!(snapshot::is_uid("s1"));
        assert!(snapshot::is_uid("s42"));
        assert!(snapshot::is_uid("s999"));
    }

    #[test]
    fn is_uid_invalid() {
        assert!(!snapshot::is_uid("s"));
        assert!(!snapshot::is_uid("s0a"));
        assert!(!snapshot::is_uid("css:button"));
        assert!(!snapshot::is_uid("button"));
        assert!(!snapshot::is_uid("1s"));
    }

    #[test]
    fn is_css_selector_valid() {
        assert!(snapshot::is_css_selector("css:#button"));
        assert!(snapshot::is_css_selector("css:.class"));
        assert!(snapshot::is_css_selector("css:div > p"));
    }

    #[test]
    fn is_css_selector_invalid() {
        assert!(!snapshot::is_css_selector("#button"));
        assert!(!snapshot::is_css_selector("s1"));
        assert!(!snapshot::is_css_selector("button"));
    }

    #[test]
    fn click_result_serialization() {
        let result = ClickResult {
            clicked: "s1".to_string(),
            url: "https://example.com".to_string(),
            navigated: false,
            double_click: None,
            right_click: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["clicked"], "s1");
        assert_eq!(json["url"], "https://example.com");
        assert_eq!(json["navigated"], false);
        assert!(json.get("double_click").is_none());
        assert!(json.get("right_click").is_none());
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn click_result_serialization_with_double() {
        let result = ClickResult {
            clicked: "s1".to_string(),
            url: "https://example.com".to_string(),
            navigated: false,
            double_click: Some(true),
            right_click: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["double_click"], true);
        assert!(json.get("right_click").is_none());
    }

    #[test]
    fn click_result_serialization_with_right() {
        let result = ClickResult {
            clicked: "s1".to_string(),
            url: "https://example.com".to_string(),
            navigated: false,
            double_click: None,
            right_click: Some(true),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["right_click"], true);
        assert!(json.get("double_click").is_none());
    }

    #[test]
    fn click_at_result_serialization() {
        let result = ClickAtResult {
            clicked_at: Coords { x: 100.0, y: 200.0 },
            url: None,
            navigated: None,
            double_click: None,
            right_click: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["clicked_at"]["x"], 100.0);
        assert_eq!(json["clicked_at"]["y"], 200.0);
        assert!(json.get("double_click").is_none());
    }

    #[test]
    fn hover_result_serialization() {
        let result = HoverResult {
            hovered: "s3".to_string(),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["hovered"], "s3");
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn drag_result_serialization() {
        let result = DragResult {
            dragged: DragTargets {
                from: "s1".to_string(),
                to: "s2".to_string(),
            },
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["dragged"]["from"], "s1");
        assert_eq!(json["dragged"]["to"], "s2");
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn drag_at_result_serialization() {
        let result = DragAtResult {
            dragged_at: DragAtCoords {
                from: Coords { x: 100.0, y: 200.0 },
                to: Coords { x: 300.0, y: 400.0 },
            },
            steps: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["dragged_at"]["from"]["x"], 100.0);
        assert_eq!(json["dragged_at"]["from"]["y"], 200.0);
        assert_eq!(json["dragged_at"]["to"]["x"], 300.0);
        assert_eq!(json["dragged_at"]["to"]["y"], 400.0);
        assert!(json.get("steps").is_none());
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn drag_at_result_with_steps() {
        let result = DragAtResult {
            dragged_at: DragAtCoords {
                from: Coords { x: 0.0, y: 0.0 },
                to: Coords { x: 100.0, y: 100.0 },
            },
            steps: Some(5),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["steps"], 5);
    }

    #[test]
    fn mousedown_at_result_serialization() {
        let result = MouseDownAtResult {
            mousedown_at: Coords { x: 150.0, y: 250.0 },
            button: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["mousedown_at"]["x"], 150.0);
        assert_eq!(json["mousedown_at"]["y"], 250.0);
        assert!(json.get("button").is_none());
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn mousedown_at_result_with_button() {
        let result = MouseDownAtResult {
            mousedown_at: Coords { x: 100.0, y: 200.0 },
            button: Some("right".to_string()),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["button"], "right");
    }

    #[test]
    fn mouseup_at_result_serialization() {
        let result = MouseUpAtResult {
            mouseup_at: Coords { x: 300.0, y: 400.0 },
            button: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["mouseup_at"]["x"], 300.0);
        assert_eq!(json["mouseup_at"]["y"], 400.0);
        assert!(json.get("button").is_none());
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn mouseup_at_result_with_button() {
        let result = MouseUpAtResult {
            mouseup_at: Coords { x: 100.0, y: 200.0 },
            button: Some("middle".to_string()),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["button"], "middle");
    }

    #[test]
    fn mouse_button_to_cdp_left() {
        assert_eq!(mouse_button_to_cdp(None), "left");
        assert_eq!(mouse_button_to_cdp(Some(&MouseButton::Left)), "left");
    }

    #[test]
    fn mouse_button_to_cdp_right() {
        assert_eq!(mouse_button_to_cdp(Some(&MouseButton::Right)), "right");
    }

    #[test]
    fn mouse_button_to_cdp_middle() {
        assert_eq!(mouse_button_to_cdp(Some(&MouseButton::Middle)), "middle");
    }

    #[test]
    fn mouse_button_for_output_default() {
        assert!(mouse_button_for_output(None).is_none());
        assert!(mouse_button_for_output(Some(&MouseButton::Left)).is_none());
    }

    #[test]
    fn mouse_button_for_output_non_default() {
        assert_eq!(
            mouse_button_for_output(Some(&MouseButton::Right)),
            Some("right".to_string())
        );
        assert_eq!(
            mouse_button_for_output(Some(&MouseButton::Middle)),
            Some("middle".to_string())
        );
    }

    // =========================================================================
    // Key validation and parsing tests
    // =========================================================================

    #[test]
    fn parse_single_key() {
        let parsed = parse_key_combination("Enter").unwrap();
        assert_eq!(parsed.modifiers, 0);
        assert_eq!(parsed.key, "Enter");
    }

    #[test]
    fn parse_modifier_plus_key() {
        let parsed = parse_key_combination("Control+A").unwrap();
        assert_eq!(parsed.modifiers, 2); // Control = bit 1 = 2
        assert_eq!(parsed.key, "A");
    }

    #[test]
    fn parse_multiple_modifiers() {
        let parsed = parse_key_combination("Control+Shift+A").unwrap();
        assert_eq!(parsed.modifiers, 10); // Control(2) + Shift(8) = 10
        assert_eq!(parsed.key, "A");
    }

    #[test]
    fn parse_all_modifiers() {
        let parsed = parse_key_combination("Alt+Control+Meta+Shift+a").unwrap();
        assert_eq!(parsed.modifiers, 15); // 1 + 2 + 4 + 8 = 15
        assert_eq!(parsed.key, "a");
    }

    #[test]
    fn parse_invalid_key_error() {
        let err = parse_key_combination("FooBar").unwrap_err();
        assert!(
            err.message.contains("Invalid key: 'FooBar'"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn parse_duplicate_modifier_error() {
        let err = parse_key_combination("Control+Control+A").unwrap_err();
        assert!(
            err.message.contains("Duplicate modifier: 'Control'"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn parse_single_letter_key() {
        let parsed = parse_key_combination("a").unwrap();
        assert_eq!(parsed.modifiers, 0);
        assert_eq!(parsed.key, "a");
    }

    #[test]
    fn parse_function_key() {
        let parsed = parse_key_combination("F12").unwrap();
        assert_eq!(parsed.modifiers, 0);
        assert_eq!(parsed.key, "F12");
    }

    #[test]
    fn parse_shift_plus_arrow() {
        let parsed = parse_key_combination("Shift+ArrowDown").unwrap();
        assert_eq!(parsed.modifiers, 8); // Shift = 8
        assert_eq!(parsed.key, "ArrowDown");
    }

    #[test]
    fn is_valid_key_modifiers() {
        assert!(is_valid_key("Alt"));
        assert!(is_valid_key("Control"));
        assert!(is_valid_key("Meta"));
        assert!(is_valid_key("Shift"));
    }

    #[test]
    fn is_valid_key_common() {
        assert!(is_valid_key("Enter"));
        assert!(is_valid_key("Tab"));
        assert!(is_valid_key("Space"));
        assert!(is_valid_key("Backspace"));
        assert!(is_valid_key("a"));
        assert!(is_valid_key("Z"));
        assert!(is_valid_key("0"));
        assert!(is_valid_key("F1"));
        assert!(is_valid_key("F24"));
    }

    #[test]
    fn is_valid_key_invalid() {
        assert!(!is_valid_key("FooBar"));
        assert!(!is_valid_key(""));
        assert!(!is_valid_key("enter")); // case-sensitive
    }

    #[test]
    fn is_modifier_checks() {
        assert!(is_modifier("Alt"));
        assert!(is_modifier("Control"));
        assert!(is_modifier("Meta"));
        assert!(is_modifier("Shift"));
        assert!(!is_modifier("Enter"));
        assert!(!is_modifier("a"));
    }

    // =========================================================================
    // CDP key mapping tests
    // =========================================================================

    #[test]
    fn cdp_key_value_special_keys() {
        assert_eq!(cdp_key_value("Enter"), "Enter");
        assert_eq!(cdp_key_value("Tab"), "Tab");
        assert_eq!(cdp_key_value("Space"), " ");
        assert_eq!(cdp_key_value("Escape"), "Escape");
        assert_eq!(cdp_key_value("Backspace"), "Backspace");
        assert_eq!(cdp_key_value("Delete"), "Delete");
    }

    #[test]
    fn cdp_key_value_single_chars() {
        assert_eq!(cdp_key_value("a"), "a");
        assert_eq!(cdp_key_value("Z"), "Z");
        assert_eq!(cdp_key_value("5"), "5");
    }

    #[test]
    fn cdp_key_value_symbols() {
        assert_eq!(cdp_key_value("Minus"), "-");
        assert_eq!(cdp_key_value("Equal"), "=");
        assert_eq!(cdp_key_value("Comma"), ",");
        assert_eq!(cdp_key_value("Period"), ".");
        assert_eq!(cdp_key_value("Slash"), "/");
        assert_eq!(cdp_key_value("Semicolon"), ";");
        assert_eq!(cdp_key_value("Quote"), "'");
        assert_eq!(cdp_key_value("Backquote"), "`");
        assert_eq!(cdp_key_value("BracketLeft"), "[");
        assert_eq!(cdp_key_value("BracketRight"), "]");
        assert_eq!(cdp_key_value("Backslash"), "\\");
    }

    #[test]
    fn cdp_key_value_modifiers() {
        assert_eq!(cdp_key_value("Alt"), "Alt");
        assert_eq!(cdp_key_value("Control"), "Control");
        assert_eq!(cdp_key_value("Meta"), "Meta");
        assert_eq!(cdp_key_value("Shift"), "Shift");
    }

    #[test]
    fn cdp_key_value_function_keys() {
        assert_eq!(cdp_key_value("F1"), "F1");
        assert_eq!(cdp_key_value("F12"), "F12");
        assert_eq!(cdp_key_value("F24"), "F24");
    }

    #[test]
    fn cdp_key_value_navigation() {
        assert_eq!(cdp_key_value("ArrowUp"), "ArrowUp");
        assert_eq!(cdp_key_value("ArrowDown"), "ArrowDown");
        assert_eq!(cdp_key_value("Home"), "Home");
        assert_eq!(cdp_key_value("End"), "End");
        assert_eq!(cdp_key_value("PageUp"), "PageUp");
        assert_eq!(cdp_key_value("PageDown"), "PageDown");
    }

    #[test]
    fn cdp_key_code_letters() {
        assert_eq!(cdp_key_code("a"), "KeyA");
        assert_eq!(cdp_key_code("z"), "KeyZ");
        assert_eq!(cdp_key_code("A"), "KeyA");
        assert_eq!(cdp_key_code("Z"), "KeyZ");
    }

    #[test]
    fn cdp_key_code_digits() {
        assert_eq!(cdp_key_code("0"), "Digit0");
        assert_eq!(cdp_key_code("5"), "Digit5");
        assert_eq!(cdp_key_code("9"), "Digit9");
    }

    #[test]
    fn cdp_key_code_modifiers() {
        assert_eq!(cdp_key_code("Alt"), "AltLeft");
        assert_eq!(cdp_key_code("Control"), "ControlLeft");
        assert_eq!(cdp_key_code("Meta"), "MetaLeft");
        assert_eq!(cdp_key_code("Shift"), "ShiftLeft");
    }

    #[test]
    fn cdp_key_code_special() {
        assert_eq!(cdp_key_code("Enter"), "Enter");
        assert_eq!(cdp_key_code("Tab"), "Tab");
        assert_eq!(cdp_key_code("Space"), "Space");
        assert_eq!(cdp_key_code("Backspace"), "Backspace");
        assert_eq!(cdp_key_code("Escape"), "Escape");
    }

    #[test]
    fn cdp_key_code_function_keys() {
        assert_eq!(cdp_key_code("F1"), "F1");
        assert_eq!(cdp_key_code("F12"), "F12");
    }

    #[test]
    fn cdp_key_code_symbols() {
        assert_eq!(cdp_key_code("Minus"), "Minus");
        assert_eq!(cdp_key_code("Comma"), "Comma");
        assert_eq!(cdp_key_code("Period"), "Period");
        assert_eq!(cdp_key_code("Slash"), "Slash");
    }

    // =========================================================================
    // windowsVirtualKeyCode tests (issue #227)
    // =========================================================================

    #[test]
    fn vk_letters() {
        assert_eq!(windows_virtual_key_code("a"), 65);
        assert_eq!(windows_virtual_key_code("A"), 65);
        assert_eq!(windows_virtual_key_code("z"), 90);
        assert_eq!(windows_virtual_key_code("Z"), 90);
    }

    #[test]
    fn vk_digits() {
        assert_eq!(windows_virtual_key_code("0"), 48);
        assert_eq!(windows_virtual_key_code("5"), 53);
        assert_eq!(windows_virtual_key_code("9"), 57);
    }

    #[test]
    fn vk_special() {
        assert_eq!(windows_virtual_key_code("Enter"), 13);
        assert_eq!(windows_virtual_key_code("Tab"), 9);
        assert_eq!(windows_virtual_key_code("Escape"), 27);
        assert_eq!(windows_virtual_key_code("Backspace"), 8);
        assert_eq!(windows_virtual_key_code("Space"), 32);
        assert_eq!(windows_virtual_key_code("Delete"), 46);
        assert_eq!(windows_virtual_key_code("Insert"), 45);
    }

    #[test]
    fn vk_navigation() {
        assert_eq!(windows_virtual_key_code("ArrowLeft"), 37);
        assert_eq!(windows_virtual_key_code("ArrowUp"), 38);
        assert_eq!(windows_virtual_key_code("ArrowRight"), 39);
        assert_eq!(windows_virtual_key_code("ArrowDown"), 40);
        assert_eq!(windows_virtual_key_code("Home"), 36);
        assert_eq!(windows_virtual_key_code("End"), 35);
        assert_eq!(windows_virtual_key_code("PageUp"), 33);
        assert_eq!(windows_virtual_key_code("PageDown"), 34);
    }

    #[test]
    fn vk_modifiers() {
        assert_eq!(windows_virtual_key_code("Shift"), 16);
        assert_eq!(windows_virtual_key_code("Control"), 17);
        assert_eq!(windows_virtual_key_code("Alt"), 18);
        assert_eq!(windows_virtual_key_code("Meta"), 91);
    }

    #[test]
    fn vk_function_keys() {
        assert_eq!(windows_virtual_key_code("F1"), 112);
        assert_eq!(windows_virtual_key_code("F12"), 123);
        assert_eq!(windows_virtual_key_code("F24"), 135);
    }

    #[test]
    fn vk_unknown_returns_zero() {
        assert_eq!(windows_virtual_key_code("FooBar"), 0);
        assert_eq!(windows_virtual_key_code(""), 0);
    }

    // =========================================================================
    // key_text tests (issue #227)
    // =========================================================================

    #[test]
    fn key_text_letters_no_shift() {
        assert_eq!(key_text("a", 0), Some("a".to_string()));
        assert_eq!(key_text("A", 0), Some("A".to_string()));
    }

    #[test]
    fn key_text_letters_with_shift() {
        // Shift + lowercase -> uppercase
        assert_eq!(key_text("a", 8), Some("A".to_string()));
        // Shift + uppercase stays uppercase
        assert_eq!(key_text("A", 8), Some("A".to_string()));
    }

    #[test]
    fn key_text_digits() {
        assert_eq!(key_text("5", 0), Some("5".to_string()));
        assert_eq!(key_text("5", 8), Some("%".to_string()));
        assert_eq!(key_text("1", 8), Some("!".to_string()));
    }

    #[test]
    fn key_text_printable_named_keys() {
        assert_eq!(key_text("Enter", 0), Some("\r".to_string()));
        assert_eq!(key_text("Tab", 0), Some("\t".to_string()));
        assert_eq!(key_text("Space", 0), Some(" ".to_string()));
    }

    #[test]
    fn key_text_symbols() {
        assert_eq!(key_text("Minus", 0), Some("-".to_string()));
        assert_eq!(key_text("Minus", 8), Some("_".to_string()));
        assert_eq!(key_text("Slash", 0), Some("/".to_string()));
        assert_eq!(key_text("Slash", 8), Some("?".to_string()));
    }

    #[test]
    fn key_text_non_printable_returns_none() {
        assert!(key_text("Escape", 0).is_none());
        assert!(key_text("Backspace", 0).is_none());
        assert!(key_text("ArrowUp", 0).is_none());
        assert!(key_text("PageDown", 0).is_none());
        assert!(key_text("F1", 0).is_none());
        assert!(key_text("F12", 0).is_none());
        assert!(key_text("Shift", 8).is_none());
        assert!(key_text("Control", 2).is_none());
    }

    // =========================================================================
    // TypeResult and KeyResult serialization tests
    // =========================================================================

    #[test]
    fn type_result_serialization() {
        let result = TypeResult {
            typed: "Hello".to_string(),
            length: 5,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["typed"], "Hello");
        assert_eq!(json["length"], 5);
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn type_result_with_snapshot() {
        let result = TypeResult {
            typed: "test".to_string(),
            length: 4,
            snapshot: Some(serde_json::json!({"role": "document"})),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["typed"], "test");
        assert_eq!(json["length"], 4);
        assert!(json.get("snapshot").is_some());
    }

    #[test]
    fn key_result_serialization_single_press() {
        let result = KeyResult {
            pressed: "Enter".to_string(),
            repeat: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["pressed"], "Enter");
        assert!(json.get("repeat").is_none());
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn key_result_serialization_with_repeat() {
        let result = KeyResult {
            pressed: "ArrowDown".to_string(),
            repeat: Some(5),
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["pressed"], "ArrowDown");
        assert_eq!(json["repeat"], 5);
    }

    #[test]
    fn key_result_serialization_with_snapshot() {
        let result = KeyResult {
            pressed: "Tab".to_string(),
            repeat: None,
            snapshot: Some(serde_json::json!({"role": "document"})),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["pressed"], "Tab");
        assert!(json.get("snapshot").is_some());
        assert!(json.get("repeat").is_none());
    }

    #[test]
    fn key_result_combination() {
        let result = KeyResult {
            pressed: "Control+A".to_string(),
            repeat: None,
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["pressed"], "Control+A");
    }

    // =========================================================================
    // ScrollResult serialization tests
    // =========================================================================

    #[test]
    fn scroll_result_serialization() {
        let result = ScrollResult {
            scrolled: Coords { x: 0.0, y: 600.0 },
            position: Coords { x: 0.0, y: 600.0 },
            snapshot: None,
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["scrolled"]["x"], 0.0);
        assert_eq!(json["scrolled"]["y"], 600.0);
        assert_eq!(json["position"]["x"], 0.0);
        assert_eq!(json["position"]["y"], 600.0);
        assert!(json.get("snapshot").is_none());
    }

    #[test]
    fn scroll_result_with_snapshot() {
        let result = ScrollResult {
            scrolled: Coords { x: 0.0, y: 300.0 },
            position: Coords { x: 0.0, y: 300.0 },
            snapshot: Some(serde_json::json!({"role": "document"})),
        };
        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["scrolled"]["y"], 300.0);
        assert_eq!(json["position"]["y"], 300.0);
        assert!(json.get("snapshot").is_some());
        assert_eq!(json["snapshot"]["role"], "document");
    }

    #[test]
    fn scroll_result_without_snapshot_omits_field() {
        let result = ScrollResult {
            scrolled: Coords { x: 200.0, y: 0.0 },
            position: Coords { x: 200.0, y: 100.0 },
            snapshot: None,
        };
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(!json_str.contains("snapshot"));
    }

    // =========================================================================
    // Scroll delta computation tests
    // =========================================================================

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_delta_returns_correct_values() {
        let (dx, dy, px, py) = compute_delta((10.0, 20.0), (30.0, 50.0));
        assert_eq!(dx, 20.0);
        assert_eq!(dy, 30.0);
        assert_eq!(px, 30.0);
        assert_eq!(py, 50.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_delta_negative_scroll() {
        let (dx, dy, px, py) = compute_delta((100.0, 200.0), (50.0, 100.0));
        assert_eq!(dx, -50.0);
        assert_eq!(dy, -100.0);
        assert_eq!(px, 50.0);
        assert_eq!(py, 100.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_delta_no_movement() {
        let (dx, dy, px, py) = compute_delta((0.0, 0.0), (0.0, 0.0));
        assert_eq!(dx, 0.0);
        assert_eq!(dy, 0.0);
        assert_eq!(px, 0.0);
        assert_eq!(py, 0.0);
    }

    // =========================================================================
    // Scroll direction delta computation tests
    // =========================================================================

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_scroll_delta_down_default() {
        let (dx, dy) = compute_scroll_delta(ScrollDirection::Down, None, 1024.0, 768.0);
        assert_eq!(dx, 0.0);
        assert_eq!(dy, 768.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_scroll_delta_up_default() {
        let (dx, dy) = compute_scroll_delta(ScrollDirection::Up, None, 1024.0, 768.0);
        assert_eq!(dx, 0.0);
        assert_eq!(dy, -768.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_scroll_delta_right_default() {
        let (dx, dy) = compute_scroll_delta(ScrollDirection::Right, None, 1024.0, 768.0);
        assert_eq!(dx, 1024.0);
        assert_eq!(dy, 0.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_scroll_delta_left_default() {
        let (dx, dy) = compute_scroll_delta(ScrollDirection::Left, None, 1024.0, 768.0);
        assert_eq!(dx, -1024.0);
        assert_eq!(dy, 0.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_scroll_delta_down_with_amount() {
        let (dx, dy) = compute_scroll_delta(ScrollDirection::Down, Some(300), 1024.0, 768.0);
        assert_eq!(dx, 0.0);
        assert_eq!(dy, 300.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn compute_scroll_delta_right_with_amount() {
        let (dx, dy) = compute_scroll_delta(ScrollDirection::Right, Some(200), 1024.0, 768.0);
        assert_eq!(dx, 200.0);
        assert_eq!(dy, 0.0);
    }
}
