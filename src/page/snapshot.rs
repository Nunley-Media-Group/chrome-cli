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
        let escaped_sel =
            serde_json::to_string(interactive_sel).expect("serializing a &'static str cannot fail");
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
    if args.include_iframes && frame.is_some() {
        return Err(AppError {
            message: "--include-iframes and --frame are mutually exclusive".to_string(),
            code: ExitCode::GeneralError,
            custom_json: None,
        });
    }

    if args.include_iframes {
        return execute_aggregate_snapshot(global, args).await;
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
        aggregate: false,
        frame_uid_ranges: Vec::new(),
        frame_ids: Vec::new(),
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
        use std::fmt::Write as _;

        let mut text = crate::snapshot::format_text(&root, args.verbose);
        if build.truncated {
            let _ = writeln!(
                text,
                "[... truncated: {} nodes, showing first {}]",
                build.total_nodes,
                crate::snapshot::MAX_NODES
            );
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
    if build.truncated
        && let Some(obj) = json_value.as_object_mut()
    {
        obj.insert("truncated".to_string(), serde_json::Value::Bool(true));
        obj.insert(
            "total_nodes".to_string(),
            serde_json::Value::Number(build.total_nodes.into()),
        );
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

// =============================================================================
// Script runner compute function
// =============================================================================

/// Compute a page snapshot against an existing session and return the JSON value.
///
/// Used by the script runner to invoke `page snapshot` without printing to stdout.
///
/// # Errors
///
/// Returns `AppError` on snapshot failure.
pub async fn compute_snapshot(
    managed: &mut agentchrome::connection::ManagedSession,
    _args: &PageSnapshotArgs,
) -> Result<serde_json::Value, AppError> {
    // Delegate to a minimal implementation using the provided session.
    managed.ensure_domain("Accessibility").await?;
    managed.ensure_domain("Runtime").await?;

    let result = managed
        .send_command("Accessibility.getFullAXTree", None)
        .await
        .map_err(|e| AppError::snapshot_failed(&e.to_string()))?;

    let nodes = result["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("response missing 'nodes' array"))?;

    let build = crate::snapshot::build_tree(nodes, false);

    // Persist UID mapping
    let (url, _) = super::get_page_info(managed).await?;
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

    serde_json::to_value(&build.root).map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })
}

/// Aggregate snapshot: build main-frame tree and splice every enumerable
/// iframe's accessibility tree under its owner node.
#[allow(clippy::too_many_lines)]
async fn execute_aggregate_snapshot(
    global: &GlobalOpts,
    args: &PageSnapshotArgs,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    managed.ensure_domain("Accessibility").await?;
    managed.ensure_domain("Runtime").await?;
    managed.ensure_domain("DOM").await?;

    // Enumerate all frames in document order.
    let frames = agentchrome::frame::list_frames(&mut managed).await?;

    // Build main-frame tree.
    let main_ax = managed
        .send_command("Accessibility.getFullAXTree", None)
        .await
        .map_err(|e| AppError::snapshot_failed(&e.to_string()))?;
    let main_nodes = main_ax["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("response missing 'nodes' array"))?;
    let mut main_build = crate::snapshot::build_tree(main_nodes, args.verbose);

    // Optional shadow-DOM pass on the main frame.
    if args.pierce_shadow {
        let known: std::collections::HashSet<i64> = main_build.uid_map.values().copied().collect();
        let next_uid = main_build.uid_map.len();
        let supplemental = shadow_dom_supplemental_pass(&managed, &known, next_uid).await;
        main_build.uid_map.extend(supplemental);
    }

    let mut merged_root = main_build.root;
    let mut merged_uid_map = main_build.uid_map;
    let mut frame_uid_ranges: Vec<(u32, (u32, u32))> = Vec::new();
    let mut frame_ids: Vec<(u32, String)> = Vec::new();
    // Record main-frame range.
    if !merged_uid_map.is_empty() {
        frame_uid_ranges.push((
            0,
            (1, u32::try_from(merged_uid_map.len()).unwrap_or(u32::MAX)),
        ));
    }
    if let Some(main_frame) = frames.first() {
        frame_ids.push((0, main_frame.id.clone()));
    }

    // Splice each non-main frame.
    for frame_info in frames.iter().skip(1) {
        let Ok(owner) = managed
            .send_command(
                "DOM.getFrameOwner",
                Some(serde_json::json!({ "frameId": frame_info.id })),
            )
            .await
        else {
            continue;
        };
        let Some(owner_backend_id) = owner["backendNodeId"].as_i64() else {
            continue;
        };

        // Build that frame's AX tree. Try same-origin (frameId on the page session)
        // first; if that fails, attempt an OOPIF attach.
        let frame_ax = managed
            .send_command(
                "Accessibility.getFullAXTree",
                Some(serde_json::json!({ "frameId": frame_info.id })),
            )
            .await;

        let (frame_nodes_value, frame_session_for_shadow): (
            serde_json::Value,
            Option<agentchrome::connection::ManagedSession>,
        ) = if let Ok(v) = frame_ax {
            (v, None)
        } else {
            // OOPIF path.
            let targets = client
                .send_command("Target.getTargets", None)
                .await
                .map_err(|e| AppError {
                    message: format!("Target.getTargets failed: {e}"),
                    code: ExitCode::ProtocolError,
                    custom_json: None,
                })?;
            let Some(target_id) = targets["targetInfos"].as_array().and_then(|arr| {
                arr.iter()
                    .find(|t| {
                        t["targetId"].as_str() == Some(frame_info.id.as_str())
                            || (t["type"].as_str() == Some("iframe")
                                && t["url"].as_str() == Some(frame_info.url.as_str()))
                    })
                    .and_then(|t| t["targetId"].as_str().map(String::from))
            }) else {
                continue;
            };
            let Ok(oopif_raw) = client.create_session(&target_id).await else {
                continue;
            };
            let oopif = agentchrome::connection::ManagedSession::new(oopif_raw);
            let Ok(v) = oopif
                .send_command("Accessibility.getFullAXTree", None)
                .await
            else {
                continue;
            };
            (v, Some(oopif))
        };

        let Some(frame_nodes) = frame_nodes_value["nodes"].as_array() else {
            continue;
        };

        let uid_offset = merged_uid_map.len();
        let mut frame_build =
            crate::snapshot::build_tree_with_uid_offset(frame_nodes, args.verbose, uid_offset);

        // Optional shadow-DOM pass within this frame.
        if args.pierce_shadow {
            let known: std::collections::HashSet<i64> =
                frame_build.uid_map.values().copied().collect();
            let next_uid = uid_offset + frame_build.uid_map.len();
            let shadow_session = frame_session_for_shadow.as_ref().unwrap_or(&managed);
            let supplemental = shadow_dom_supplemental_pass(shadow_session, &known, next_uid).await;
            frame_build.uid_map.extend(supplemental);
        }

        let uid_start = u32::try_from(uid_offset + 1).unwrap_or(u32::MAX);
        let uid_end =
            u32::try_from(merged_uid_map.len() + frame_build.uid_map.len()).unwrap_or(u32::MAX);
        if !frame_build.uid_map.is_empty() {
            frame_uid_ranges.push((frame_info.index, (uid_start, uid_end)));
        }
        frame_ids.push((frame_info.index, frame_info.id.clone()));

        merged_uid_map.extend(frame_build.uid_map);

        // Annotate the spliced subtree's root with the frame index.
        let mut frame_root = frame_build.root;
        frame_root.frame = Some(frame_info.index);

        crate::snapshot::splice_frame_subtree(&mut merged_root, owner_backend_id, frame_root);
    }

    // Persist aggregate SnapshotState.
    let (url, _title) = get_page_info(&managed).await?;
    let state = crate::snapshot::SnapshotState {
        url,
        timestamp: agentchrome::session::now_iso8601(),
        uid_map: merged_uid_map,
        frame_index: None,
        frame_id: None,
        aggregate: true,
        frame_uid_ranges,
        frame_ids,
    };
    if let Err(e) = crate::snapshot::write_snapshot_state(&state) {
        let warning = serde_json::json!({
            "warning": format!("could not save snapshot state: {e}"),
            "command": "page snapshot",
        });
        eprintln!("{warning}");
    }

    let root = if args.compact {
        crate::snapshot::compact_tree(&merged_root)
    } else {
        merged_root
    };

    // Plain/text output path
    if !global.output.json && !global.output.pretty {
        let text = crate::snapshot::format_text(&root, args.verbose);
        if let Some(ref file_path) = args.file {
            std::fs::write(file_path, &text).map_err(|e| {
                AppError::file_write_failed(&file_path.display().to_string(), &e.to_string())
            })?;
        } else {
            crate::output::emit_plain(&text, &global.output)?;
        }
        return Ok(());
    }

    let json_value = serde_json::to_value(&root).map_err(|e| AppError {
        message: format!("serialization error: {e}"),
        code: ExitCode::GeneralError,
        custom_json: None,
    })?;

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

    crate::output::emit(&json_value, &global.output, "page snapshot", |v| {
        let total_nodes = crate::snapshot::count_nodes(v);
        let roles = crate::snapshot::top_roles(v, 5);
        serde_json::json!({
            "total_nodes": total_nodes,
            "top_roles": roles,
        })
    })
}
