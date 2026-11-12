# Ordinary editor-view reference captures

Captured on Apple Metal from the existing `Client::draw_d2` and
`Client::draw_d3_with_camera_override` methods before scene-preparation extraction,
based on rewrite checkpoint `0f96555c`. The fixture source is in
`../../editor_views.rs`; images are 96 x 80, RGBA8, with their original alpha.

The eight cases cover raster/compute 2D, grids and disabled overlays, theme
colors, 3D orbit lighting, neutral/sky/transparent backgrounds, three independent
guide layers, a camera override and the reference's separate guide camera.
These are narrow deterministic GPU fixtures, not full Creator workflow coverage.

The tests compare decoded pixels exactly. They never regenerate these expected
images. The pre-extraction capture run passed in 3.18 s; its log is
`/private/tmp/eldiron-editor-view-baseline.log`. An independent after-extraction
run produced the same bytes in all eight cases before adding native output tests.
