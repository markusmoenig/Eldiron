# Paint panel reference images

These captures came from the original framebuffer draw method at checkpoint
4114b24c, before command extraction. The initial capture passed in 3.32 seconds
(`/private/tmp/eldiron-tool-panel-baseline.log`). Tests define fixed fixtures
at widths 360 and 640, height 360, with the existing BlackBlue HUD theme.

The reference adapter must preserve every RGBA byte. Tests never regenerate
expected images. Native comparisons exclude only actual reference/native glyph
boxes and compare every remaining RGB pixel within one encoded unit; text
metrics and antialiasing equivalence remain open. These component fixtures do
not establish complete Creator workflow or visual acceptance.
