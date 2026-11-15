//! Inspection-only producer counts. Device pool live/peak storage is reported
//! separately; these source counts are not total application/driver memory.
use eldiron_presentation::SceneViews;

pub(super) fn snapshot(scenes: &SceneViews) -> serde_json::Value {
    let paint = scenes.surface_paint_stats();
    let images = scenes.image_allocation_usage();
    let immutable = scenes.context().immutable_buffer_cache_usage();
    let retained = scenes.context().retained_cache_usage();
    serde_json::json!({
        "pending_frames": scenes.has_pending_surface_paint_frames(),
        "context_retained_cache": {
            "reserved_bytes":retained.bytes, "peak_reserved_bytes":retained.peak_bytes,
            "reservations":retained.reservations, "byte_limit":retained.byte_limit,
            "requests":retained.requests,"rejected_requests":retained.rejected_requests,
        },
        "context_immutable_buffers": {
            "live_bytes": immutable.live_bytes, "live_allocations": immutable.live_allocations,
            "peak_live_bytes": immutable.peak_live_bytes,
            "cached_gpu_bytes": immutable.gpu_bytes, "cached_cpu_bytes": immutable.cpu_bytes,
        },
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
