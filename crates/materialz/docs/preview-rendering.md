# Material preview rendering

Procedural outputs run as Metal compute shaders. Varying material channels bake
into linear float4 GPU buffers at 512×512. Multiple changed channels share one
command buffer and one completion wait. The renderer copies these buffers into
its texture atlas with GPU blits; it does not turn shading maps into Swift pixel
arrays or upload them through the CPU texture path.

Channel caches compare generated shader source and parameter values. Renaming or
moving a material, selecting a tile, or rotating the camera does not rebake
unchanged channels. Pattern panes and tile thumbnails share a device-scoped
pipeline/library cache. Whole-material mixers conservatively propagate constant
fields, avoiding texture baking for uniform outputs. Unknown or varying outputs
continue through the normal bake path.

Bump normals are calculated on the GPU. Mesh displacement remains a CPU geometry
operation: a changed height buffer is read once to construct the displaced sphere.
Camera rotation updates the existing session without rebuilding the mesh, atlas,
acceleration structures or targets. Material/lighting/geometry changes still
recreate a session when their rendered content changes.

The sphere path tracer retains the existing 768×768, 64-sample, eight-bounce
quality. It reads back displayed milestones only, rather than every accumulated
sample. Tone mapping, backdrop composition and linear-to-sRGB conversion run on
the GPU. SwiftUI currently receives the final 8-bit CGImage, so a display readback
remains. Legacy CPU image methods remain for reference/parity checks.

Native checks compare GPU atlas copies and bump normals with the CPU reference,
check constant propagation against Rust channel evaluation, reject non-finite
images, and verify channel reuse for camera/layout edits.

Local first-frame benchmark, `examples/seamless.material` (three tiles), same
interactive request with displacement and no thumbnail work:

| Request | Before | GPU path |
| --- | ---: | ---: |
| First frame | 1.69 s | 0.31 s |
| Same material again | 1.11 s | 0.032 s |
| Rotated camera | 1.36 s | 0.013 s |

These are local development measurements, not latency guarantees for other
hardware or graphs. Cold driver compilation and larger graphs can still cost
more. Benchmark logs are in `work/render-baseline.log` and
`work/render-gpu-final.log`.
