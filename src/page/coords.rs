//! `page coords` subcommand — resolve a selector to frame-local and page-global coordinates.

use serde::Serialize;

use agentchrome::error::AppError;

use agentchrome::coords::BoundingBox;

use crate::cli::{GlobalOpts, PageCoordsArgs};
use crate::coord_helpers::{frame_viewport_offset, resolve_element_box};

use super::{print_output, setup_session};

// =============================================================================
// Output types
// =============================================================================

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameRef {
    index: u32,
    id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoordPoint {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoordView {
    bounding_box: CoordRect,
    center: CoordPoint,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoordRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoordsOutput {
    frame: FrameRef,
    frame_local: CoordView,
    page: CoordView,
    frame_offset: CoordPoint,
}

// =============================================================================
// Helpers
// =============================================================================

fn bounding_box_to_rect(bb: BoundingBox) -> CoordRect {
    CoordRect {
        x: bb.x,
        y: bb.y,
        width: bb.width,
        height: bb.height,
    }
}

fn center_of(bb: BoundingBox) -> CoordPoint {
    CoordPoint {
        x: bb.x + bb.width / 2.0,
        y: bb.y + bb.height / 2.0,
    }
}

fn offset_bounding_box(bb: BoundingBox, offset: (f64, f64)) -> BoundingBox {
    BoundingBox {
        x: bb.x + offset.0,
        y: bb.y + offset.1,
        width: bb.width,
        height: bb.height,
    }
}

// =============================================================================
// Executor
// =============================================================================

/// Execute the `page coords` subcommand.
///
/// # Errors
///
/// Returns `AppError` if the session, frame, or selector resolution fails.
pub(crate) async fn execute_coords(
    global: &GlobalOpts,
    args: &PageCoordsArgs,
    frame: Option<&str>,
) -> Result<(), AppError> {
    let (client, mut managed) = setup_session(global).await?;

    // Resolve frame context (`mut` required so we can call frame_session_mut inside a scope)
    let mut frame_ctx =
        crate::output::resolve_optional_frame(&client, &mut managed, frame, None).await?;

    // Determine the numeric frame index and the CDP frame ID via Page.getFrameTree
    let frames = agentchrome::frame::list_frames(&mut managed).await?;
    let (frame_index, frame_cdp_id) = if let Some(ref ctx) = frame_ctx {
        let idx = frame.and_then(|f| f.parse::<u32>().ok()).unwrap_or(0);
        // Try to match the FrameContext's CDP frame_id against the frames list
        let cdp_id = agentchrome::frame::frame_id(ctx)
            .and_then(|fid| frames.iter().find(|fi| fi.id == fid))
            .or_else(|| frames.iter().find(|fi| fi.index == idx))
            .map(|fi| fi.id.clone())
            .unwrap_or_default();
        (idx, cdp_id)
    } else {
        // Main frame: index 0
        let cdp_id = frames.first().map(|fi| fi.id.clone()).unwrap_or_default();
        (0u32, cdp_id)
    };

    // Ensure DOM + Runtime domains are available on the effective session (scoped borrow)
    {
        let eff_mut = if let Some(ref mut ctx) = frame_ctx {
            agentchrome::frame::frame_session_mut(ctx, &mut managed)
        } else {
            &mut managed
        };
        eff_mut.ensure_domain("DOM").await?;
        eff_mut.ensure_domain("Runtime").await?;
    }

    // Resolve element bounding box in frame-local coordinates
    let local_box = resolve_element_box(&managed, frame_ctx.as_ref(), &args.selector).await?;

    // Get frame offset (main frame → (0,0); child frame → iframe's page position)
    let (off_x, off_y) = if let Some(ref ctx) = frame_ctx {
        frame_viewport_offset(&managed, ctx).await?
    } else {
        (0.0, 0.0)
    };

    // Compute page-global bounding box
    let page_box = offset_bounding_box(local_box, (off_x, off_y));

    let output = CoordsOutput {
        frame: FrameRef {
            index: frame_index,
            id: frame_cdp_id,
        },
        frame_local: CoordView {
            bounding_box: bounding_box_to_rect(local_box),
            center: center_of(local_box),
        },
        page: CoordView {
            bounding_box: bounding_box_to_rect(page_box),
            center: center_of(page_box),
        },
        frame_offset: CoordPoint { x: off_x, y: off_y },
    };

    print_output(&output, &global.output)
}
