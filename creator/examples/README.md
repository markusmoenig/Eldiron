# Native material editor prototype

Run the standalone UI fixture:

```sh
cargo run --offline -p eldiron-creator --example material_editor
```

Render at the current 380 logical pixel sidebar width, with Retina density:

```sh
cargo run --offline -p eldiron-creator --example material_editor -- \
  --snapshot /tmp/material-editor.png --width 380 --height 800 --density 2
```

The fixture supplies presentation data and applies editor commands to that data.
It does not evaluate materials, modify project files, or integrate MaterialZ.
Cards display host-supplied thumbnails when available; the fixture does not
fabricate procedural results. The fixture itself has no map or sidebar tab strip.

Drag a source onto a card, row, or new-row target. Drag a row's right terminal to
an output terminal, or start from the output. Click a connected input terminal to disconnect; dragging still connects rows.
Right-click also disconnects.
The editor fits four compact cards per line; longer operation chains wrap
inside their existing material row. Outlined spare slots accept source drops;
the empty trailing row starts with one outlined tile. Captions are centered.
Select Properties or Sources below the graph. Clicking an operation opens its
properties. Clicking a source selects it for the production Tile preview; dragging
it adds the source instead. Drag a numeric field to preview a
value and commit on release. Escape cancels the gesture. Click the source search
field to type; Backspace removes a character. Search matches source names and
categories. Touchpad scrolling follows the pointer over the rows, properties,
or sources; small deltas accumulate without scrolling the map or dock.

The production widget in `creator/src/material_editor.rs` emits stable-ID commands
and never changes the validated material document itself. The host must validate
commands through MaterialZ, record undo, and supply a fresh presentation view.
Replacing the view cancels gestures and clears pending commands, so a drag cannot
leak into a newly selected material. Numeric drags emit one final command per
gesture. The production host in `creator/src/procedural_material.rs` now connects this
widget to MaterialZ, project persistence/undo, Make Proc, a regular MaterialZ sidebar tab and a GPU
texture preview in the minimap panel. Selecting a procedural tile selects the MZ
tab automatically; other tabs restore the normal minimap. The central map remains
visible and interactive. Icon-only segmented buttons choose material output /
selected tile, the PBR output channel and single / infinite UV mode; tooltips
identify each button. The preview fills its pane with two tile rows; single mode
repeats the image, while infinite mode samples a continuous region at that scale.
Tile evaluates the selected operation or catalogue source's actual RGBA
result; it does not replace the material output or bake. Catalogue sources use
a temporary document rather than inserting a node into the authored material. Full specialized
parameter widgets and row input wiring remain follow-up work.

The core is checked in at `crates/materialz`; no external MaterialZ checkout is
needed. The real GPU integration and combined workspace snapshot can be checked with:

```sh
cargo test --offline -p eldiron-creator --lib procedural_material -- --ignored
```

This requires access to a Metal/Vulkan/DX12 adapter. The workspace snapshot is
written to `/tmp/eldiron-procedural-workspace.png` (Output) and
`/tmp/eldiron-procedural-selected-tile.png` (Tile), and
`/tmp/eldiron-procedural-source-preview.png` (catalogue source); no user project
is changed.
