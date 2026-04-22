use serde::Serialize;

use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, PageFindArgs};
use crate::output;

use super::{get_page_info, setup_session};

// =============================================================================
// Output types
// =============================================================================

/// A single element match from `page find`.
#[derive(Debug, Clone, Serialize)]
struct FindMatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    uid: Option<String>,
    role: String,
    name: String,
    #[serde(rename = "boundingBox", skip_serializing_if = "Option::is_none")]
    bounding_box: Option<BoundingBox>,
}

/// Pixel-based bounding box of an element.
#[derive(Debug, Clone, Serialize)]
struct BoundingBox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

// =============================================================================
// Summary builder
// =============================================================================

/// Build a domain-specific summary for the `page find` large-response gate.
///
/// Fields:
/// - `match_count`: total number of matches returned
/// - `roles_seen`: distinct role values across all matches
fn summary_of_find(matches: &[FindMatch]) -> serde_json::Value {
    #[allow(clippy::cast_possible_truncation)]
    let match_count = matches.len() as u64;
    let mut roles: Vec<String> = matches
        .iter()
        .map(|m| m.role.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    roles.sort_unstable();

    serde_json::json!({
        "match_count": match_count,
        "roles_seen": roles,
    })
}

// =============================================================================
// Bounding box resolution
// =============================================================================

/// Resolve bounding box for a DOM node by its `backendDOMNodeId`.
///
/// Returns `None` if the element is invisible or has been removed from the DOM.
async fn resolve_bounding_box(
    managed: &ManagedSession,
    backend_dom_node_id: i64,
) -> Option<BoundingBox> {
    // Resolve backendNodeId → nodeId via DOM.describeNode
    let describe = managed
        .send_command(
            "DOM.describeNode",
            Some(serde_json::json!({ "backendNodeId": backend_dom_node_id })),
        )
        .await
        .ok()?;

    let node_id = describe["node"]["nodeId"].as_i64()?;

    // Get box model for the resolved nodeId
    let box_result = managed
        .send_command(
            "DOM.getBoxModel",
            Some(serde_json::json!({ "nodeId": node_id })),
        )
        .await
        .ok()?;

    let content = box_result["model"]["content"].as_array()?;
    if content.len() < 8 {
        return None;
    }

    // Content quad: [x1,y1, x2,y2, x3,y3, x4,y4] — top-left and bottom-right
    let x1 = content[0].as_f64()?;
    let y1 = content[1].as_f64()?;
    let x3 = content[4].as_f64()?;
    let y3 = content[5].as_f64()?;

    Some(BoundingBox {
        x: x1,
        y: y1,
        width: x3 - x1,
        height: y3 - y1,
    })
}

// =============================================================================
// CSS selector search
// =============================================================================

/// Find elements by CSS selector using CDP DOM methods.
///
/// Returns up to `limit` matches with role, name, and bounding box.
async fn find_by_selector(
    managed: &ManagedSession,
    selector: &str,
    limit: usize,
) -> Result<Vec<FindMatch>, AppError> {
    // Get the document root
    let doc = managed
        .send_command("DOM.getDocument", None)
        .await
        .map_err(|e| AppError::snapshot_failed(&e.to_string()))?;
    let root_node_id = doc["root"]["nodeId"]
        .as_i64()
        .ok_or_else(|| AppError::snapshot_failed("DOM.getDocument missing root nodeId"))?;

    // Query all matching nodes
    let query_result = managed
        .send_command(
            "DOM.querySelectorAll",
            Some(serde_json::json!({
                "nodeId": root_node_id,
                "selector": selector,
            })),
        )
        .await
        .map_err(|e| AppError {
            message: format!("CSS selector query failed: {e}"),
            code: ExitCode::ProtocolError,
            custom_json: None,
        })?;

    let node_ids = query_result["nodeIds"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("DOM.querySelectorAll missing nodeIds"))?;

    let mut matches = Vec::with_capacity(limit.min(node_ids.len()));
    for node_id_val in node_ids.iter().take(limit) {
        let Some(node_id) = node_id_val.as_i64() else {
            continue;
        };

        // Get backendDOMNodeId via DOM.describeNode
        let describe = managed
            .send_command(
                "DOM.describeNode",
                Some(serde_json::json!({ "nodeId": node_id })),
            )
            .await;

        let backend_dom_node_id = describe
            .as_ref()
            .ok()
            .and_then(|d| d["node"]["backendNodeId"].as_i64());

        // Get accessibility info for this node
        let (role, name) = if let Some(backend_id) = backend_dom_node_id {
            let ax_result = managed
                .send_command(
                    "Accessibility.getPartialAXTree",
                    Some(serde_json::json!({
                        "backendNodeId": backend_id,
                        "fetchRelatives": false,
                    })),
                )
                .await;

            if let Ok(ax) = ax_result {
                let nodes = ax["nodes"].as_array();
                let first = nodes.and_then(|arr| arr.first());
                let role = first
                    .and_then(|n| n["role"]["value"].as_str())
                    .unwrap_or("none")
                    .to_string();
                let name = first
                    .and_then(|n| n["name"]["value"].as_str())
                    .unwrap_or("")
                    .to_string();
                (role, name)
            } else {
                ("none".to_string(), String::new())
            }
        } else {
            ("none".to_string(), String::new())
        };

        // Get bounding box
        let bounding_box = if let Some(backend_id) = backend_dom_node_id {
            resolve_bounding_box(managed, backend_id).await
        } else {
            None
        };

        matches.push(FindMatch {
            uid: None, // UIDs assigned after full snapshot
            role,
            name,
            bounding_box,
        });
    }

    Ok(matches)
}

// =============================================================================
// Snapshot capture helper
// =============================================================================

/// Capture the accessibility tree, build it, and persist snapshot state.
async fn capture_snapshot(
    managed: &ManagedSession,
) -> Result<crate::snapshot::BuildResult, AppError> {
    let ax_result = managed
        .send_command("Accessibility.getFullAXTree", None)
        .await
        .map_err(|e| AppError::snapshot_failed(&e.to_string()))?;
    let nodes = ax_result["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("response missing 'nodes' array"))?;
    let build = crate::snapshot::build_tree(nodes, false);

    let (url, _title) = get_page_info(managed).await?;
    let state = crate::snapshot::SnapshotState {
        url,
        timestamp: agentchrome::session::now_iso8601(),
        uid_map: build.uid_map.clone(),
        frame_index: None,
        frame_id: None,
        aggregate: false,
        frame_uid_ranges: Vec::new(),
        frame_ids: Vec::new(),
    };
    if let Err(e) = crate::snapshot::write_snapshot_state(&state) {
        eprintln!("warning: could not save snapshot state: {e}");
    }

    Ok(build)
}

// =============================================================================
// UID assignment from snapshot tree
// =============================================================================

/// Try to assign a UID from the snapshot tree to a CSS-matched element.
fn assign_uid_from_snapshot(node: &crate::snapshot::SnapshotNode, m: &mut FindMatch) {
    if node.role == m.role && node.name == m.name {
        if let Some(ref uid) = node.uid {
            m.uid = Some(uid.clone());
            return;
        }
    }
    for child in &node.children {
        if m.uid.is_some() {
            return;
        }
        assign_uid_from_snapshot(child, m);
    }
}

// =============================================================================
// Command executor
// =============================================================================

pub async fn execute_find(
    global: &GlobalOpts,
    args: &PageFindArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    // Validate: at least one of query, selector, or role must be provided
    if args.query.is_none() && args.selector.is_none() && args.role.is_none() {
        return Err(AppError {
            message: "a text query, --selector, or --role is required".to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    let (client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Resolve optional frame context
    let mut frame_ctx = if let Some(frame_str) = frame {
        let arg = agentchrome::frame::parse_frame_arg(frame_str)?;
        Some(agentchrome::frame::resolve_frame(&client, &mut managed, &arg).await?)
    } else {
        None
    };

    // Enable required domains (needs &mut)
    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("Accessibility").await?;
        eff_mut.ensure_domain("DOM").await?;
        eff_mut.ensure_domain("Runtime").await?;
    }

    // Capture snapshot (used by both search paths for UID assignment)
    let build = {
        let effective = if let Some(ref ctx) = frame_ctx {
            agentchrome::frame::frame_session(ctx, &managed)
        } else {
            &managed
        };
        capture_snapshot(effective).await?
    };

    let effective = if let Some(ref ctx) = frame_ctx {
        agentchrome::frame::frame_session(ctx, &managed)
    } else {
        &managed
    };

    let matches = if let Some(ref selector) = args.selector {
        // CSS selector path
        let mut css_matches = find_by_selector(effective, selector, args.limit).await?;

        // Enrich CSS matches with UIDs from the snapshot tree
        for m in &mut css_matches {
            assign_uid_from_snapshot(&build.root, m);
        }

        css_matches
    } else {
        // Accessibility text search path
        let query = args.query.as_deref().unwrap_or("");

        let hits = crate::snapshot::search_tree(
            &build.root,
            query,
            args.role.as_deref(),
            args.exact,
            args.limit,
        );

        // Resolve bounding boxes for each hit
        let mut matches = Vec::with_capacity(hits.len());
        for hit in hits {
            let bounding_box = if let Some(backend_id) = hit.backend_dom_node_id {
                resolve_bounding_box(effective, backend_id).await
            } else {
                None
            };
            matches.push(FindMatch {
                uid: hit.uid,
                role: hit.role,
                name: hit.name,
                bounding_box,
            });
        }
        matches
    };

    // Output
    if global.output.plain {
        for m in &matches {
            let uid_str = m.uid.as_ref().map_or(String::new(), |u| format!("[{u}] "));
            let bb_str = m.bounding_box.as_ref().map_or(String::new(), |bb| {
                format!(" ({},{} {}x{})", bb.x, bb.y, bb.width, bb.height)
            });
            println!("{uid_str}{} \"{}\"{bb_str}", m.role, m.name);
        }
        return Ok(());
    }

    output::emit(
        &matches,
        &global.output,
        "page find",
        |v: &Vec<FindMatch>| summary_of_find(v.as_slice()),
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // FindMatch / BoundingBox serialization tests
    // =========================================================================

    #[test]
    fn find_match_serialization_with_all_fields() {
        let m = FindMatch {
            uid: Some("s3".to_string()),
            role: "button".to_string(),
            name: "Submit".to_string(),
            bounding_box: Some(BoundingBox {
                x: 120.0,
                y: 340.0,
                width: 80.0,
                height: 36.0,
            }),
        };
        let json: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(json["uid"], "s3");
        assert_eq!(json["role"], "button");
        assert_eq!(json["name"], "Submit");
        assert_eq!(json["boundingBox"]["x"], 120.0);
        assert_eq!(json["boundingBox"]["y"], 340.0);
        assert_eq!(json["boundingBox"]["width"], 80.0);
        assert_eq!(json["boundingBox"]["height"], 36.0);
    }

    #[test]
    fn find_match_serialization_without_uid() {
        let m = FindMatch {
            uid: None,
            role: "heading".to_string(),
            name: "Title".to_string(),
            bounding_box: Some(BoundingBox {
                x: 50.0,
                y: 10.0,
                width: 300.0,
                height: 32.0,
            }),
        };
        let json: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert!(json.get("uid").is_none());
        assert_eq!(json["role"], "heading");
        assert!(json.get("boundingBox").is_some());
    }

    #[test]
    fn find_match_serialization_without_bounding_box() {
        let m = FindMatch {
            uid: Some("s1".to_string()),
            role: "button".to_string(),
            name: "Hidden".to_string(),
            bounding_box: None,
        };
        let json: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert_eq!(json["uid"], "s1");
        assert!(json.get("boundingBox").is_none());
    }

    #[test]
    fn find_match_serialization_minimal() {
        let m = FindMatch {
            uid: None,
            role: "text".to_string(),
            name: "Hello".to_string(),
            bounding_box: None,
        };
        let json: serde_json::Value = serde_json::to_value(&m).unwrap();
        assert!(json.get("uid").is_none());
        assert!(json.get("boundingBox").is_none());
        assert_eq!(json["role"], "text");
        assert_eq!(json["name"], "Hello");
    }

    #[test]
    fn find_match_array_serialization() {
        let matches = vec![
            FindMatch {
                uid: Some("s1".to_string()),
                role: "button".to_string(),
                name: "OK".to_string(),
                bounding_box: Some(BoundingBox {
                    x: 10.0,
                    y: 20.0,
                    width: 50.0,
                    height: 30.0,
                }),
            },
            FindMatch {
                uid: None,
                role: "heading".to_string(),
                name: "Title".to_string(),
                bounding_box: None,
            },
        ];
        let json: serde_json::Value = serde_json::to_value(&matches).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["uid"], "s1");
        assert!(arr[1].get("uid").is_none());
    }

    #[test]
    fn find_match_empty_array_serialization() {
        let matches: Vec<FindMatch> = vec![];
        let json = serde_json::to_string(&matches).unwrap();
        assert_eq!(json, "[]");
    }

    #[test]
    fn bounding_box_camel_case_key() {
        let m = FindMatch {
            uid: None,
            role: "button".to_string(),
            name: "Test".to_string(),
            bounding_box: Some(BoundingBox {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 50.0,
            }),
        };
        let json_str = serde_json::to_string(&m).unwrap();
        assert!(json_str.contains("\"boundingBox\""));
        assert!(!json_str.contains("\"bounding_box\""));
    }

    // =========================================================================
    // assign_uid_from_snapshot tests
    // =========================================================================

    fn make_snapshot_node(
        role: &str,
        name: &str,
        uid: Option<&str>,
        children: Vec<crate::snapshot::SnapshotNode>,
    ) -> crate::snapshot::SnapshotNode {
        crate::snapshot::SnapshotNode {
            role: role.to_string(),
            name: name.to_string(),
            uid: uid.map(String::from),
            properties: None,
            backend_dom_node_id: None,
            frame: None,
            children,
        }
    }

    #[test]
    fn assign_uid_matches_role_and_name() {
        let tree = make_snapshot_node(
            "document",
            "Page",
            None,
            vec![make_snapshot_node("button", "Submit", Some("s1"), vec![])],
        );
        let mut m = FindMatch {
            uid: None,
            role: "button".to_string(),
            name: "Submit".to_string(),
            bounding_box: None,
        };
        assign_uid_from_snapshot(&tree, &mut m);
        assert_eq!(m.uid.as_deref(), Some("s1"));
    }

    #[test]
    fn assign_uid_no_match_leaves_none() {
        let tree = make_snapshot_node(
            "document",
            "Page",
            None,
            vec![make_snapshot_node("button", "Submit", Some("s1"), vec![])],
        );
        let mut m = FindMatch {
            uid: None,
            role: "link".to_string(),
            name: "Other".to_string(),
            bounding_box: None,
        };
        assign_uid_from_snapshot(&tree, &mut m);
        assert!(m.uid.is_none());
    }

    #[test]
    fn assign_uid_first_match_wins() {
        let tree = make_snapshot_node(
            "document",
            "Page",
            None,
            vec![
                make_snapshot_node("button", "OK", Some("s1"), vec![]),
                make_snapshot_node("button", "OK", Some("s2"), vec![]),
            ],
        );
        let mut m = FindMatch {
            uid: None,
            role: "button".to_string(),
            name: "OK".to_string(),
            bounding_box: None,
        };
        assign_uid_from_snapshot(&tree, &mut m);
        assert_eq!(m.uid.as_deref(), Some("s1"));
    }

    // =========================================================================
    // summary_of_find
    // =========================================================================

    fn make_find_match(role: &str, name: &str) -> FindMatch {
        FindMatch {
            uid: None,
            role: role.to_string(),
            name: name.to_string(),
            bounding_box: None,
        }
    }

    #[test]
    fn summary_of_find_empty_matches() {
        let matches: Vec<FindMatch> = vec![];
        let summary = summary_of_find(&matches);
        assert_eq!(summary["match_count"], 0);
        let roles = summary["roles_seen"].as_array().unwrap();
        assert!(roles.is_empty());
    }

    #[test]
    fn summary_of_find_counts_matches() {
        let matches = vec![
            make_find_match("button", "OK"),
            make_find_match("button", "Cancel"),
            make_find_match("link", "Home"),
        ];
        let summary = summary_of_find(&matches);
        assert_eq!(summary["match_count"], 3);
    }

    #[test]
    fn summary_of_find_deduplicates_roles() {
        let matches = vec![
            make_find_match("button", "OK"),
            make_find_match("button", "Cancel"),
            make_find_match("link", "Home"),
        ];
        let summary = summary_of_find(&matches);
        let roles: Vec<&str> = summary["roles_seen"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(roles.len(), 2);
        assert!(roles.contains(&"button"));
        assert!(roles.contains(&"link"));
    }

    #[test]
    fn summary_of_find_roles_sorted() {
        let matches = vec![
            make_find_match("link", "Home"),
            make_find_match("button", "OK"),
            make_find_match("heading", "Title"),
        ];
        let summary = summary_of_find(&matches);
        let roles: Vec<&str> = summary["roles_seen"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        // Must be sorted
        let mut sorted = roles.clone();
        sorted.sort_unstable();
        assert_eq!(roles, sorted);
    }
}
