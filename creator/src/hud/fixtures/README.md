# Common HUD reference images

These eighteen 900x160 RGBA8 captures came from the original `Hud::draw` at
checkpoint `c2e0bb8d`, before command extraction. The isolated baseline capture
passed in 2.64 s (`/private/tmp/eldiron-hud-reference-baseline.log`).

`frame_tests.rs` defines nine HUD modes in both 2D and orbit, fixed style colors
and MOVE/SIZE captions, a raw 63x29 translucent image scaled to 40px, a 3x3
light overlay, coordinates, progress, grid and gizmo controls. The reference
adapter must preserve every RGBA byte; tests never regenerate expected images.

Native comparison covers all non-text pixels within one RGB unit, masking only
the union of actual reference/native glyph boxes plus one coverage pixel. At
least 90% of each fixture must remain compared. The opaque window ignores source
display alpha without changing source/export bytes. Full text spacing/raster/AA
parity remains open, so these fixtures do not establish whole-HUD or Creator
visual acceptance. Glyph background and pixel-edge sampling have separate GPU
coverage/math tests, including fractional densities and overlapping glyph boxes.
