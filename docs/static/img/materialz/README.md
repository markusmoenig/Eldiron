# Native MaterialZ documentation screenshots

These PNGs are curated, authored documentation assets. They show actual Eldiron
widgets or scene rendering, with no AI header, synthetic controls or added overlay.
Full captures, temporary renders, projects and logs remain in ignored `generated/`.
Only the selected PNGs here belong in documentation commits.

From the repository root on a native development machine:

```sh
cargo test -p eldiron-creator --lib build_iso_published_controls_playground -- --ignored --nocapture
cargo test -p eldiron-creator --lib recipe_browser_and_publisher_native_dialog_review -- --ignored --nocapture
cargo test -p eldiron-creator --lib render_workspace_snapshot -- --ignored --nocapture
cargo test -p eldiron-creator --lib materialz_documentation_ -- --ignored --nocapture
ELDIRON_UI_REVIEW="$PWD/generated/mz-docs" ELDIRON_ISO_REVIEW_PROJECT="$PWD/generated/mz-iso2d-recipe-playground/MZRecipePlayground.eldiron" cargo test -p eldiron-creator --lib mesh_selection_panel_keeps_map_and_exposes_clickable_edit_and_height_controls -- --nocapture
python3 docs/scripts/update-materialz-screenshots.py
```

The playground requires the earlier generated terrain example; its builder and
`crates/shared/examples/iso_2d/README.md` describe the input locations. The terrain
paint pair comes from the existing terrain-paint native review harness under
`generated/iso-terrain-paint/`. `render_workspace_snapshot` writes its full native
capture to `/tmp/eldiron-procedural-workspace.png`.

The curation script contains the exact source paths and crops. Cropping removes
unused map space and surrounding chrome to keep controls readable. No pixel
content is synthesized, recolored or replaced. Check the resulting images at
normal size after changes, and run `npm run build` from `docs/`.
