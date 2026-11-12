# Eldiron Creator

**Eldiron** is a cross-platform creator for classic retro role-playing games (RPGs). Its primary goal is to enable the creation of RPGs reminiscent of the 1980s and 1990s while incorporating modern features such as multiplayer support, procedural content generation, and more.

Eldiron natively supports **2D** (like Ultima 4/5), **isometric**, and **first-person** RPGs, allowing developers to craft a variety of experiences effortlessly.

The embedded MaterialZ (MZ) core adds editable procedural world materials,
shape-based map composition, ruleset item icons, clothing, models and rigs.
The same project-palette artwork supports 2D sprites and native 3D meshes.
Bitmap tiles and avatars remain supported. See the
[material guide](../docs/docs/building_maps/procedural_materials.md) and
[appearance guide](../docs/docs/characters_items/appearance.md).

| 2D · Hideout | 2D Iso · Wayfarer's Rest | 3D · Stonefall Dungeon |
| :---: | :---: | :---: |
| ![Hideout 2D in Eldiron Creator v0.96.0](../docs/static/img/creator/hideout2d-v096.png) | ![Wayfarer's Rest in 2D Iso in Eldiron Creator v0.96.0](../docs/static/img/iso/wayfarers-rest-v096.png) | ![Stonefall Dungeon first-person gameplay in Eldiron Creator v0.96.0](../docs/static/img/creator/stonefall-first-person-v096.png) |

Eldiron is open-source and licensed under the **MIT License**.

For a more detailed feature overview please visit [Eldiron.com](https://eldiron.com) or the [GitHub repository](https://github.com/markusmoenig/Eldiron).

```bash
cargo install eldiron-creator
```

## License

Licensed under the MIT License.

The native GPU migration host can be run with:

```bash
cargo run -p eldiron-creator --no-default-features --features native-host --bin eldiron-creator-native -- [PROJECT.eldiron ...]
```

It assembles the existing Creator UI through the common GPU compositor and Swash
text atlases, and uses injected GPU geometry previews with acknowledged picking.
The normal `eldiron-creator` launcher remains the default while main world/tool/
history ownership, other previews and full platform/workflow parity are completed.
The migration host's `--smoke` requires three real surface presentations; it fails
on a timeout. It does not certify physical input, all workflows or performance.
