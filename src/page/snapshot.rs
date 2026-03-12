use agentchrome::error::{AppError, ExitCode};

use crate::cli::{GlobalOpts, PageSnapshotArgs};

use super::{get_page_info, setup_session};

pub async fn execute_snapshot(
    global: &GlobalOpts,
    args: &PageSnapshotArgs,
) -> Result<(), AppError> {
    let (_client, mut managed) = setup_session(global).await?;
    if global.auto_dismiss_dialogs {
        let _dismiss = managed.spawn_auto_dismiss().await?;
    }

    // Enable required domains
    managed.ensure_domain("Accessibility").await?;
    managed.ensure_domain("Runtime").await?;

    // Capture the accessibility tree
    let result = managed
        .send_command("Accessibility.getFullAXTree", None)
        .await
        .map_err(|e| AppError::snapshot_failed(&e.to_string()))?;

    let nodes = result["nodes"]
        .as_array()
        .ok_or_else(|| AppError::snapshot_failed("response missing 'nodes' array"))?;

    // Build tree and assign UIDs
    let build = crate::snapshot::build_tree(nodes, args.verbose);

    // Get page URL for snapshot state
    let (url, _title) = get_page_info(&managed).await?;

    // Persist UID mapping
    let state = crate::snapshot::SnapshotState {
        url,
        timestamp: agentchrome::session::now_iso8601(),
        uid_map: build.uid_map,
    };
    if let Err(e) = crate::snapshot::write_snapshot_state(&state) {
        eprintln!("warning: could not save snapshot state: {e}");
    }

    // Format output
    let formatted = if global.output.json || global.output.pretty {
        // JSON output — add truncation info to root if applicable
        let mut json_value = serde_json::to_value(&build.root).map_err(|e| AppError {
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
        let serializer = if global.output.pretty {
            serde_json::to_string_pretty(&json_value)
        } else {
            serde_json::to_string(&json_value)
        };
        serializer.map_err(|e| AppError {
            message: format!("serialization error: {e}"),
            code: ExitCode::GeneralError,
            custom_json: None,
        })?
    } else {
        // Text output (default and --plain)
        let mut text = crate::snapshot::format_text(&build.root, args.verbose);
        if build.truncated {
            text.push_str(&format!(
                "[... truncated: {} nodes, showing first {}]\n",
                build.total_nodes,
                crate::snapshot::MAX_NODES
            ));
        }
        text
    };

    // Write to file or stdout
    if let Some(ref file_path) = args.file {
        std::fs::write(file_path, &formatted).map_err(|e| {
            AppError::file_write_failed(&file_path.display().to_string(), &e.to_string())
        })?;
    } else {
        print!("{formatted}");
    }

    Ok(())
}
