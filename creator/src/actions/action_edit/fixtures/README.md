# Action display reference fixtures

These twelve 320/800 by 360 RGBA8 captures came from the original
`EditorDisplaySession::draw` at checkpoint e040e079, before paint extraction.
The one-time capture passed in 2.10 seconds:
`/private/tmp/eldiron-action-display-baseline.log`.

The fixtures cover ordinary and mirrored profiles, selected/hovered points,
fine snap controls, the scale slider, nonuniform presets, pan/zoom and clipped
geometry. Existing reference pixels, including alpha, must remain byte-exact.
Tests decode the original PNGs and never regenerate them. Native Swash text
metrics/AA and complete workflow/platform acceptance require separate evidence.
