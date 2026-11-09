# Current Creator documentation screenshots

Audited on 2026-11-09. These assets are captured from the actual Creator UI with
the Brown theme, using the checked-in Hideout 2D and Stonefall Dungeon examples.
They contain no AI header, generated UI, recoloring, or added labels. The geometry
guide uses a small rectangular sector created in an in-memory project copy.
Stonefall is shown with the editor's neutral fill light at full strength and its
grid and post effects disabled for readability; the saved project is unchanged.

## Audit and replacements

All eleven legacy screenshot assets referenced by current UI guides were replaced
at their usage sites. The original files remain available for historical references.

| Guide / subject | Previous asset | Current asset |
| --- | --- | --- |
| Creator introduction | `/img/docs/screenshot.png` | `overview.png` |
| Rect painting | `building_maps/rect_tool.png` | `rect-tool.png` |
| Tile metadata | `building_maps/editing_tile.png` | `tile-metadata.png` |
| Tilesets (two guides) | `/img/screenshots/tilesets.png` | `tileset-editor.png` |
| Tile Picker | `/img/screenshots/dungeon3d_iso.png` | `tile-picker.png` |
| Character attributes | `creator/docks/attributes.png` | `entity-nodes.png` |
| Screen widgets | `screens/screens_widgets.png` | `screen-editor.png` |
| Drawing linedefs | `linedef_tool.png` | `linedef-tool.png` |
| Selecting a sector | `sector.png` | `selected-sector.png` |
| Applying a tile | `applied_tile.png` | `applied-tile.png` |
| Map HUD | `/img/docs/hud.png` | `map-hud.png` |

The documentation homepage's hero and nine feature/format images previously used
release screenshots. They now reference current
native captures: 2D, 3D, authoring, settings, Behavior Nodes, painting, Prefabs, and
the Tile Picker. The 3D geometry guide also gains a current Stonefall overview.
Captions and directly related control names were checked against these captures.

The remaining old UI screenshots occur in dated blog posts and the Foreword's
development history; those deliberately retain their original appearance. PNGs
such as `3d_tutorial*.png`, `3d_terrain*.png`, `3d_gizmo.png`, `region_settings.png`,
`script_editor.png`, `characters_items.png`, and `tileset_section.png` have no current
guide references and were not substituted with unrelated scenes. The 2D/3D game
examples in `2d_or_3d.mdx` are runtime captures rather than gray Creator UI images.
Game-player thumbnails, illustrations, logos, and dated video thumbnails are also
outside the editor UI refresh. The MZ and isometric guide images already have native
capture workflows and were retained.

## Reproduce

From the repository root on a machine with a native GPU and Python Pillow:

```sh
cargo test -p eldiron-creator --lib documentation_screenshots -- --ignored --nocapture --test-threads=1
python3 docs/scripts/update-guide-screenshots.py
cd docs
npm run build
```

The opt-in capture module is `creator/src/editor/documentation_capture.rs`.
It creates an isolated editor, loads project copies into memory, and drives the
normal widgets and renderer. It does not connect to an existing Creator session or
save changes to the source projects. Full 1440 × 960 captures stay in ignored
`generated/docs-current/`; `ELDIRON_DOC_CAPTURES` can override that capture location.
The curation script expects the default directory and records every crop explicitly.
Only selected documentation PNGs are copied here. Inspect the full captures and
curated crops at normal size after changing the UI or screenshot fixtures.
