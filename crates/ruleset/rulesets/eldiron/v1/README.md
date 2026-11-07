# Eldiron Official Ruleset v1

This is the first bundled official ruleset for Eldiron.

It defines the initial v1 gameplay baseline with explicit tables and dice-style
values: core attributes, Human and Warrior defaults, progression, damage kinds,
weapon and armor categories, starter equipment, cooldowns, abilities, spells,
actions, procedural FX presets, audio/message hooks, resource gathering,
multi-stage ritual crafting, and default humanoid visuals.

`rules.graph.json` is the production official rules source. Creator edits the
same node format through **Game / Rules**, one definition branch at a time.
Projects own a full graph and preserve their original plus recovery checkpoints.
Minimal inherited deltas and upgrade policies are not implemented yet.

The split TOML rules files are a conversion-test reference only. The runtime
loads the embedded graph and compiles it into the existing validated value tree.
`locales.toml` and bundled artwork retain their own formats.

FX presets are dedicated `/fx/presets/name` particle branches with reusable
emission, motion, lifetime, size, color, direction/gravity, spawn-area, and curve
modules. They compile directly to emitter settings, with animated Creator
previews. Runtime tests compare every official preset and stage override against
the old `fx.toml` recipe translator; the rest of the graph still has exact
value-tree conversion coverage.

Behavior policies are separate definition branches such as
`/behavior/lookout/hostile` and `/behavior/engage/default`. Entity behavior nodes
select these policies by stable ID. Dispositions, action references, distances,
and delays are validated through the same ruleset validator.

The current playable progression ends at level 10. Warrior, Cleric, and Ranger
gain their core kit at level 1 and one new action at levels 2, 4, 6, 8, and 10.
Class `unlocks.level_N` tables are the sole owner of ability/spell acquisition;
starting loadouts own items only. The same table shape extends to level 30
without a schema change.

`attributes.roles` maps the official `HP`, `MAX_HP`, `LEVEL`, `EXP`, `DMG`, and
`ARMOR` names onto generic health, maximum-health, level, experience,
weapon-damage, and armor semantics. Class
`progression.level.resource_gains` entries name arbitrary current/maximum
resource pairs, and action costs are arbitrary declared attribute maps, so
custom rulesets do not need HP-, MP-, or combat-stat-specific engine fields.

Equipment slots, category handedness, occupied-slot conflicts, and class
permissions are rules-owned and enforced through one resolved runtime policy.
Missing class permission arrays are unrestricted, while explicit empty arrays
permit none; classless sandbox actors do not acquire implicit official class
restrictions. The optional `equipment.avatar_anchors` table maps those
rules-owned slot ids onto the avatar renderer's main/off-hand anchors without
hardcoded slot aliases.

The default procedural body recipes are:

```text
assets/humanoid.mzgeometry.json
assets/geometry/orc.mzgeometry.json
assets/geometry/skeleton.mzgeometry.json
```

They share the production MZ appearance evaluator: directional sprites in 2D,
posed native meshes in 3D, fitted clothing, weapon grips and accessory sockets.
The Orc and Skeleton have their own authored bodies and motions. Bitmap
`.eldiron_avatar` assets remain supported as a separate image-based path.
Explicit character `tile_id`, `avatar` or `avatar_id` choices override the default.

Every official item has an editable 32×32 MZ icon under
`assets/procedural_icons/`, with companion project-palette bindings. World and
worn models live in `assets/geometry/`; surface recipes and their index bindings
live in `assets/materials/`. Creator's item **Appearance** workspace opens
**Icon On**, **Icon Off**, **Model On**, **Model Off**, worn models and materials.
Project overrides survive refresh and save/reopen. Internal artwork swatches are
excluded from the ordinary world Tile Picker.

The project owns one editable palette, seeded with Endesga 64 for a new official
project. MZ recipe colors, ramps and emission use its indices. Palette switches
match target colors and preserve slot identities; they do not introduce a second
rules-owned palette.

Action and local interface-command icons still use the PNG library:

```text
assets/icons/<id>/on/0.png
assets/icons/<id>/on/1.png       # optional animation frame
assets/icons/<id>/off/0.png      # optional inactive state
```

Bitmap item/state overrides are also supported. Bundled bitmap fallback artwork
is fitted from canonical images when adapting the project palette; explicit
project bitmap overrides retain their colors. A state-local `material.png` is
reserved for future bitmap-map authoring, separate from current MZ-generated PBR
maps.

`icons.toml` and `assets/icons/ATTRIBUTION.md` retain the upstream attribution for
the Game-icons-derived PNG subset. Its importer creates missing source artwork
without replacing artist edits:

```sh
cargo run -p eldiron-icon-builder -- crates/ruleset/rulesets/eldiron/v1/icons.toml crates/ruleset/rulesets/eldiron/v1/assets/icons
```

See the geometry and procedural icon READMEs for review/export examples. Current
Creator guides live in `docs/docs/characters_items/appearance.md`,
`docs/docs/characters_items/rigging.md` and
`docs/docs/building_maps/procedural_materials.md` at the repository root.

Definitions do not each need their own icon or particle block. The official
ruleset uses `ui.action_icon_fallbacks` and `ui.item_icon_fallbacks` for shared
semantic glyphs, plus `fx.action_fallbacks` and `fx.condition_fallbacks` for
shared procedural presets. Explicit per-definition presentation wins, and a
custom ruleset may change or omit every fallback table.

The spell economy uses a deliberately small material vocabulary. Moonleaf,
Sun Shards, Grave Dust, and Ember Resin come from placeable resource nodes.
Recipes refine them into Moonwater, Consecrated Oil, Warding Salt, and Ember
Beads. Spells consume those prepared reagents, while higher recipes can invest
them in reusable Ritual Censers and Sunward Charms. Items use editable MZ icon/model recipes; actions retain semantic PNG
fallbacks. This keeps gameplay definitions separate from the chosen artwork.

Successful recipes also exercise the optional `[crafting.skill_gain]` loop.
The official settings grant deterministic progress below a recipe's mastery
range, which lets repeated reagent preparation lead into the Censer and Charm
recipes without requiring a combat class or externally scripted skill awards.

Every official spell also binds to the optional `words_of_power` invocation
scheme. The lexicon and word meanings live in `invocations.toml`; exact
sequences remain action-owned beside costs, targeting, and reagents.

The torch demonstrates independent MZ icon/model states. Its lit icon bakes a
four-frame flame loop; its 3D recipe can attach palette-linked particles to the
model or rig. `active` selects On/Off bindings, while behavior owns light
switching and fuel use. The Appearance workspace edits the two states separately.
Bitmap-only state mappings through `on_tile_id` and `off_tile_id` remain available
for custom items.

The torch also demonstrates rules-owned durability. Its `[durability]` table
drains `condition` while `active`, measured in game minutes, and removes the
item when the condition reaches `0%`.
