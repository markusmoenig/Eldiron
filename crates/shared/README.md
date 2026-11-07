# Eldiron Shared

Shared code and common types used across the [Eldiron](https://github.com/markusmoenig/Eldiron) applications, including the creator, client, and server.

Procedural artwork is shared project data: material bundles and palette bindings,
layered map compositions, geometry/rig documents and ruleset asset references.
`ruleset_materials`, `appearance_assets` and `rusterix_utils` retain project
overrides and resolve runtime appearances; `equipment` handles stable starting
item references. `screen_package` carries screen artwork dependencies between
projects with identity and palette remapping.

See the [material](../../docs/docs/building_maps/procedural_materials.md) and
[appearance](../../docs/docs/characters_items/appearance.md) guides for authoring.

## License

Licensed under the MIT License.
