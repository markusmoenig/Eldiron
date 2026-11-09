# Official item and UI icons

Every official item has an independent, editable MZ PBR recipe at 32×32 pixels.
There are 41 items and 42 states, including the unlit torch. Construction uses
ordinary shape, color ramp, border, transform and alpha-compositing tiles.
There are no embedded images, custom shaders or item-specific runtime generators.

The library also includes seven flat navigation/intent icons: `ui_preferences`
(brass cog), `ui_equipment` (breastplate), `ui_inventory` (backpack),
`ui_spellbook` (open book), `ui_actions` (command scroll), `look` (eye), and `use`
(glove). These follow the same editable 32px, palette-bound MZ workflow and take
precedence over the bundled PNGs in runtime command buttons. Edit their material
tiles in the Tile Picker. They retain their flat UI projection in every camera.

Each `palette.json` maps tile IDs and color parameter names to **project palette
indices**. These bindings are authoritative; RGB values in editor/export caches
are derived. Edit colors with the palette controls in Creator, or edit the indices
here. All recipe color controls, including outlines and ramps, have bindings.
The library supports smaller and larger project palettes through the normal
palette switching/remapping workflow.

Use an item's **Appearance → Icon On** or **Icon Off** control in the project browser. Edits are
stored in the project, participate in undo and survive ruleset sync/save/reopen.
The standalone files in this folder define new-project defaults; they do not
replace a project's authored overrides. Icon artwork and worn geometry/surfaces
are separate editable compositions using the same project palette.

`torch/off.material` and `torch/off.palette.json` define the unlit state. The
lit recipe uses Loop Flow and exports four frames over one second. Inventory
and world-state tiles use the same frames at the normal animation tick, with
palette-derived color and packed material maps. The flame has emission; the
unlit torch does not.

Export individual PNGs, the full sheet and an index (plus torch animation frames):

```sh
cargo run -p eldiron-creator --example ruleset_icons -- /tmp/eldiron-item-icons
```

The optional second argument tests a smaller palette, e.g. `16` or `32`.
PNG files are previews; the material recipes and palette bindings are the source.

| Group | Editable recipe | Palette bindings |
| --- | --- | --- |
| weapons | [training_sword](training_sword/icon.material) | `training_sword/palette.json` |
| weapons | [hand_axe](hand_axe/icon.material) | `hand_axe/palette.json` |
| weapons | [novice_mace](novice_mace/icon.material) | `novice_mace/palette.json` |
| weapons | [hunting_bow](hunting_bow/icon.material) | `hunting_bow/palette.json` |
| weapons | [training_spear](training_spear/icon.material) | `training_spear/palette.json` |
| armor | [padded_armor](padded_armor/icon.material) | `padded_armor/palette.json` |
| armor | [cleric_vestments](cleric_vestments/icon.material) | `cleric_vestments/palette.json` |
| armor | [leather_vest](leather_vest/icon.material) | `leather_vest/palette.json` |
| armor | [chain_shirt](chain_shirt/icon.material) | `chain_shirt/palette.json` |
| armor | [round_shield](round_shield/icon.material) | `round_shield/palette.json` |
| armor | [ritual_censer](ritual_censer/icon.material) | `ritual_censer/palette.json` |
| armor | [sunward_charm](sunward_charm/icon.material) | `sunward_charm/palette.json` |
| clothing | [linen_shirt](linen_shirt/icon.material) | `linen_shirt/palette.json` |
| clothing | [wool_trousers](wool_trousers/icon.material) | `wool_trousers/palette.json` |
| clothing | [leather_shoes](leather_shoes/icon.material) | `leather_shoes/palette.json` |
| containers | [small_bag](small_bag/icon.material) | `small_bag/palette.json` |
| containers | [loot_corpse](loot_corpse/icon.material) | `loot_corpse/palette.json` |
| ammunition | [wooden_arrows](wooden_arrows/icon.material) | `wooden_arrows/palette.json` |
| reagents | [blessed_herb](blessed_herb/icon.material) | `blessed_herb/palette.json` |
| reagents | [moonwater](moonwater/icon.material) | `moonwater/palette.json` |
| reagents | [consecrated_oil](consecrated_oil/icon.material) | `consecrated_oil/palette.json` |
| reagents | [warding_salt](warding_salt/icon.material) | `warding_salt/palette.json` |
| reagents | [ember_bead](ember_bead/icon.material) | `ember_bead/palette.json` |
| materials | [green_wood](green_wood/icon.material) | `green_wood/palette.json` |
| materials | [feather](feather/icon.material) | `feather/palette.json` |
| materials | [wild_herb](wild_herb/icon.material) | `wild_herb/palette.json` |
| materials | [moonleaf](moonleaf/icon.material) | `moonleaf/palette.json` |
| materials | [sun_shard](sun_shard/icon.material) | `sun_shard/palette.json` |
| materials | [grave_dust](grave_dust/icon.material) | `grave_dust/palette.json` |
| materials | [ember_resin](ember_resin/icon.material) | `ember_resin/palette.json` |
| resources | [wild_herb_node](wild_herb_node/icon.material) | `wild_herb_node/palette.json` |
| resources | [green_wood_node](green_wood_node/icon.material) | `green_wood_node/palette.json` |
| resources | [bird_nest_node](bird_nest_node/icon.material) | `bird_nest_node/palette.json` |
| resources | [moonleaf_patch](moonleaf_patch/icon.material) | `moonleaf_patch/palette.json` |
| resources | [sunstone_outcrop](sunstone_outcrop/icon.material) | `sunstone_outcrop/palette.json` |
| resources | [old_grave](old_grave/icon.material) | `old_grave/palette.json` |
| resources | [resinous_stump](resinous_stump/icon.material) | `resinous_stump/palette.json` |
| currency | [copper_coin](copper_coin/icon.material) | `copper_coin/palette.json` |
| currency | [silver_coin](silver_coin/icon.material) | `silver_coin/palette.json` |
| currency | [gold_coin](gold_coin/icon.material) | `gold_coin/palette.json` |
| tools | [torch](torch/icon.material) | `torch/palette.json` |
