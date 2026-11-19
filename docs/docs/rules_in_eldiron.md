---
title: "Rules In Eldiron"
sidebar_position: 6.5
---

This page explains how the official rules are applied inside Eldiron.

For the player-facing rulebook, see [Official Rules](./official_rules). This
page is about storage, embedding, project selection, Creator behavior, runtime
resolution, project-owned rules, and rule testing.

## Source Of Truth

The official rules are compiled from `crates/ruleset/rulesets/eldiron/v1/rules.graph.json`.
A project selects its bundled ruleset in Game / Settings. Creator, local play
and the dedicated host resolve the same definitions. Older archives may contain
rules snapshots. Loading ignores them, and saving omits them.

The split TOML files beside the bundled graph are reference fixtures used for
verification. Change the canonical source to publish changes to the official rules.

## Internal Ruleset Maintenance

The internal rules editor is a temporary workbench available in maintainer builds.
Its drafts and checkpoints live in memory. The shared catalogue keeps stable IDs
for classes, recipes and grouped items. A game's content and exposure are authored
through its world, character options, resource placements and behavior graphs.

Global definitions own gameplay policy. Character and item configuration graphs select those definitions and author entity-specific settings. Timing units are explicit per subsystem: existing action timing uses seconds, world scheduling commands use in-game minutes, and world resource recovery/expiry use simulation milliseconds.

## Locales And Assets

**Game / Locales** continues to merge project text with bundled locale defaults.
Project assets can replace bundled assets by lookup name. Project avatars named
`humanoid`, `orc`, or `skeleton` replace the respective bundled artwork.
Explicit character/item presentation still takes precedence over default ruleset
presentation.

## Conversion Tests

The bundled source is compiled and compared against the frozen split TOML
reference for values, definitions, list order, and scalar types. Editor tests
also cover field and nested-list edits, save/reload, undo, and recovery. Runtime
regressions exercise resolved rules, including a robot sandbox with a custom
resource action and no races, classes, professions, spells, or progression.

## Character Defaults

Identity defaults are optional ruleset references:

```toml
[identity.defaults]
race = "Human"
class = "Warrior"
```

When present, they must name declared race/class entries. When absent, Eldiron
does not invent an official race or class: a classless sandbox stays classless,
including its start UI and action-bar slot resolution. An explicit character
race or class still wins over the default.

When a character starts, ruleset defaults are applied in this order:

1. global attribute defaults
2. default race and class when the character has none
3. race defaults
4. class defaults
5. class starting loadout, unless the character defines explicit startup items

Character attributes identify the concrete character and store runtime state.
They should not redefine rules that already live in the official ruleset.

For example, a minimal character can set:

```toml
[attributes]
race = "Human"
class = "Warrior"
```

The runtime can then apply Human and Warrior defaults from the effective
ruleset.

For settlement NPCs, keep combat identity and economic role separate:

```toml
[attributes]
race = "Human"
class = "Citizen"
profession = "Blacksmith"
```

`Citizen` gives the NPC a civilian baseline. `profession` is available for shop,
service, crafting, training, and dialogue rules without turning every merchant
or smith into a combat class.

Professions are role labels, not hard crafting caps. Recipe access is gated by
ruleset skills such as `fletching`, `herbalism`, `alchemy`, `ritualism`, or
`restoration`. A character can carry skill points with attributes like
`skill_fletching = 25`, while simple recipes can require `0` skill and be
available immediately. Recipes can also require known spells, such as
`moonwater` requiring `minor_heal`.

If a character does not define `start_equipped_items`,
`startup_equipped_items`, or `add_equip_items`, the class loadout supplies
equipped weapons, armor, and clothing. If a character does not define
`start_items`, `startup_items`, or `add_items`, the class loadout supplies its
starting inventory.

Explicit character startup item attributes always override the class loadout.
`start_active_items` can name any item in either explicit startup list when the
game needs it to spawn active. The item still receives its normal ruleset
`active` event. State-mapped world and UI visuals update automatically, while
light, durability, and other behavior remain defined by the item rather than
the character.

Rulesets assign engine-facing meaning through optional semantic attribute
roles. The official mapping is:

```toml
[attributes.roles]
health = "HP"
max_health = "MAX_HP"
level = "LEVEL"
experience = "EXP"
weapon_damage = "DMG"
armor = "ARMOR"
```

A different ruleset can use names such as `VITAL`, `RANK`, `HARM`, and `WARD`;
damage, healing, respawn, XP, equipment summaries, graphical UI placeholders,
and terminal stats follow the mapping. The role values must reference
attributes declared by the same ruleset. A classless sandbox may omit
progression roles, in which case Eldiron does not invent `LEVEL` or `EXP`.

Class resource growth uses a generic list rather than HP/MP-specific fields:

```toml
[classes.Cleric.progression.level]
resource_gains = [
  { attribute = "HP", maximum_attribute = "MAX_HP", per_level = 5 },
  { attribute = "MP", maximum_attribute = "MAX_MP", per_level = 3 },
]
primary_attribute_gain = 1
```

Custom resources use the same shape. The current and maximum attributes grow
together while an explicitly authored current value—such as an injured
character's HP—is preserved.

Action costs use the same generic attribute ids:

```toml
[actions.focus_bolt]
kind = "spell"
cost = { FOCUS = 4 }
result = { damage = "spells.focus_bolt.damage" }
```

All typed resource costs are checked before the cast and committed by attribute
name. `MP` has no privileged runtime branch.

Class unlock tables are the sole gameplay owner of when abilities and spells
become known:

```toml
[classes.Warrior.unlocks.level_1]
abilities = ["basic_attack", "guard"]

[classes.Warrior.unlocks.level_2]
abilities = ["power_strike"]

[classes.Warrior.unlocks.level_10]
abilities = ["executioner_strike"]
```

Do not duplicate these lists on the class or in `starting_loadout`; the latter
owns items only. The runtime materializes every unlock at or below the
character's level, the server uses the same table for authorization, and the
action UI uses it to explain future unlock levels. Validators reject unlocks
outside `progression.level.max_level`, unknown references, unlocked definitions
without an action, and broken `rules.*` action-bar commands.

Level thresholds may be explicit:

```toml
[progression.level]
max_level = 10

[progression.xp_table]
level_2 = 100
level_3 = 250
```

or formula-driven:

```toml
[progression.level]
max_level = 30
xp_for_level = "level * level * 100"
```

`max_level` is authoritative for both forms. Runtime XP grants cannot advance
past it, XP-table rows above it are validation errors, and a classless game may
omit `classes` and the entire `progression` module without warnings.

## Intent Rules

Common intent policy belongs in the effective ruleset.

An action can bind an engine input intent. The binding is case-insensitive and
must be unique:

```toml
[actions.strike]
kind = "attack"
intent = "attack"
target = "hostile_or_neutral_entity"
range = "weapon"
cooldown = 1.0
result = { damage = "weapon" }
```

The action id may be anything. The action bound to `attack` supplies ordinary
and follow-attack requirements, damage, range, ammunition, and cooldown, so a
custom ruleset does not need an action named `basic_attack`.

Action target kinds describe who or what the action can affect:

- `hostile_entity`: hostile targets only
- `hostile_or_neutral_entity`: hostile and neutral targets, but not friendly targets
- `friendly_entity`: friendly targets only
- `friendly_or_self`: friendly targets or the acting character
- `any_entity`: any character target
- `ground_item`: a nearby item on the ground
- `resource_node`: a ruleset resource item such as a herb or wood node
- `self`: the acting character

The runtime resolves the target disposition from race relations and reputation.
Reputation defaults to `0`, which means normal: keep the base race relation.
Rules should use structured keys that tools can validate.

## Derived Stats

Rulesets may calculate an effective attribute without storing the calculated
result on every character:

```toml
[derived_stats.POWER]
formula = "base + floor(max(0, INT - 10) / 4) + floor(level / 5)"
minimum = 0

[derived_stats.MAX_MP]
formula = "base + WIS"
minimum = 0
```

`base` is the recipient's saved value for the derived stat. `level` reads the
ruleset-configured level attribute. Other identifiers resolve through the same
effective-attribute path, so formulas can depend on ordinary attributes or
other derived stats. Dependencies are cycle-validated; the runtime also has a
cycle guard.

Formula syntax supports `+`, `-`, `*`, `/`, comparisons, `&&`, `||`,
parentheses, and `min`, `max`, `clamp`, `abs`, `floor`, `ceil`, and `round`.
Optional `minimum` and `maximum` fields clamp the formula result.

The server uses effective values for combat formulas, numeric action
requirements, `maximum_attribute` action clamps, healing, resource
regeneration caps, and respawn health. The action UI evaluates the same
formula. Conditions modify dependencies before they enter a formula and then
modify the resulting derived stat. Nothing writes the calculated result back
to the saved base attribute.

## Conditions

Conditions are reusable timed or persistent state. Actions apply or remove
them, while the condition definition owns duration, stacking, tags, trait
immunities, numeric attribute modifiers, periodic effects, and visual phases:

```toml
[conditions.guarded]
name = "Guarded"
duration = 2.0
stacking = "refresh"
max_stacks = 1
tags = ["stance", "beneficial"]
immune_traits = []
modifiers = [{ attribute = "ARMOR", add = 2 }]

[conditions.weakened]
duration = 4.0
stacking = "stack"
max_stacks = 3
modifiers = [
  { attribute = "POWER", add = -0.5, multiply = 0.9, minimum = 1 },
]

[conditions.guarded.fx.apply]
preset = "hit_burst"

[conditions.guarded.fx.active]
preset = "holy_glow"

[conditions.poisoned.periodic]
interval = 1.0
initial_delay = 1.0
effects = [
  { damage = 2, damage_kind = "poison" },
  { resource = "STAMINA", add = -1, minimum = 0 },
]

[conditions.poisoned.fx.tick]
preset = "poison_burst"

[actions.guard]
kind = "stance"
target = "self"
cooldown = 3.0
result = { apply_condition = "guarded" }
```

Valid stacking policies are `replace`, `refresh`, `stack`, and `ignore`.
`max_stacks` may exceed one only for `stack`. A zero duration persists until
removed. The runtime exposes `conditions`, `condition_<id>_remaining`, and
`condition_<id>_stacks` on the entity for UI and scripts. Active periodic
conditions also expose `condition_<id>_tick_remaining`; applying-source
identity is stored as `condition_<id>_source`. Periodic effects may deal typed
damage, heal, or add to a numeric attribute/resource with optional minimum and
maximum bounds. Their values scale with the active stack count.

Static modifiers accept `add`, `multiply`, `minimum`, and `maximum`; at least
one operation is required. Evaluation is independent of TOML table order:

1. sum every `add × stacks`
2. multiply by every `multiply ^ stacks`
3. apply the strongest minimum and strongest maximum

Combat formulas and numeric action requirements read this effective value
without overwriting the entity's saved base attribute; the action UI uses the
same calculation. A multiplier must be non-negative, and a modifier's minimum
cannot exceed its maximum. If separately authored active bounds conflict, the
strongest maximum is applied last.

Condition FX stages are `apply`, `active`, `tick`, and `remove`. The `active`
stage is a replicated persistent particle emitter that follows the affected
entity until the condition ends. Each other stage is a one-shot ruleset FX
preset. An explicit `[conditions.<id>.fx.<stage>]` wins. If it is absent, the
runtime may use `[fx.condition_fallbacks].<stage>`; if that mapping is also
absent, no particle is required.

Scripts may observe `condition_applied`, `condition_tick`, and
`condition_removed`. Their payload contains applying source id in `x`, stack
count in `y`, remaining seconds in `z`, and the condition id as the string.

These mirrored attributes are the condition save contract. Because they live
on the serialized entity, active ids, stacks, remaining duration, periodic
phase, and stable source `creator_id` survive a map save/load. Timers pause
while the region is unloaded. On restoration, stale or expired definitions are
dropped, the source is resolved to its new runtime id (or falls back to the
affected entity), and only the persistent `active` FX is rebuilt. Transient
ruleset FX items are never restored, and restoration does not replay lifecycle
events or one-shot `apply` particles.

## Optional Invocation Schemes

Ruleset actions can have optional token-sequence bindings. The official example
defines the available words separately from the action:

```toml
[invocation_schemes.words_of_power]
kind = "token_sequence"
tokens = ["LO", "VI", "FUL", "YA", "IR", "SAR"]
separator = " "
max_tokens = 4
case_sensitive = false

[actions.minor_heal]
invocations = [
  { scheme = "words_of_power", sequence = ["LO", "VI"] },
]
```

A screen can assemble tokens in any style and submit:

```toml
command = "intent.invoke:words_of_power:{UI.spell.runes}"
```

The server resolves the phrase to the bound action before normal targeting and
execution. This keeps word/rune interfaces, icon action bars, hotkeys, and text
commands interchangeable. Invocation tokens do not require icons.

## Item Templates

Ruleset items are gameplay definitions. Creator still needs real project item
templates so users can drag items onto the map.

Creator therefore syncs ruleset-backed item templates from ruleset definitions
when a project is opened or created.

For example, a ruleset entry like:

```toml
[items.weapons.training_sword]
name = "Training Sword"
description = "A blunt wooden practice sword used for early drills and safe sparring."
category = "sword"
slot = "main_hand"
rarity = "common"
icon = "training_sword"
visual_template = "sword_diagonal"
```

becomes a normal project item template tagged with:

```toml
[attributes]
ruleset_path = "items.weapons.training_sword"
ruleset_kind = "weapon"
ruleset_id = "training_sword"
on_look = "A blunt wooden practice sword used for early drills and safe sparring."
```

Creator creates missing ruleset-backed items and refreshes existing
ruleset-backed items whose `ruleset_path` still points to the official item.
Custom project items remain separate project assets.

The official item library supplies editable **32×32 MZ PBR icon recipes** in
`assets/procedural_icons/`, with companion `palette.json` bindings. World models
live in `assets/geometry/`; garment and item surfaces use `assets/materials/`.
These recipes and project palette indices are the editable source. Generated
color/material frames are render data, not a replacement for the recipe.

Open **Appearance** under a project item to edit **Icon On**, **Icon Off**,
**Model On**, **Model Off**, worn geometry and referenced materials. On is the
default; a separate inactive state is optional. Edits to a shared asset affect
its references, and project overrides survive ruleset synchronization. The
world Tile Picker excludes the internal ruleset icon/material swatches.
See [Appearance](characters_items/appearance) for the complete workflow.

Bitmap icons remain supported through `[icons]` and
`assets/icons/<id>/<state>/<frame>.png`, including explicit project bitmap
state overrides and semantic fallback artwork. Official action and local
interface-command icons still use that PNG library. Their artwork should not
be described as editable MZ item recipes. The optional PNG-state `material.png`
remains reserved; MZ-generated PBR maps use the procedural bake path instead.

Custom actions and items may share any semantic icon id, so new content does
not require one-off artwork. Official actions deliberately use their own ids
to support the artist workflow. Action icons resolve an explicit `ui.icon`, then an
icon matching the required ability, required spell, or action id, then
`[ui.action_icon_fallbacks]` by `healing`, `condition`, action `kind`, and
`default`. For items, an authored item-id state PNG wins before the explicit
semantic `icon`/`icon_template` and `[ui.item_icon_fallbacks]` by
`ruleset_kind` and `default`. Ruleset-backed project item templates receive the
resolved fallback during sync, but it is only used when their own artwork is
unavailable.

Local interface commands use `[ui.commands.<name>]` for ruleset-themed
presentation without pretending to be gameplay actions. For example,
`ui.spellbook` resolves `[ui.commands.spellbook].icon`, while `ui.actions`
resolves `[ui.commands.actions].icon`. Their authoritative PNGs use the same
`assets/icons/<id>/on/0.png` layout and can be replaced by an artist like any
other bundled icon.

Action particle stages work the same way. Explicit
`[actions.<id>.fx.<stage>]` data wins; otherwise
`[fx.action_fallbacks.<semantic-role>].<stage>` may supply a shared preset.
Healing and condition roles precede the action kind. Attack, spell, stance, and
other mappings are authored by the ruleset rather than hardcoded by official
action id. These fallback tables are optional, and all referenced icon and FX
preset ids are validated.

Ruleset-backed item templates can also carry behavior, authoring text, tile
bindings and lights. The torch's behavior toggles `active`; its independently
editable MZ On/Off icons and models supply the corresponding appearance. The lit
icon retains four-frame animation, and its 3D recipe can attach particle effects.
Light switching, durability and fuel consumption remain item behavior/rules.
The default official torch drains condition in game minutes while active and
is removed when it reaches zero.

For bitmap-state mappings, this behavior is generic. When an item has a boolean `active` attribute and
defines `off_tile_id` and/or `on_tile_id`, changing `active` automatically
updates its world `source` from the matching available mapping. A mapping may
use a tile UUID, tile alias, or palette index. If the requested state has no
mapping, Eldiron preserves the current source. Explicit bitmap icon frames are
stored separately from their source tile. MZ state recipes use the Appearance
workspace and their procedural bindings instead.

Ruleset item ids are stable. Startup loadouts can reference `training_sword` or
`padded_armor` even when the visible item name is `Training Sword` or
`Padded Armor`.

## Equipment Policy

Slots, category conflicts, and class permissions are authored in the ruleset
rather than hardcoded by the engine:

```toml
[equipment]
weapon_slots = ["main_hand", "off_hand"]
armor_slots = ["head", "torso", "legs", "hands", "feet", "shield"]

[equipment.avatar_anchors]
main_hand = "main_hand"
off_hand = "off_hand"
shield = "off_hand"

[equipment.weapon_categories.spear]
handed = "two_handed"

[equipment.armor_categories.shield]
occupies_slots = ["off_hand"]

[classes.Warrior]
allowed_weapons = ["sword", "axe", "mace", "spear", "bow"]
allowed_armor = ["cloth", "leather", "chain", "shield"]
```

The item's declared `slot` is always occupied. A two-handed weapon additionally
occupies all `weapon_slots`, and `occupies_slots` adds any category-specific
conflicts. In the example, a shield is stored in `shield` but consumes
`off_hand`, so it cannot coexist with a bow or spear.

Slot arrays are ordered and duplicate IDs are invalid. For weapon damage,
range, cooldown, script source context, and UI/terminal equipment totals,
Eldiron checks equipped weapon slots in the authored order. The first occupied
weapon slot is the primary attack source. There is no second slot list in
**Game / Settings** and the engine does not guess names such as `main_hand`;
custom names work directly.

`avatar_anchors` maps any declared slot name onto the avatar renderer's
main-hand or off-hand frame anchor. It is optional; omit it for text-only games
or equipment that should not be drawn on the avatar.

Class permission arrays are independently optional:

- missing `allowed_weapons` or `allowed_armor` means unrestricted for that
  family
- an explicit empty array means the class may equip none from that family
- clothing follows `allowed_armor`
- a classless character remains unrestricted by class, but still follows
  slots and handedness

The validator checks starting loadouts against this same policy. At runtime,
startup, drag/drop, command, and script equip operations all use one cached
policy and reject changes without losing or moving the item.

## Palette Ownership

The project owns one editable palette, initially seeded with the official
ruleset's Endesga 64 defaults. Avatars, item fallbacks, world art and game UI all
resolve indices through it. Ruleset synchronization never replaces project colors.
Use the [Palette Tool](creator/tools/palette) to edit slots, load a local palette,
or switch directly from Lospec with perceptual and shading-ramp matching.
Bundled bitmap item icons are recolored from their canonical images. Project-owned
bitmap overrides retain their original colors. Palette slots retain material and
finish metadata when their colors change.

## Visual Defaults

The official ruleset can bundle default visual assets.

The global fallback avatar reference is:

```toml
[visuals.defaults]
avatar = "humanoid"
```

The official procedural bodies are:

```text
assets/humanoid.mzgeometry.json
assets/geometry/orc.mzgeometry.json
assets/geometry/skeleton.mzgeometry.json
```

Runtime loading evaluates these editable rigs as directional sprites in 2D and
native posed meshes in 3D, using shared palette-bound materials, fitted garments
and held-item grips. The Orc has its own proportions and heavier clips; the
Skeleton has skeletal geometry rather than a copied humanoid atlas. Bitmap
`.eldiron_avatar` sources remain available for image-based avatars.

Explicit project `tile_id`, `avatar` and `avatar_id` choices still select the
character's visual source. Project geometry/material overrides are retained
across refreshes. Select race/class and equipment in character setup, then use
[Appearance](characters_items/appearance) to preview or edit the resolved body.

The official ruleset also demonstrates race traits as gameplay data rather than
engine branches. Skeleton materializes `traits = ["undead", "skeletal"]`;
the Cleric's `turn_undead` action uses a generic `target_attributes` membership
predicate and applies the ordinary particle-backed `turned` condition. Custom
races, traits, predicates, and conditions use the same path.

Ruleset items can also define `avatar_channels`:

```toml
[items.clothing.linen_shirt]
color = 2
worth = 5
avatar_channels = ["torso", "arms"]
```

When no project override, explicit tile, authored item-id icon, or semantic icon
is available, Eldiron can use the default avatar's idle front frame, extract
the requested channels, recolor them from the project palette, and use that
shape for inventory, equipped-slot, and ground-item previews. This is a
missing-art fallback, separate from the official MZ item recipes. Project-owned
bitmap overrides retain their own colors.

## Runtime Resolution

Runtime systems should use the effective ruleset, not scattered character or
item rule attributes.

Creator and the host use the selected bundled ruleset with concrete character
and item identity and state, such as race, class, level, and equipment. An online
client receives the host’s public projection of its active rules.

The practical result is that Creator, graphical clients, terminal clients, and
shared server logic all answer the same rules questions.

## Testing Rules

Rules can be tested in the terminal client:

```bash
eldiron-client-terminal rules check
eldiron-client-terminal rules check test_projects/Hideout2D.eldiron
eldiron-client-terminal rules summary
eldiron-client-terminal rules character Cleric race=Human level=2
eldiron-client-terminal rules character Ranger race=Human level=1
eldiron-client-terminal rules item training_sword STR=12
eldiron-client-terminal rules item hunting_bow DEX=12
eldiron-client-terminal rules item linen_shirt
eldiron-client-terminal rules class Warrior
eldiron-client-terminal rules recipe wooden_arrows
eldiron-client-terminal rules recipe hunting_bow
eldiron-client-terminal rules xp 5
eldiron-client-terminal rules weapon training_sword STR=12
eldiron-client-terminal rules spell fire_spark INT=12
eldiron-client-terminal rules roll items.weapons.training_sword.damage STR=12
```

The same style of command is also available in Creator's **Game / Console**:

```text
rules overview
rules validate
rules list
rules list classes
rules show items.weapons.training_sword
rules class Warrior
rules show recipes.wooden_arrows
rules show recipes.hunting_bow
rules xp 5
rules weapon training_sword STR=12
rules spell fire_spark INT=12
rules roll items.weapons.training_sword.damage STR=12
```

Use the inspector commands to browse the effective ruleset:

- `rules overview`: show active ruleset metadata and section counts
- `rules validate`: check references, rolls, XP tables, visuals, items, spells, and classes
- `rules list`: list races, classes, professions, skills, recipes, weapons,
  armor, spells, abilities, actions, conditions, and invocation schemes
- `rules list <section>`: list one section
- `rules show <path>`: show the TOML at a ruleset path

Use the calculator commands to answer balancing questions without needing to run
a full gameplay scenario.

In play, official action distances are resolved before per-character
`[intent_distance]` values. The same `attack` icon can therefore use melee
range for swords and maces, or bow range for Rangers. Directional 2D intents
scan the chosen lane up to that range, so `attack` plus a direction can select a
hostile target beyond the adjacent tile when the equipped weapon allows it.
Weapons can also declare ammunition. For example, `hunting_bow` requires
`wooden_arrows` and `ammunition_quantity = 1`; a successful weapon attack
consumes that quantity from matching inventory stacks before damage is queued.
Stackable inventory items use `quantity` for the current count and `max_stack`
for slot capacity. The same stack-counting path is used by action `consumes`
entries for reagents, materials, and future crafting inputs. For example,
`minor_heal` consumes `1 blessed_herb` and `1 moonwater` only after target,
range, MP, and effect checks pass.

Regenerating resources use top-level `resource_regen` rules. For example,
`[resource_regen.MP]` restores mana over real-time seconds, carries fractional
progress between ticks, and clamps the result to `MAX_MP`. This keeps MP
restoration in the ruleset instead of in individual behavior graphs or screen widgets.

Map-bound resources use `resource_profiles` and `resource_pools` in the same
project rules document. Authoring binds a sector, linedef or Geometry Object to
a profile; artwork alone does not create supplies. Bundled profiles cover a
multi-supply tree, an unlimited river, a finite recovering well and skeleton
remains. Extraction shares authoritative stock, tool/vessel checks and atomic
commitment across solo and hosted play. Saved identity and stock survive reload;
recovery uses running simulation time with no offline catch-up. See
[Bind world resources](./creator/authoring#bind-world-resources) for working TOML,
shared pools, death-linked remains and current limitations.

Legacy resource-item nodes remain supported separately from inventory materials.
For example,
`wild_herb_node` is a placed world item with `static = true`, `resource_id =
"wild_herb_node"`, `respawn = 300`, and `amount = 2`. Gathering it with
`gather_herbs` adds `wild_herb x2` to the actor's inventory, hides the node, and
lets it become visible again after its respawn timer. It also sends a localized
success message such as `You gather Wild Herb x2`. `green_wood_node` works the
same way for `gather_wood`, producing `green_wood x3`, while `bird_nest_node`
uses `gather_feathers` to produce `feather x2`. The same representation defines
Moonleaf Patches, Sunstone Outcrops, Old Graves, and Resinous Stumps for ritual
materials. The text command path can use these actions too:

```text
gather herbs
gather wood
gather feathers
gather moonleaf
mine sun shards
sift grave dust
tap ember resin
craft blessed herb
craft wooden arrows
craft hunting bow
craft moonwater
craft consecrated oil
craft warding salt
craft ember beads
```

When no target is named, the text command chooses the nearest visible resource
node for that action and leaves range validation to the rules action.

## Recipe Commitment

The supplied [bone production chain](./official_rules#world-to-bone-armor-production)
uses the same item identities and crafting action path in local and hosted play.
Recipe `consumes` entries naming an item with `quantity_unit = "volume"` debit
matching contents from top-level owned/equipped vessels, preserving those vessels.
Solid inputs still come from inventory stacks. All quantities are aggregated
with action item costs before commitment; insufficient materials, liquid, usable
tools, deployed stations or output capacity leave action effects unchanged.

Enforced recipe requirements use records such as
`tool = { capability = "carve_bone", condition_cost = 1 }` and
`station = { capability = "bonework", range = 2.0 }`. Stations must be visible,
usable root world items within the declared 3D range. Carrying a station does not
grant its capability. Legacy text station values remain descriptive hints.
`fixed_quality = 100` selects the bone slice's fixed quality; absent that field,
the existing skill-based quality calculation applies. Crafts are immediate;
scheduled work, reservations and liquid recipe output policies remain separate.

A repair recipe declares `repair = { family = "bone_armor", amount = 20, maximum = 100 }`
and no produced items or replacement quality. Its action uses `target = "item"`.
The existing owned-item request supplies the exact runtime item ID; the planner
checks ownership and family membership and mutates only that item's condition.
It shares material, tool, station, action-cost and retry behavior with crafting.

Equipment categories may declare `requires_positive_condition = true` and a
wear record such as `wear = { damage_kind = "physical", slot = "torso", condition_cost = 1 }`.
Wear runs after a committed event actually lowers health. Equipment contribution
eligibility is shared by combat and equipment-stat presentation. Categories
without these fields keep their earlier behavior; this does not implement the
complete universal contribution or enchantment contracts.

### Passive wearer contributions

Equipment categories can explicitly project item-local ratings into actor
attributes:

```toml
[equipment.armor_categories.bone_armor]
requires_positive_condition = true
contributions = [{ attribute = "ARMOR", item_attribute = "ARMOR" }]

[enchantments.strength]
name = "Bear's Vigor"
requires_positive_condition = true
allowed_categories = ["bone_armor"]
modifiers = [{ attribute = "STR", add = 2 }]

[items.armor.example_cuirass]
name = "Example Cuirass"
category = "bone_armor"
slot = "torso"
enchantments = ["strength"]

[items.armor.example_cuirass.attributes]
ARMOR = 4
```

Targets must be declared numeric attributes. Modifiers support finite `add`,
`multiply`, `minimum` and `maximum` values, with unique targets per enchantment.
Allowed categories are optional restrictions; positive condition is required
by default. These fields declare passive wearer behavior while equipped.

The shared evaluator calculates a saved/derived baseline, sums applicable
equipment, enchantment and stack-scaled condition additions, applies their
combined multipliers, then minimum and maximum bounds. Derived dependencies
use the same effective values. Maximum wins if contribution bounds conflict.
Intermediate fractions retain the existing numeric behavior; this slice does
not introduce a new combat rounding or quality curve.

Each result has a structured source trace. Querying it never moves an item,
changes saved attributes, spends charges or advances condition time. Explicit
`armor.<attribute>` and `equipped.<attribute>` expressions remain item-rating
queries; ordinary actor attributes include declared projections. The legacy
structured damage adapter excludes ratings already projected into that same
actor attribute, including a wielded focus's projected source bonus. Explicit
custom combat formulas retain their authored meaning.

Definition presets live in the normal item `enchantments` array. Item construction
materializes unique attachment UUIDs in `item_enchantment_instances`, a serialized
item-state attribute carried by existing saves and item deltas. Reapplying item
data preserves existing attachments. Unmaterialized legacy presets have a stable
compound identity including the actual runtime item ID, creator/template UUID,
preset position and definition ID. A template UUID alone never identifies a
contributing item instance. Trusted runtime attachment changes preserve these
explicit instance identities. An explicit empty attachment list
overrides presets after removal. Treat this state as runtime-owned data.

One item cannot repeat an enchantment definition; distinct item sources combine.
Items with enchantment state do not stack-merge. Invalid identities, references,
ratings or non-finite candidate totals reject an equipment change before items
move. Armor families can list optional `variants` in their existing covered
slots; family repair accepts those variants without replacing the five-piece
coverage list.

The supplied Bear cuirass proves passive attachment, break/repair and
ownership lifetime. The inscription recipe below adds gameplay acquisition. Charged triggers, talents,
item-local modifier stages and advanced stacking/replacement groups still need
their remaining contracts and implementation.

## Persistent learning

The canonical rules document owns `knowledge`, `teaching`, learning actions,
profession lessons and book definitions. `learning.toml` is their readable
reference fixture. Knowledge definitions own recipe grants in one direction:
a recipe with no grant is open, and any registered knowledge granting a gated
recipe provides access. Recipe skill requirements remain separate.

```toml
[knowledge.boneworking_patterns]
name = "Advanced Boneworking"
recipes = ["shape_bone_batch"]

[teaching.advanced_boneworking]
name = "Advanced Boneworking"
knowledge = ["boneworking_patterns"]
skill = "boneworking"
required_skill = 10

[actions.learn_boneworking]
name = "Learn Advanced Boneworking"
kind = "learn"
lesson = "advanced_boneworking"
target = "any_entity"
range = 2.0
```

These are maintainer references, not steps required in standard Creator.
`professions.Boneworker.teaching` and `items.books.boneworking_manual.teaching`
both name `advanced_boneworking`. The study action uses the same lesson with
`target = "inventory_item"`. Teaching needs an available, nonhostile nearby
teacher; study needs the exact owned, usable book. Generic action resource and
item costs are staged with the acquisition on a learner copy before commit.
The source book cannot also be consumed as tuition.

`classes.<id>.starting_knowledge` seeds registered knowledge once. Character
`starting_knowledge` overrides the class even when empty. A version-1
`knowledge_journal` entity attribute stores acquisition UUIDs, stable knowledge
IDs and class/character/teacher/book provenance. An initialized empty journal
also prevents reseeding. Malformed or unsupported journals are preserved and
rejected rather than reset. Definitions removed later do not erase history.

`enchantments.<id>.knowledge_grants` supplies temporary access while equipped
and eligible under that enchantment's condition policy. A grant-only
enchantment may have no numeric modifiers. Reads never create acquisitions;
permanent learning checks the journal separately, so an equipped grant cannot
prevent learning the same knowledge permanently. Craft commitment checks
current access again before consuming any inputs.

The journal uses existing saved and owner-replicated entity attributes. Source
instance IDs and creator UUIDs record provenance without requiring that source
to remain alive. Existing gameplay request sequencing protects committed
retries; this adds no new protocol message, persistence schema or arbitrary
NPC operation journal. `stats` and `{E:1.knowledge.summary}` expose the same
permanent/temporary distinction.

## Targeted enchanting recipes

`enchanting.toml` is the readable reference for the canonical Bear inscription
content. The existing craft action and staged recipe planner also mutate exact
owned targets; they do not replace them with item-template outputs.

```toml
[recipes.inscribe_bear_cuirass]
name = "Inscribe the Bear"
skill = "boneworking"
required_skill = 10
recommended_skill = 20
difficulty = 20
consumes = [
    { item = "bear_inlay", quantity = 1 },
    { item = "tree_resin", quantity = 1 },
    { item = "fresh_water", quantity = 1 },
]
produces = []

[recipes.inscribe_bear_cuirass.enchant]
definition = "strength"
items = ["bone_cuirass"]
name = "Bone Cuirass of the Bear"

[recipes.inscribe_bear_cuirass.tool]
capability = "carve_bone"
condition_cost = 2

[recipes.inscribe_bear_cuirass.station]
capability = "bonework"
range = 2.0

[actions.inscribe_bear_cuirass]
name = "Inscribe the Bear"
kind = "craft"
recipe = "inscribe_bear_cuirass"
target = "item"
```

`enchant.definition` names a registered passive enchantment; `items` is an
explicit eligible definition list. Optional `name` is an authored result label;
omitting it preserves a custom item name. It is independent of the numeric
effect's identity or description. The default changes the supplied display
names to **Bear's Vigor** and **Bone Cuirass of the Bear**, retaining the existing
`strength` and `strength_bone_cuirass` identifiers. Saved instances are not
automatically renamed or migrated. A newly inscribed cuirass retains its
original `bone_cuirass` definition ID, rather than becoming the preset variant.

The resolver validates target/enchantment references, category compatibility,
unique eligible IDs and exact-item action targeting. A mutation cannot combine
repair, replacement outputs or a quality reset. It cannot consume its target
definition as either a recipe input or action cost. Runtime also requires a
single owned, usable target and rejects duplicate/invalid attachments.

Knowledge, skill, inputs, liquid contents, tool and station requirements use
the normal recipe checks. Attachment, optional name, debits, wear and training
are staged together. Equipped prospective values are checked for finite totals
before commit. Failed operations leave all staged changes uncommitted. Runtime
item/attachment identity, ordinary item deltas and durable player receipts are
reused without new protocol variants or persistence schemas.

`knowledge.bear_inscription` grants both `carve_bear_inlay` and
`inscribe_bear_cuirass`; the Inscriber lesson or reusable manual teaches that
knowledge at Boneworking 10. Inlay production uses a bone plate, tree resin and
water, so every ingredient traces to existing world resources. Gameplay
disenchanting, charge consumption, recharging and triggered effects remain
separate extensions.

## Crafting journal inspection

The shared `JournalModel` indexes registered crafting actions by recipe and
craft, filters them by current recipe knowledge and supplies case-insensitive
search, bounded pages, acquisition explanations and exact owned-target choices.
Knowledge availability is cached independently of movement; inspection reports
are invalidated by the actor, station, rules and pause snapshot. It is separate
from the character's saved acquisition journal.

`RecipeInspector` runs the authoritative recipe planner on a private scratch
context. It reconstructs the owning replica's cooldowns and condition stacks,
stages input and liquid debits, output capacity, tool wear, mutation, quality,
training and prospective equipped values, then discards that result. Scratch
item/attachment identities never leave inspection or enter a save, receipt,
runtime allocator or network request. Inspection does not advance simulation.
The UI receives only readiness, a reason, ingredient counts and result text.

Owned-target compatibility and action requirements are shared with real recipe
execution. Exact target choices include both runtime item ID and creator UUID;
loss or replacement requires an explicit new choice. A client preview is
advisory. The host always revalidates current state and commits the existing
craft transaction atomically.

The GPU HUD uses retained text, sources, catalogue capture and existing
action-bar assignment. Recipe cards select details; a separate operation button
submits a self/entity or exact owned-item action. Changed requirements between
press and release invalidate that captured operation. Craft and target menus
are paged, and the Knowledge tab separates permanent records from active item
grants. No inventory mutation occurs in the presentation controller.

`ui.journal` opens the floating journal; `ui.crafting` and `ui.knowledge` are
aliases. The default Creator toolbar includes it, and existing Actions and
Spellbook panels offer a Journal button. Its supplied icon reuses native MZ
catalogue artwork. Custom screens and Entity Nodes can bind `ui.journal` using
the normal command system.

Known crafting actions can be pinned without appearing in a class's starter
action bar. Other action assignment policies remain unchanged. Pins use the
existing `command_slot_*` attributes and `CommandSlot` request, with normal
character persistence and owner replication. Pinning never bypasses action,
knowledge or recipe checks. Public stations expose only capability and
condition inspection fields in addition to existing public item data; private
stock, container contents and other opaque properties retain existing policy.

Focused coverage includes planner/preview parity and read-only guarantees,
knowledge/source lifetimes, large-list geometry, native input capture, a native
GPU rendering/retention check and one owner-replica solo/dedicated pin, craft,
stale-preview, save/reconnect and durable-retry scenario.

## Containers

Item containers are normal ruleset item templates with `container = true` and
`container_slots`. The first official container is `small_bag`, a takeable
six-slot pouch.

Container UI is ruleset-driven, not screen-driven. Items can select a
`container_template`, and the runtime opens a floating draggable panel. The
panel can be closed with Escape or its close button. Inventory items can be
dragged into the panel, and items inside the panel can be dragged back to
inventory slots, equipment slots, or the map. Clicking an item inside an open
container transfers it to the first free player inventory slot. It is drawn
procedurally when no tile skin is supplied:

```toml
[ui.container_templates.bag_small]
mode = "procedural"
columns = 3
rows = 2
slot_size = 32
gap = 4
padding = 8
title = true

[items.containers.small_bag]
container_template = "bag_small"
```

Template tile fields can be supplied under
`[ui.container_templates.<id>.tiles]` for `top_left`, `top`, `top_right`,
`left`, `center`, `right`, `bottom_left`, `bottom`, `bottom_right`, and `slot`.
If those fields are absent, the procedural renderer is used.

The current text command path can move top-level inventory items into and out
of an inventory or visible world container, and can open a container floater:

```text
open small bag
put wild herb in bag
take wild herb from bag
```

Stackable items merge inside containers. When a dead character script calls
`drop_items("")`, the official rules create a lootable corpse container instead
of placing every carried item directly on the map. The corpse uses the normal
container UI and can be opened with `open <name>` or by clicking it. Once the
corpse is empty, the tombstone disappears when `despawn_when_empty = true` in
`[loot.corpse]`. Non-empty corpses use `despawn_seconds`. If the corpse belongs
to a respawning NPC, the timer is shortened by
`despawn_before_respawn_seconds`, so the body disappears shortly before the NPC
returns.

NPC respawn is also rules-driven. `[respawn.npc]` defaults to enabled, restores
NPC health to full, restores startup loadout and behavior state, and removes
the NPC corpse on respawn. Player characters are excluded from this automatic
path; their death and resurrection flow stays in the player script. For one
NPC, use `respawn_seconds = 120` to change the delay or `respawn = false` to
keep it dead.

## Economy

The official economy is part of the project rules document. Runtime wallets store one integer
base amount. In v1 the base is copper:

```toml
[economy]
base = "copper"

[economy.starting_wealth]
player = 50

[economy.currencies.copper]
symbol = "c"
value = 1

[economy.currencies.silver]
symbol = "s"
value = 10

[economy.currencies.gold]
symbol = "g"
value = 100
```

Item `worth`, shop prices, rewards, and `wealth` overrides are measured in base
units. The UI can format the same balance compactly, so `125` displays as
`1g 2s 5c`. New player characters start with `50` base units, displayed as
`5s`, unless their character attributes define an explicit `wealth`. Use
`{PLAYER.MONEY}` for formatted display and `{PLAYER.FUNDS}` when raw base units
are needed for tests or logic.

Currency items are ordinary ruleset-backed item templates marked
`monetary = true`. Taking them adds their base value to the wallet instead of
placing the item in inventory.

To make a money loot item with a specific value, set the currency and amount on
the item instance or template:

```toml
[attributes]
monetary = true
currency = "silver"
amount = 5
worth = 50
```

## Recipes

Recipes are part of the project rules document, alongside items, actions, skills,
professions, and spells. The official set includes immediate
crafts and a multi-stage ritual economy:

- `wooden_arrows`: consumes `green_wood x1` and `feather x2`, produces `wooden_arrows x10`
- `blessed_herb`: requires `minor_heal`, consumes `wild_herb x1`, produces `blessed_herb x1`
- `hunting_bow`: recommends `skill_fletching = 25`, consumes `green_wood x3`, produces `hunting_bow x1`
- `moonwater`: distills `moonleaf x2` into `moonwater x2`
- `consecrated_oil`: combines `blessed_herb x1` and `sun_shard x1`
- `warding_salt`: purifies `grave_dust x2` with `sun_shard x1`
- `ember_beads`: shapes `ember_resin x2` into `ember_bead x3`
- `ritual_censer`: invests three material families in reusable spell equipment
- `sunward_charm`: turns advanced ritual materials into reusable resistance

Recipe execution consumes input stack quantities and merges output stack
quantities into existing inventory slots when possible. This is the same economy
path that later shops, gathering nodes, crafting stations, and profession
services can use.

The text command path can craft known recipes by name:

```text
craft wooden arrows
craft hunting bow
craft blessed herb
craft moonwater
craft consecrated oil
craft warding salt
craft ember beads
craft ritual censer
craft sunward charm
```

Recipes can also be exposed through rules actions such as
`rules.craft_blessed_herb`, `rules.distill_moonwater`,
`rules.mix_warding_salt`, and `rules.craft_ritual_censer`. This lets screen
command slots trigger the same recipe path as text commands and scripts while
keeping recipes as the source of truth for materials, spell gates, skill
targets, and outputs.

Recipes can still use `required_skill` for hard gates, but ordinary crafting is
better modeled through output quality. `recommended_skill`, `difficulty`, and a
supporting attribute such as `DEX` or `WIS` set crafted item `quality` from
`1..100`; crafted items start at `condition = 100`. Weapon damage scales by item
quality and condition, so a new Ranger can craft immediately but starts with
rougher gear.

When `[crafting.skill_gain]` is enabled, a successful recipe also advances its
skill. The official configuration grants one point per success, plus one while
below the recipe's recommendation, and stops awarding points twenty above that
recommendation. The skill definition's `max` remains the absolute cap. This
turns repeated low-tier preparations into a real path toward gated equipment;
failed attempts never grant skill.

## Future Versioning

The project stores which ruleset version it expects.

This allows future games to request a specific ruleset:

```toml
[ruleset]
id = "eldiron.official"
version = "3.0.0"
source = "official"
```

Future versions can add or change rules while older projects keep the version
they selected. Bugfixes, localization improvements, and compatible additions can
still be shipped through bundled ruleset updates according to the selected
update policy.
