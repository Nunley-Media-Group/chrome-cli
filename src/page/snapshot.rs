use agentchrome::connection::ManagedSession;
use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, PageSnapshotArgs};

use super::{get_page_info, setup_session};

/// Run a supplemental JS pass to find interactive elements inside open shadow
/// roots that the AX tree may not have captured.
///
/// Returns a map of additional `uid → backendDOMNodeId` entries that should be
/// merged into the main UID map. UIDs start at `s{next_uid}` so they don't
/// collide with UIDs already assigned by `build_tree`.
#[allow(clippy::too_many_lines, clippy::similar_names)]
async fn shadow_dom_supplemental_pass(
    session: &ManagedSession,
    known_backend_ids: &std::collections::HashSet<i64>,
    next_uid: usize,
) -> std::collections::HashMap<String, i64> {
    // JS: collect all shadow roots in document order.
    let hosts_js = r"(function() {
        var roots = [];
        function collect(root) {
            var all = root.querySelectorAll('*');
            for (var i = 0; i < all.length; i++) {
                if (all[i].shadowRoot) {
                    roots.push(all[i].shadowRoot);
                    collect(all[i].shadowRoot);
                }
            }
        }
        collect(document);
        return roots;
    })()";

    let roots_result = session
        .send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": hosts_js,
                "returnByValue": false,
            })),
        )
        .await;

    let Ok(roots_result) = roots_result else {
        return std::collections::HashMap::new();
    };

    let roots_obj_id = roots_result["result"]["objectId"]
        .as_str()
        .unwrap_or_default();
    if roots_obj_id.is_empty() {
        return std::collections::HashMap::new();
    }

    // Get each shadow root object.
    let Ok(props) = session
        .send_command(
            "Runtime.getProperties",
            Some(serde_json::json!({
                "objectId": roots_obj_id,
                "ownProperties": true,
            })),
        )
        .await
    else {
        return std::collections::HashMap::new();
    };

    let interactive_sel = "a[href],button,input,select,textarea,\
        [role='button'],[role='link'],[role='checkbox'],[role='radio'],\
        [role='combobox'],[role='menuitem'],[role='tab'],[role='switch'],\
        [role='slider'],[role='spinbutton'],[role='searchbox'],\
        [role='option'],[role='treeitem']";

    let mut uid_map = std::collections::HashMap::new();
    let mut uid_counter = next_uid;

    let Some(props_arr) = props["result"].as_array() else {
        return uid_map;
    };

    for prop in props_arr {
        // Skip non-index properties.
        if prop["name"]
            .as_str()
            .and_then(|n| n.parse::<u32>().ok())
            .is_none()
        {
            continue;
        }
        let root_obj_id = prop["value"]["objectId"].as_str().unwrap_or_default();
        if root_obj_id.is_empty() {
            continue;
        }

        // Query interactive elements within this shadow root.
        let escaped_sel = serde_json::to_string(interactive_sel).unwrap_or_default();
        let Ok(query_result) = session
            .send_command(
                "Runtime.callFunctionOn",
                Some(serde_json::json!({
                    "functionDeclaration": format!(
                        "function() {{ return Array.from(this.querySelectorAll({escaped_sel})); }}"
                    ),
                    "objectId": root_obj_id,
                    "returnByValue": false,
                })),
            )
            .await
        else {
            continue;
        };

        let elems_obj_id = query_result["result"]["objectId"]
            .as_str()
            .unwrap_or_default();
        if elems_obj_id.is_empty() {
            continue;
        }

        let Ok(elem_props) = session
            .send_command(
                "Runtime.getProperties",
                Some(serde_json::json!({
                    "objectId": elems_obj_id,
                    "ownProperties": true,
                })),
            )
            .await
        else {
            continue;
        };

        let Some(elem_arr) = elem_props["result"].as_array() else {
            continue;
        };

        for elem_prop in elem_arr {
            if elem_prop["name"]
                .as_str()
                .and_then(|n| n.parse::<u32>().ok())
                .is_none()
            {
                continue;
            }
            let elem_obj_id = elem_prop["value"]["objectId"].as_str().unwrap_or_default();
            if elem_obj_id.is_empty() {
                continue;
            }

            // Get backendNodeId for this element.
            let Ok(node_result) = session
                .send_command(
                    "DOM.requestNode",
                    Some(serde_json::json!({ "objectId": elem_obj_id })),
                )
                .await
            else {
                continue;
            };

            let Some(node_id) = node_result["nodeId"].as_i64().filter(|&id| id > 0) else {
                continue;
            };

            let Ok(desc) = session
                .send_command(
                    "DOM.describeNode",
                    Some(serde_json::json!({ "nodeId": node_id })),
                )
                .await
            else {
                continue;
            };

            let Some(backend_id) = desc["node"]["backendNodeId"].as_i64() else {
                continue;
            };

            // Only add IDs not already in the AX tree UID map.
            if known_backend_ids.contains(&backend_id) {
                continue;
            }

            uid_counter += 1;
            let uid = format!("s{uid_counter}");
            uid_map.insert(uid, backend_id);
        }
    }

    uid_map
}

#[allow(clippy::too_many_lines)]
pub async fn execute_snapshot(
    global: &GlobalOpts,
    args: &PageSnapshotArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
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
        eff_mut.ensure_domain("Runtime").await?;
    }

    // Capture the accessibility tree — pass frameId when targeting a same-origin frame
    let fid = frame_ctx.as_ref().and_then(agentchrome::frame::frame_id);
    let ax_params = fid.map(|id| serde_json::json!({ "frameId": id }));

    let result = {
        let effective = if let Some(ref ctx) = frame_ctx {
            agentchrome::frame::frame_session(ctx, &managed)
        } else {
            &managed
        };
        effective
            .send_command("Accessibility.getFullAXTree", ax_params)
            .await
            .map_err(|e| AppError::snapshot_failed(&e.to_string()))?
    };

    let nodes = result["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("response missing 'nodes' array"))?;

    // Build tree and assign UIDs
    let mut build = crate::snapshot::build_tree(nodes, args.verbose);

    // Supplemental shadow DOM pass: find interactive elements inside open shadow
    // roots that the AX tree may have missed.
    if args.pierce_shadow {
        let effective = if let Some(ref ctx) = frame_ctx {
            agentchrome::frame::frame_session(ctx, &managed)
        } else {
            &managed
        };

        // Collect known backendDOMNodeIds from the main AX pass.
        let known_ids: std::collections::HashSet<i64> = build.uid_map.values().copied().collect();
        let next_uid = build.uid_map.len();

        let supplemental = shadow_dom_supplemental_pass(effective, &known_ids, next_uid).await;

        // Merge supplemental UIDs into the main map.
        build.uid_map.extend(supplemental);
    }

    // Get page URL for snapshot state (always from main frame)
    let (url, _title) = get_page_info(&managed).await?;

    // Determine frame metadata for snapshot state
    let (snap_frame_index, snap_frame_id) = match &frame_ctx {
        Some(agentchrome::frame::FrameContext::SameOrigin { frame_id, .. }) => {
            let fid_clone = frame_id.clone();
            let frames_result = agentchrome::frame::list_frames(&mut managed).await;
            let idx = frames_result
                .ok()
                .and_then(|frames| frames.iter().find(|f| f.id == fid_clone).map(|f| f.index));
            (idx, Some(fid_clone))
        }
        Some(agentchrome::frame::FrameContext::OutOfProcess { frame_id, .. }) => {
            let fid_clone = frame_id.clone();
            let frames_result = agentchrome::frame::list_frames(&mut managed).await;
            let idx = frames_result
                .ok()
                .and_then(|frames| frames.iter().find(|f| f.id == fid_clone).map(|f| f.index));
            (idx, Some(fid_clone))
        }
        _ => (None, None),
    };

    // Persist UID mapping
    let state = crate::snapshot::SnapshotState {
        url,
        timestamp: agentchrome::session::now_iso8601(),
        uid_map: build.uid_map,
        frame_index: snap_frame_index,
        frame_id: snap_frame_id,
    };
    if let Err(e) = crate::snapshot::write_snapshot_state(&state) {
        eprintln!("warning: could not save snapshot state: {e}");
    }

    // Apply compact filtering if requested
    let root = if args.compact {
        crate::snapshot::compact_tree(&build.root)
    } else {
        build.root
    };

    // Plain/text output path
    if !global.output.json && !global.output.pretty {
        let mut text = crate::snapshot::format_text(&root, args.verbose);
        if build.truncated {
            text.push_str(&format!(
                "[... truncated: {} nodes, showing first {}]\n",
                build.total_nodes,
                crate::snapshot::MAX_NODES
            ));
        }

        if let Some(ref file_path) = args.file {
            std::fs::write(file_path, &text).map_err(|e| {
                AppError::file_write_failed(&file_path.display().to_string(), &e.to_string())
            })?;
        } else {
            crate::output::emit_plain(&text, &global.output)?;
        }
        return Ok(());
    }

    // JSON output — add truncation info to root if applicable
    let mut json_value = serde_json::to_value(&root).map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;
    if build.truncated {
        if let Some(obj) = json_value.as_object_mut() {
            obj.insert("truncated".to_string(), serde_json::Value::Bool(true));
            obj.insert(
                "total_nodes".to_string(),
                serde_json::Value::Number(build.total_nodes.into()),
            );
        }
    }

    // Write to file if --file is given (bypass the gate)
    if let Some(ref file_path) = args.file {
        let serializer = if global.output.pretty {
            serde_json::to_string_pretty(&json_value)
        } else {
            serde_json::to_string(&json_value)
        };
        let formatted = serializer.map_err(|e| AppError {
            message: format!("serialization error: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?;
        std::fs::write(file_path, &formatted).map_err(|e| {
            AppError::file_write_failed(&file_path.display().to_string(), &e.to_string())
        })?;
        return Ok(());
    }

    // Emit through the large-response gate
    crate::output::emit(&json_value, &global.output, "page snapshot", |v| {
        let total_nodes = crate::snapshot::count_nodes(v);
        let roles = crate::snapshot::top_roles(v, 5);
        serde_json::json!({
            "total_nodes": total_nodes,
            "top_roles": roles,
        })
    })
}
