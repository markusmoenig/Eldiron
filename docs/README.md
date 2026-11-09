# Eldiron documentation

The Docusaurus site uses `docs/` for current guides, `blog/` for dated posts and
`src/pages/` for landing pages. The sidebar is generated from guide front matter.

Current MZ entry points:

- [Procedural Materials](docs/building_maps/procedural_materials.md): row editing,
  map composition, saved tiling, images, animation and global material settings.
- [Appearance](docs/characters_items/appearance.md): ruleset outfits, item states,
  icons, worn/world models and shared palette bindings.
- [Rigging](docs/characters_items/rigging.md): full-viewport model, skeleton,
  attachment and animation editing; bitmap avatars remain a separate path.
- [MZ Recipes on Geometry](docs/building_maps/mz_geometry_recipes.md): experimental
  selection-driven construction and Prefab integration. See the
  [status and handoff](../crates/materialz/docs/selection-recipes-status.md) for
  the implemented scope and the sandbox's 2D-first direction.
- [Documentation coverage audit](docs/ruleset_capability_audit.md#mz-documentation-coverage-2026-11-07).

From this directory, `npm run build` validates the site. Dated blog posts and
imported core documentation should retain their historical context; current
workflow changes belong in the guides. Check user-facing control labels against
Creator code, and distinguish implemented behavior from design concepts and
unverified platform/performance claims.

Current UI screenshots and the gray-interface screenshot audit are documented in
[the native capture notes](static/img/creator/README.md). Regenerate them with the
opt-in Creator capture and `scripts/update-guide-screenshots.py`; historical release
screenshots remain with their dated posts.
