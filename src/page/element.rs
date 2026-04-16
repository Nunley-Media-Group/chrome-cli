use serde::Serialize;

use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, PageElementArgs};

use super::{get_viewport_dimensions, print_output, setup_session};

// =============================================================================
// Output types
// =============================================================================

/// Output for `page element`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ElementInfo {
    role: String,
    name: String,
    tag_name: String,
    bounding_box: ElementBoundingBox,
    properties: ElementProperties,
    in_viewport: bool,
}

#[derive(Serialize)]
struct ElementBoundingBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Serialize)]
#[allow(clippy::struct_excessive_bools)]
struct ElementProperties {
    enabled: bool,
    focused: bool,
    checked: Option<bool>,
    expanded: Option<bool>,
    required: bool,
    readonly: bool,
}

// =============================================================================
// Target resolution
// =============================================================================

/// Check if a target string is a UID (matches pattern: 's' + digits).
fn is_element_uid(target: &str) -> bool {
    if !target.starts_with('s') {
        return false;
    }
    let rest = &target[1..];
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// Check if a target string is a CSS selector (starts with 'css:').
fn is_element_css_selector(target: &str) -> bool {
    target.starts_with("css:")
}

/// Resolve a target (UID or CSS selector) to a backend DOM node ID.
async fn resolve_element_target(session: &ManagedSession, target: &str) -> Result<i64, AppError> {
    if is_element_uid(target) {
        let state = crate::snapshot::read_snapshot_state()
            .map_err(|e| AppError {
                message: format!("Failed to read snapshot state: {e}"),
                code: ExitCode::GeneralError,
                custom_json: None,
            })?
            .ok_or_else(AppError::no_snapshot_state)?;
        let backend_node_id = state
            .uid_map
            .get(target)
            .copied()
            .ok_or_else(|| AppError::element_target_not_found(target))?;
        Ok(backend_node_id)
    } else if is_element_css_selector(target) {
        let selector = &target[4..];

        let doc_response = session.send_command("DOM.getDocument", None).await?;
        let root_node_id = doc_response["root"]["nodeId"]
            .as_i64()
            .ok_or_else(|| AppError::css_selector_not_found(selector))?;

        let query_params = serde_json::json!({
            "nodeId": root_node_id,
            "selector": selector,
        });
        let query_response = session
            .send_command("DOM.querySelector", Some(query_params))
            .await?;

        let node_id = query_response["nodeId"].as_i64().unwrap_or(0);
        if node_id == 0 {
            return Err(AppError::css_selector_not_found(selector));
        }

        let describe_params = serde_json::json!({ "nodeId": node_id });
        let describe_response = session
            .send_command("DOM.describeNode", Some(describe_params))
            .await?;

        let backend_node_id = describe_response["node"]["backendNodeId"]
            .as_i64()
            .ok_or_else(|| AppError::css_selector_not_found(selector))?;

        Ok(backend_node_id)
    } else {
        Err(AppError::element_target_not_found(target))
    }
}

// =============================================================================
// Accessibility property extraction
// =============================================================================

/// Extract an AX property boolean value by name from the properties array.
fn get_ax_bool_property(properties: &[serde_json::Value], name: &str) -> Option<bool> {
    properties.iter().find_map(|p| {
        if p["name"].as_str() == Some(name) {
            p["value"]["value"].as_bool()
        } else {
            None
        }
    })
}

/// Format an `Option<bool>` as plain text: "yes", "no", or "n/a".
fn format_opt_bool(val: Option<bool>) -> &'static str {
    match val {
        Some(true) => "yes",
        Some(false) => "no",
        None => "n/a",
    }
}

// =============================================================================
// Element info retrieval
// =============================================================================

/// Retrieve element info from CDP given a `backendNodeId`.
async fn fetch_element_info(
    managed: &ManagedSession,
    backend_node_id: i64,
    target: &str,
) -> Result<ElementInfo, AppError> {
    // Get accessibility data
    let ax_params = serde_json::json!({
        "backendNodeId": backend_node_id,
        "fetchRelatives": false,
    });
    let ax_response = managed
        .send_command("Accessibility.getPartialAXTree", Some(ax_params))
        .await
        .map_err(|e| AppError::element_target_not_found(&format!("{target} ({e})")))?;

    let nodes = ax_response["nodes"].as_array();
    let ax_node = nodes.and_then(|n| n.first());

    let role = ax_node
        .and_then(|n| n["role"]["value"].as_str())
        .unwrap_or("none")
        .to_string();
    let name = ax_node
        .and_then(|n| n["name"]["value"].as_str())
        .unwrap_or("")
        .to_string();

    let props = ax_node
        .and_then(|n| n["properties"].as_array())
        .map_or(&[][..], Vec::as_slice);

    let disabled = get_ax_bool_property(props, "disabled");
    let enabled = !disabled.unwrap_or(false);
    let focused = get_ax_bool_property(props, "focused").unwrap_or(false);
    let checked = get_ax_bool_property(props, "checked");
    let expanded = get_ax_bool_property(props, "expanded");
    let required = get_ax_bool_property(props, "required").unwrap_or(false);
    let readonly = get_ax_bool_property(props, "readonly").unwrap_or(false);

    // Get bounding box via DOM.getBoxModel with backendNodeId
    let box_params = serde_json::json!({ "backendNodeId": backend_node_id });
    let box_result = managed
        .send_command("DOM.getBoxModel", Some(box_params))
        .await;

    let (bx, by, bw, bh) = match box_result {
        Ok(ref val) => {
            let content = val["model"]["content"].as_array();
            match content {
                Some(c) if c.len() >= 8 => {
                    let x1 = c[0].as_f64().unwrap_or(0.0);
                    let y1 = c[1].as_f64().unwrap_or(0.0);
                    let x3 = c[4].as_f64().unwrap_or(0.0);
                    let y3 = c[5].as_f64().unwrap_or(0.0);
                    (x1, y1, x3 - x1, y3 - y1)
                }
                _ => (0.0, 0.0, 0.0, 0.0),
            }
        }
        Err(_) => (0.0, 0.0, 0.0, 0.0),
    };

    // Get tag name via DOM.describeNode
    let describe_params = serde_json::json!({ "backendNodeId": backend_node_id });
    let describe_response = managed
        .send_command("DOM.describeNode", Some(describe_params))
        .await
        .map_err(|e| AppError::element_target_not_found(&format!("{target} ({e})")))?;

    let tag_name = describe_response["node"]["nodeName"]
        .as_str()
        .unwrap_or("UNKNOWN")
        .to_string();

    // Get viewport dimensions for inViewport computation
    let (vp_width, vp_height) = get_viewport_dimensions(managed).await?;
    let vw = f64::from(vp_width);
    let vh = f64::from(vp_height);

    let zero_bbox = bw == 0.0 && bh == 0.0;
    let in_viewport = !zero_bbox && (bx + bw > 0.0) && (bx < vw) && (by + bh > 0.0) && (by < vh);

    Ok(ElementInfo {
        role,
        name,
        tag_name,
        bounding_box: ElementBoundingBox {
            x: bx,
            y: by,
            width: bw,
            height: bh,
        },
        properties: ElementProperties {
            enabled,
            focused,
            checked,
            expanded,
            required,
            readonly,
        },
        in_viewport,
    })
}

// =============================================================================
// Plain text output
// =============================================================================

/// Print element info as human-readable plain text.
fn print_element_plain(info: &ElementInfo) {
    let yn = |b: bool| if b { "yes" } else { "no" };
    println!("Role:       {}", info.role);
    println!("Name:       {}", info.name);
    println!("Tag:        {}", info.tag_name);
    println!(
        "Bounds:     {}, {}, {}x{}",
        info.bounding_box.x, info.bounding_box.y, info.bounding_box.width, info.bounding_box.height
    );
    println!("In Viewport: {}", yn(info.in_viewport));
    println!("Enabled:    {}", yn(info.properties.enabled));
    println!("Focused:    {}", yn(info.properties.focused));
    println!("Checked:    {}", format_opt_bool(info.properties.checked));
    println!("Expanded:   {}", format_opt_bool(info.properties.expanded));
    println!("Required:   {}", yn(info.properties.required));
    println!("Read-only:  {}", yn(info.properties.readonly));
}

// =============================================================================
// Command executor
// =============================================================================

pub async fn execute_element(
    global: &GlobalOpts,
    args: &PageElementArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session(global).await?;

    // Resolve optional frame context
    let mut frame_ctx = if let Some(frame_str) = frame {
        let arg = agentchrome::frame::parse_frame_arg(frame_str)?;
        Some(agentchrome::frame::resolve_frame(&client, &mut managed, &arg).await?)
    } else {
        None
    };

    // Enable domains on effective session (needs &mut)
    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("DOM").await?;
        eff_mut.ensure_domain("Accessibility").await?;
    }

    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    let backend_node_id = resolve_element_target(effective, &args.target).await?;
    let element_info = fetch_element_info(effective, backend_node_id, &args.target).await?;

    if global.output.plain {
        print_element_plain(&element_info);
    } else {
        print_output(&element_info, &global.output)?;
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
    fn is_element_uid_valid() {
        assert!(is_element_uid("s1"));
        assert!(is_element_uid("s10"));
        assert!(is_element_uid("s999"));
        assert!(is_element_uid("s0"));
    }

    #[test]
    fn is_element_uid_invalid() {
        assert!(!is_element_uid(""));
        assert!(!is_element_uid("s")); // digits required
        assert!(!is_element_uid("S1")); // uppercase S
        assert!(!is_element_uid("1s")); // wrong prefix
        assert!(!is_element_uid("css:#id"));
        assert!(!is_element_uid("s1a")); // non-digit suffix
    }

    #[test]
    fn is_element_css_selector_valid() {
        assert!(is_element_css_selector("css:#id"));
        assert!(is_element_css_selector("css:.class"));
        assert!(is_element_css_selector("css:button"));
        assert!(is_element_css_selector("css:"));
    }

    #[test]
    fn is_element_css_selector_invalid() {
        assert!(!is_element_css_selector(""));
        assert!(!is_element_css_selector("s1"));
        assert!(!is_element_css_selector("#id"));
        assert!(!is_element_css_selector("CSS:#id")); // case sensitive
    }

    #[test]
    fn format_opt_bool_values() {
        assert_eq!(format_opt_bool(Some(true)), "yes");
        assert_eq!(format_opt_bool(Some(false)), "no");
        assert_eq!(format_opt_bool(None), "n/a");
    }

    #[test]
    fn get_ax_bool_property_found_true() {
        let props = serde_json::json!([
            {"name": "disabled", "value": {"value": true}},
            {"name": "focused", "value": {"value": false}},
        ]);
        let arr = props.as_array().unwrap();
        assert_eq!(get_ax_bool_property(arr, "disabled"), Some(true));
        assert_eq!(get_ax_bool_property(arr, "focused"), Some(false));
    }

    #[test]
    fn get_ax_bool_property_not_found() {
        let props = serde_json::json!([
            {"name": "disabled", "value": {"value": true}},
        ]);
        let arr = props.as_array().unwrap();
        assert_eq!(get_ax_bool_property(arr, "checked"), None);
    }

    #[test]
    fn get_ax_bool_property_empty_array() {
        let arr: Vec<serde_json::Value> = vec![];
        assert_eq!(get_ax_bool_property(&arr, "focused"), None);
    }

    #[test]
    fn element_info_serialization_camel_case() {
        let info = ElementInfo {
            role: "button".to_string(),
            name: "Submit".to_string(),
            tag_name: "INPUT".to_string(),
            bounding_box: ElementBoundingBox {
                x: 100.0,
                y: 200.0,
                width: 150.0,
                height: 40.0,
            },
            properties: ElementProperties {
                enabled: true,
                focused: false,
                checked: None,
                expanded: None,
                required: false,
                readonly: false,
            },
            in_viewport: true,
        };
        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert_eq!(json["role"], "button");
        assert_eq!(json["name"], "Submit");
        assert_eq!(json["tagName"], "INPUT");
        assert_eq!(json["boundingBox"]["x"], 100.0);
        assert_eq!(json["boundingBox"]["y"], 200.0);
        assert_eq!(json["boundingBox"]["width"], 150.0);
        assert_eq!(json["boundingBox"]["height"], 40.0);
        assert_eq!(json["properties"]["enabled"], true);
        assert_eq!(json["properties"]["focused"], false);
        assert!(json["properties"]["checked"].is_null());
        assert!(json["properties"]["expanded"].is_null());
        assert_eq!(json["properties"]["required"], false);
        assert_eq!(json["properties"]["readonly"], false);
        assert_eq!(json["inViewport"], true);
        // verify snake_case keys are NOT present
        assert!(json.get("tag_name").is_none());
        assert!(json.get("bounding_box").is_none());
        assert!(json.get("in_viewport").is_none());
    }

    #[test]
    fn element_info_serialization_with_optional_bools() {
        let info = ElementInfo {
            role: "checkbox".to_string(),
            name: "Accept".to_string(),
            tag_name: "INPUT".to_string(),
            bounding_box: ElementBoundingBox {
                x: 0.0,
                y: 0.0,
                width: 16.0,
                height: 16.0,
            },
            properties: ElementProperties {
                enabled: true,
                focused: false,
                checked: Some(true),
                expanded: Some(false),
                required: true,
                readonly: false,
            },
            in_viewport: false,
        };
        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        assert_eq!(json["properties"]["checked"], true);
        assert_eq!(json["properties"]["expanded"], false);
        assert_eq!(json["inViewport"], false);
    }
}
