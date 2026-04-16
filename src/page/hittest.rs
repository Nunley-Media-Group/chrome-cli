use serde::Serialize;

use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, PageHitTestArgs};

use super::{get_viewport_dimensions, print_output, setup_session};

// =============================================================================
// Output types
// =============================================================================

/// Element info for hit target and intercepted-by fields.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementInfo {
    pub tag: String,
    pub id: Option<String>,
    pub class: Option<String>,
    pub uid: Option<String>,
}

/// Element in the z-index stack.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackElement {
    pub tag: String,
    pub id: Option<String>,
    pub class: Option<String>,
    pub uid: Option<String>,
    pub z_index: String,
}

/// Full output for `page hittest`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitTestResult {
    pub frame: String,
    pub hit_target: ElementInfo,
    pub intercepted_by: Option<ElementInfo>,
    pub stack: Vec<StackElement>,
    pub suggestion: Option<String>,
}

// =============================================================================
// UID lookup helper
// =============================================================================

/// Try to resolve a `backendNodeId` to a UID from a pre-loaded snapshot state.
/// Returns `None` if no state is provided or the node is not in the map.
fn lookup_uid(
    snapshot: Option<&crate::snapshot::SnapshotState>,
    backend_node_id: i64,
) -> Option<String> {
    snapshot?
        .uid_map
        .iter()
        .find(|&(_, &v)| v == backend_node_id)
        .map(|(k, _)| k.clone())
}

// =============================================================================
// Element description helpers
// =============================================================================

/// Build a CSS-like selector string for an element (used in suggestions).
fn element_selector(info: &ElementInfo) -> String {
    let mut sel = info.tag.clone();
    if let Some(ref id) = info.id {
        sel.push('#');
        sel.push_str(id);
    } else if let Some(ref class) = info.class {
        if let Some(first) = class.split_whitespace().next() {
            sel.push('.');
            sel.push_str(first);
        }
    }
    sel
}

// =============================================================================
// Overlay detection
// =============================================================================

/// Interactive HTML tags — elements users expect to click on.
const INTERACTIVE_TAGS: &[&str] = &["a", "button", "input", "select", "textarea", "label"];

/// Detect an overlay by checking whether the topmost stack element is
/// non-interactive while an interactive element exists deeper in the stack.
/// Also catches the case where the CDP hit target differs from stack\[0\].
fn detect_overlay(
    hit_backend_id: i64,
    stack: &[StackElement],
    stack_backend_ids: &[i64],
) -> Option<ElementInfo> {
    if stack.is_empty() || stack_backend_ids.is_empty() {
        return None;
    }

    let topmost = &stack[0];
    let topmost_id = stack_backend_ids[0];

    // Case 1: CDP hit target differs from the topmost stack element — clear overlay.
    // Skip comparison when topmost_id is -1 (unresolved via CSS selector).
    if topmost_id >= 0 && topmost_id != hit_backend_id {
        return Some(ElementInfo {
            tag: topmost.tag.clone(),
            id: topmost.id.clone(),
            class: topmost.class.clone(),
            uid: topmost.uid.clone(),
        });
    }

    // Case 2: The topmost element is non-interactive but an interactive element
    // exists deeper in the stack — the topmost element is intercepting clicks.
    let topmost_is_interactive =
        INTERACTIVE_TAGS.contains(&topmost.tag.as_str()) || topmost.uid.is_some();

    if !topmost_is_interactive {
        let has_interactive_below = stack
            .iter()
            .skip(1)
            .any(|e| INTERACTIVE_TAGS.contains(&e.tag.as_str()) || e.uid.is_some());
        if has_interactive_below {
            return Some(ElementInfo {
                tag: topmost.tag.clone(),
                id: topmost.id.clone(),
                class: topmost.class.clone(),
                uid: topmost.uid.clone(),
            });
        }
    }

    None
}

/// Generate a workaround suggestion when an overlay is detected.
fn generate_suggestion(
    overlay: &ElementInfo,
    hit_target: &ElementInfo,
    stack: &[StackElement],
) -> String {
    let overlay_sel = element_selector(overlay);

    // Try to find the first element in the stack that is likely the intended target
    // (the first element after the overlay that has an interactive tag or a UID).
    let intended = stack.iter().skip(1).find(|e| {
        e.uid.is_some()
            || matches!(
                e.tag.as_str(),
                "a" | "button" | "input" | "select" | "textarea" | "label"
            )
    });

    if let Some(target) = intended {
        let target_sel = element_selector(&ElementInfo {
            tag: target.tag.clone(),
            id: target.id.clone(),
            class: target.class.clone(),
            uid: target.uid.clone(),
        });
        if let Some(ref uid) = target.uid {
            format!(
                "Element intercepted by {overlay_sel} \u{2014} try targeting the underlying {target_sel} (uid: {uid}) directly"
            )
        } else {
            format!(
                "Element intercepted by {overlay_sel} \u{2014} try targeting the underlying {target_sel} directly"
            )
        }
    } else {
        let target_sel = element_selector(hit_target);
        format!(
            "Element intercepted by {overlay_sel} \u{2014} try targeting {target_sel} via CSS selector or use --frame to bypass"
        )
    }
}

// =============================================================================
// Command executor
// =============================================================================

#[allow(clippy::too_many_lines)]
pub async fn execute_hittest(
    global: &GlobalOpts,
    args: &PageHitTestArgs,
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

    // Enable required domains
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

    // Determine frame label for output
    let frame_label = frame.map_or_else(|| "main".to_string(), ToString::to_string);

    // Viewport bounds check
    let (vp_width, vp_height) = get_viewport_dimensions(effective).await?;
    if args.x >= vp_width || args.y >= vp_height {
        return Err(AppError {
            message: format!(
                "Coordinates ({}, {}) are outside the viewport bounds ({vp_width}x{vp_height})",
                args.x, args.y
            ),
            code: ExitCode::TargetError,
            custom_json: None,
        });
    }

    // Get document root (required for DOM.getNodeForLocation and stack resolution)
    let doc_result = effective
        .send_command("DOM.getDocument", None)
        .await
        .map_err(|e| AppError {
            message: format!("Failed to get document: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;
    let root_node_id = doc_result["root"]["nodeId"].as_i64().unwrap_or(1);

    // CDP hit test: DOM.getNodeForLocation
    let hit_params = serde_json::json!({
        "x": args.x,
        "y": args.y,
        "includeUserAgentShadowDOM": false,
    });
    let hit_result = effective
        .send_command("DOM.getNodeForLocation", Some(hit_params))
        .await
        .map_err(|e| AppError {
            message: format!("DOM.getNodeForLocation failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let hit_backend_id = hit_result["backendNodeId"]
        .as_i64()
        .ok_or_else(|| AppError {
            message: "DOM.getNodeForLocation did not return a backendNodeId".to_string(),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    // Describe the hit target
    let describe_params = serde_json::json!({ "backendNodeId": hit_backend_id });
    let describe_result = effective
        .send_command("DOM.describeNode", Some(describe_params))
        .await
        .map_err(|e| AppError {
            message: format!("DOM.describeNode failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let hit_tag = describe_result["node"]["localName"]
        .as_str()
        .or_else(|| describe_result["node"]["nodeName"].as_str())
        .unwrap_or("unknown")
        .to_lowercase();

    let hit_attrs = extract_attributes(&describe_result["node"]);

    // Load snapshot state once for all UID lookups
    let snapshot_state = crate::snapshot::read_snapshot_state().ok().flatten();
    let hit_uid = lookup_uid(snapshot_state.as_ref(), hit_backend_id);

    let hit_target = ElementInfo {
        tag: hit_tag,
        id: hit_attrs.0,
        class: hit_attrs.1,
        uid: hit_uid,
    };

    // Z-index stack enumeration via Runtime.evaluate
    let context_id = frame_ctx
        .as_ref()
        .and_then(agentchrome::frame::execution_context_id);

    let js_code = format!(
        r"(function() {{
            var elems = document.elementsFromPoint({x}, {y});
            return JSON.stringify(elems.map(function(el) {{
                var style = window.getComputedStyle(el);
                return {{
                    tag: el.tagName.toLowerCase(),
                    id: el.id || null,
                    class: el.className || null,
                    zIndex: style.zIndex || 'auto',
                    backendNodeId: null
                }};
            }}));
        }})()",
        x = args.x,
        y = args.y
    );

    let mut eval_params = serde_json::json!({
        "expression": js_code,
        "returnByValue": true,
    });
    if let Some(ctx_id) = context_id {
        eval_params["contextId"] = serde_json::json!(ctx_id);
    }

    let eval_result = effective
        .send_command("Runtime.evaluate", Some(eval_params))
        .await
        .map_err(|e| AppError {
            message: format!("Runtime.evaluate failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let stack_json_str = eval_result["result"]["value"].as_str().unwrap_or("[]");

    let raw_stack: Vec<serde_json::Value> =
        serde_json::from_str(stack_json_str).unwrap_or_default();

    // Resolve backend node IDs for stack elements via DOM.querySelector-like approach
    // We use DOM.getNodeForLocation result as the primary, and for the rest we look up by resolving
    let mut stack: Vec<StackElement> = Vec::new();
    let mut stack_backend_ids: Vec<i64> = Vec::new();

    for raw_elem in &raw_stack {
        let tag = raw_elem["tag"].as_str().unwrap_or("unknown").to_string();
        let id = raw_elem["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);
        let class = raw_elem["class"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);
        let z_index = raw_elem["zIndex"].as_str().unwrap_or("auto").to_string();

        // Try to resolve backend node ID for UID lookup
        let backend_id = resolve_stack_element_backend_id(
            effective,
            root_node_id,
            &tag,
            id.as_deref(),
            class.as_deref(),
        )
        .await;
        let uid = backend_id.and_then(|bid| lookup_uid(snapshot_state.as_ref(), bid));

        stack_backend_ids.push(backend_id.unwrap_or(-1));
        stack.push(StackElement {
            tag,
            id,
            class,
            uid,
            z_index,
        });
    }

    // Overlay detection
    let intercepted_by = detect_overlay(hit_backend_id, &stack, &stack_backend_ids);
    let suggestion = intercepted_by
        .as_ref()
        .map(|overlay| generate_suggestion(overlay, &hit_target, &stack));

    let result = HitTestResult {
        frame: frame_label,
        hit_target,
        intercepted_by,
        stack,
        suggestion,
    };

    print_output(&result, &global.output)?;
    Ok(())
}

// =============================================================================
// Attribute extraction
// =============================================================================

/// Extract `id` and `class` from a CDP node description.
fn extract_attributes(node: &serde_json::Value) -> (Option<String>, Option<String>) {
    let attrs = node["attributes"].as_array();
    let mut id = None;
    let mut class = None;

    if let Some(attr_list) = attrs {
        // Attributes come as [name, value, name, value, ...]
        let mut i = 0;
        while i + 1 < attr_list.len() {
            let name = attr_list[i].as_str().unwrap_or_default();
            let value = attr_list[i + 1].as_str().unwrap_or_default();
            match name {
                "id" if !value.is_empty() => id = Some(value.to_string()),
                "class" if !value.is_empty() => class = Some(value.to_string()),
                _ => {}
            }
            i += 2;
        }
    }

    (id, class)
}

/// Try to resolve a stack element's backend node ID via CSS selector.
/// Accepts the document `root_node_id` to avoid repeated `DOM.getDocument` calls.
async fn resolve_stack_element_backend_id(
    session: &ManagedSession,
    root_node_id: i64,
    tag: &str,
    id: Option<&str>,
    class: Option<&str>,
) -> Option<i64> {
    // Build a CSS selector for this element
    let selector = if let Some(id_val) = id {
        format!("{tag}#{id_val}")
    } else if let Some(class_val) = class {
        if let Some(first_class) = class_val.split_whitespace().next() {
            format!("{tag}.{first_class}")
        } else {
            return None;
        }
    } else {
        return None;
    };

    let query_params = serde_json::json!({
        "nodeId": root_node_id,
        "selector": selector,
    });
    let query_response = session
        .send_command("DOM.querySelector", Some(query_params))
        .await
        .ok()?;

    let node_id = query_response["nodeId"].as_i64().filter(|&n| n != 0)?;

    let describe_params = serde_json::json!({ "nodeId": node_id });
    let describe_response = session
        .send_command("DOM.describeNode", Some(describe_params))
        .await
        .ok()?;

    describe_response["node"]["backendNodeId"].as_i64()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_result_serialization_camel_case() {
        let result = HitTestResult {
            frame: "main".to_string(),
            hit_target: ElementInfo {
                tag: "div".to_string(),
                id: Some("blocker".to_string()),
                class: Some("overlay".to_string()),
                uid: None,
            },
            intercepted_by: None,
            stack: vec![StackElement {
                tag: "div".to_string(),
                id: Some("blocker".to_string()),
                class: Some("overlay".to_string()),
                uid: None,
                z_index: "9999".to_string(),
            }],
            suggestion: None,
        };

        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert_eq!(json["frame"], "main");
        assert_eq!(json["hitTarget"]["tag"], "div");
        assert_eq!(json["hitTarget"]["id"], "blocker");
        assert_eq!(json["hitTarget"]["class"], "overlay");
        assert!(json["hitTarget"]["uid"].is_null());
        assert!(json["interceptedBy"].is_null());
        assert_eq!(json["stack"][0]["zIndex"], "9999");
        assert!(json["suggestion"].is_null());
        // Verify camelCase (no snake_case keys)
        assert!(json.get("hit_target").is_none());
        assert!(json.get("intercepted_by").is_none());
        assert!(json.get("z_index").is_none());
    }

    #[test]
    fn hit_test_result_with_overlay() {
        let result = HitTestResult {
            frame: "main".to_string(),
            hit_target: ElementInfo {
                tag: "div".to_string(),
                id: Some("acc-blocker".to_string()),
                class: Some("overlay transparent".to_string()),
                uid: None,
            },
            intercepted_by: Some(ElementInfo {
                tag: "div".to_string(),
                id: Some("acc-blocker".to_string()),
                class: Some("overlay transparent".to_string()),
                uid: None,
            }),
            stack: vec![
                StackElement {
                    tag: "div".to_string(),
                    id: Some("acc-blocker".to_string()),
                    class: Some("overlay transparent".to_string()),
                    uid: None,
                    z_index: "9999".to_string(),
                },
                StackElement {
                    tag: "button".to_string(),
                    id: Some("submit".to_string()),
                    class: Some("primary".to_string()),
                    uid: Some("s5".to_string()),
                    z_index: "auto".to_string(),
                },
            ],
            suggestion: Some("Element intercepted by div#acc-blocker".to_string()),
        };

        let json: serde_json::Value = serde_json::to_value(&result).unwrap();
        assert!(json["interceptedBy"].is_object());
        assert_eq!(json["interceptedBy"]["tag"], "div");
        assert!(json["suggestion"].is_string());
        assert_eq!(json["stack"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn null_uid_serialized_not_omitted() {
        let info = ElementInfo {
            tag: "div".to_string(),
            id: None,
            class: None,
            uid: None,
        };

        let json: serde_json::Value = serde_json::to_value(&info).unwrap();
        // uid field should be present and null, not omitted
        assert!(json.get("uid").is_some());
        assert!(json["uid"].is_null());
        assert!(json.get("id").is_some());
        assert!(json["id"].is_null());
        assert!(json.get("class").is_some());
        assert!(json["class"].is_null());
    }

    #[test]
    fn element_selector_with_id() {
        let info = ElementInfo {
            tag: "div".to_string(),
            id: Some("blocker".to_string()),
            class: Some("overlay".to_string()),
            uid: None,
        };
        assert_eq!(element_selector(&info), "div#blocker");
    }

    #[test]
    fn element_selector_with_class_only() {
        let info = ElementInfo {
            tag: "div".to_string(),
            id: None,
            class: Some("overlay transparent".to_string()),
            uid: None,
        };
        assert_eq!(element_selector(&info), "div.overlay");
    }

    #[test]
    fn element_selector_bare() {
        let info = ElementInfo {
            tag: "div".to_string(),
            id: None,
            class: None,
            uid: None,
        };
        assert_eq!(element_selector(&info), "div");
    }

    #[test]
    fn detect_overlay_no_overlay() {
        let stack = vec![StackElement {
            tag: "button".to_string(),
            id: Some("submit".to_string()),
            class: None,
            uid: None,
            z_index: "auto".to_string(),
        }];
        let backend_ids = vec![42];
        assert!(detect_overlay(42, &stack, &backend_ids).is_none());
    }

    #[test]
    fn detect_overlay_with_overlay() {
        let stack = vec![
            StackElement {
                tag: "div".to_string(),
                id: Some("blocker".to_string()),
                class: None,
                uid: None,
                z_index: "9999".to_string(),
            },
            StackElement {
                tag: "button".to_string(),
                id: Some("submit".to_string()),
                class: None,
                uid: None,
                z_index: "auto".to_string(),
            },
        ];
        let backend_ids = vec![10, 42];
        let result = detect_overlay(42, &stack, &backend_ids);
        assert!(result.is_some());
        let overlay = result.unwrap();
        assert_eq!(overlay.tag, "div");
        assert_eq!(overlay.id, Some("blocker".to_string()));
    }

    #[test]
    fn detect_overlay_empty_stack() {
        assert!(detect_overlay(42, &[], &[]).is_none());
    }

    #[test]
    fn detect_overlay_non_interactive_above_interactive() {
        // Same backend ID, but topmost is a non-interactive div covering a button.
        let stack = vec![
            StackElement {
                tag: "div".to_string(),
                id: Some("blocker".to_string()),
                class: None,
                uid: None,
                z_index: "9999".to_string(),
            },
            StackElement {
                tag: "button".to_string(),
                id: Some("submit".to_string()),
                class: None,
                uid: None,
                z_index: "auto".to_string(),
            },
        ];
        // CDP resolved to the div (same as stack[0]), but div is non-interactive
        let backend_ids = vec![42, 99];
        let result = detect_overlay(42, &stack, &backend_ids);
        assert!(result.is_some());
        let overlay = result.unwrap();
        assert_eq!(overlay.tag, "div");
        assert_eq!(overlay.id, Some("blocker".to_string()));
    }

    #[test]
    fn detect_overlay_non_interactive_only() {
        // All non-interactive elements — no overlay (no intended target underneath).
        let stack = vec![
            StackElement {
                tag: "div".to_string(),
                id: Some("wrapper".to_string()),
                class: None,
                uid: None,
                z_index: "auto".to_string(),
            },
            StackElement {
                tag: "body".to_string(),
                id: None,
                class: None,
                uid: None,
                z_index: "auto".to_string(),
            },
        ];
        let backend_ids = vec![42, 1];
        assert!(detect_overlay(42, &stack, &backend_ids).is_none());
    }

    #[test]
    fn generate_suggestion_with_uid() {
        let overlay = ElementInfo {
            tag: "div".to_string(),
            id: Some("acc-blocker".to_string()),
            class: None,
            uid: None,
        };
        let hit_target = ElementInfo {
            tag: "div".to_string(),
            id: Some("acc-blocker".to_string()),
            class: None,
            uid: None,
        };
        let stack = vec![
            StackElement {
                tag: "div".to_string(),
                id: Some("acc-blocker".to_string()),
                class: None,
                uid: None,
                z_index: "9999".to_string(),
            },
            StackElement {
                tag: "button".to_string(),
                id: Some("submit".to_string()),
                class: Some("primary".to_string()),
                uid: Some("s5".to_string()),
                z_index: "auto".to_string(),
            },
        ];
        let suggestion = generate_suggestion(&overlay, &hit_target, &stack);
        assert!(suggestion.contains("div#acc-blocker"));
        assert!(suggestion.contains("button#submit"));
        assert!(suggestion.contains("s5"));
    }

    #[test]
    fn generate_suggestion_without_uid() {
        let overlay = ElementInfo {
            tag: "div".to_string(),
            id: None,
            class: Some("blocker".to_string()),
            uid: None,
        };
        let hit_target = ElementInfo {
            tag: "div".to_string(),
            id: None,
            class: Some("blocker".to_string()),
            uid: None,
        };
        let stack = vec![
            StackElement {
                tag: "div".to_string(),
                id: None,
                class: Some("blocker".to_string()),
                uid: None,
                z_index: "9999".to_string(),
            },
            StackElement {
                tag: "a".to_string(),
                id: None,
                class: Some("link".to_string()),
                uid: None,
                z_index: "auto".to_string(),
            },
        ];
        let suggestion = generate_suggestion(&overlay, &hit_target, &stack);
        assert!(suggestion.contains("div.blocker"));
        assert!(suggestion.contains("a.link"));
    }

    #[test]
    fn stack_element_z_index_serialized() {
        let elem = StackElement {
            tag: "button".to_string(),
            id: None,
            class: None,
            uid: None,
            z_index: "auto".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&elem).unwrap();
        assert_eq!(json["zIndex"], "auto");
        assert!(json.get("z_index").is_none());
    }
}
