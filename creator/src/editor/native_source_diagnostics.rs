//! Inspection-only producer counts. Device pool live/peak storage is reported
//! separately; these source counts are not total application/driver memory.
use eldiron_presentation::SceneViews;

pub(super) fn snapshot(scenes: &SceneViews) -> serde_json::Value {
    let paint = scenes.surface_paint_stats();
    let images = scenes.image_allocation_usage();
    serde_json::json!({
        "pending_frames": scenes.has_pending_surface_paint_frames(),
        "context_unpooled_images": {
            "live_bytes": images.live_bytes, "live_images": images.live_images,
            "peak_live_bytes": images.peak_live_bytes, "allocations": images.allocations,
        },
        "paint_producers": {
            "sources": paint.sources,
            "logical_cpu_page_bytes": paint.logical_cpu_page_bytes,
            "gpu_plane_bytes": paint.gpu_plane_bytes,
            "gpu_table_bytes": paint.gpu_table_bytes,
            "pending_sources": paint.pending_sources,
            "failed_sources": paint.failed_sources,
            "pending_views": paint.pending_views,
            "uploaded_pages": paint.uploaded_pages,
            "reused_pages": paint.reused_pages,
            "gpu_copied_bytes": paint.gpu_copied_bytes,
            "reused_bindings": paint.reused_bindings,
            "frame_uploaded_bytes": paint.frame_uploaded_bytes,
        },
        "scene_preparation_this_frame": {
            "calls": scenes.scene_source_preparation_calls(),
            "cpu_ns": scenes.scene_source_preparation_cpu_ns(),
            "packed_cpu_bytes": scenes.scene_source_packed_cpu_bytes(),
            "uploaded_bytes": scenes.scene_source_uploaded_bytes(),
            "gpu_copied_bytes": scenes.scene_source_gpu_copied_bytes(),
        },
    })
}
