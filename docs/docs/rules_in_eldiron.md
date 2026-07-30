---
title: "Rules In Eldiron"
sidebar_position: 6.5
---

This page explains how the official rules are applied inside Eldiron.

For the player-facing rulebook, see [Official Rules](./official_rules). This
page is about storage, embedding, project selection, Creator behavior, runtime
resolution, project-level rule overrides, and rule testing.

## Source Of Truth

The official ruleset lives in the `eldiron-ruleset` crate:

```text
crates/ruleset/rulesets/
  manifest.toml
  eldiron/
    v1/
      ruleset.toml
      identity.toml
      attributes.toml
      progression.toml
      combat.toml
      economy.toml
      messages.toml
      locales.toml
      equipment.toml
      fx.toml
      icons.toml
      invocations.toml
      conditions.toml
      actions.toml
      abilities_spells.toml
      races_classes.toml
      README.md
      assets/
        humanoid.eldiron_avatar
        orc.eldiron_avatar
        skeleton.eldiron_avatar
```

This location is intentional. The ruleset is not owned by Creator only, and it
must be publishable with the crate that exposes the official ruleset API. It is
available to:

- Creator
- graphical clients
- terminal clients
- shared runtime code through `eldiron-ruleset` and `eldiron-shared`
- calculators
- automatic arena tools
- tests
- documentation generators

The current built-in ruleset is `eldiron.official` version `1.0.0`.

## Compile-Time Embedding

Official rulesets are embedded at compile time by the `eldiron-ruleset` crate.

The ruleset crate includes all official v1 TOML parts with
`include_str!`, joins them into one effective official TOML source, and also
embeds the bundled `humanoid`, `orc`, and `skeleton` avatar assets.

This lets every binary built from the repository access the same official
ruleset through a package-safe crate API without each app carrying its own
private copy.

## Project Selection

A project selects its ruleset in **Game / Settings** with the top-level
`[ruleset]` section:

```toml
[ruleset]
id = "eldiron.official"
version = "1.0.0"
schema_version = "1"
source = "official"
update_policy = "compatible"
```

The section is top level because other main game settings are top level too.

Supported intent:

- `source = "official"` uses a bundled ruleset selected by `id` and `version`
- **Game / Rules** can override that official ruleset for this project
- `update_policy` describes how future compatible updates should be handled

Older projects that do not have `[ruleset]` are migrated by adding this default
section.

## Game / Rules

For official-rules projects, **Game / Rules** is the project-level override
layer. It is empty by default because new projects use the bundled Eldiron
Official Ruleset unchanged.

The default template explains this:

```toml
# Game / Rules is the project-level override layer for the official ruleset
# selected in Game / Settings.
```

During the v1 cleanup, normal gameplay definitions should move out of character
and item attributes and into the official ruleset or this project-level
**Game / Rules** override. Character and item attributes should not redefine
cooldowns, spell behavior, class permissions, intent distance, or combat math.

Ruleset timing values use seconds. Script scheduling commands such as
`notify_in`, `block_events`, patrol waits, and random-walk sleeps still use
in-game minutes because they operate on the world clock. This keeps ruleset
combat tuning separate from authoring-time world schedules.

The effective ruleset is resolved like this:

1. Read `[ruleset]` from **Game / Settings**.
2. Load the matching bundled official ruleset.
3. Merge **Game / Rules** TOML on top.
4. Use the merged result for runtime and tools.

Ruleset localizations are resolved the same way:

1. Load the bundled English locale defaults for the selected official ruleset.
2. Merge **Game / Locales** TOML on top.
3. Use project locale entries as overrides, not as a required copy of every
   ruleset message.

## Configuration And Overrides

Official ruleset projects are configured in layers.

Use **Game / Settings** to select which bundled ruleset the project follows.
Use **Game / Rules** to override ruleset TOML for this project. The override
should contain only the tables and keys that are intentionally different from
the bundled official ruleset.

Use **Game / Locales** the same way for text. Project locale entries replace
matching bundled ruleset locale keys, while missing keys continue to come from
the official locale defaults.

Project assets can also override bundled ruleset assets when they use the same
lookup name. Ruleset avatars are loaded first, then project avatars are inserted
afterwards by avatar name. This means a project avatar named `humanoid`
overrides the bundled official `humanoid` avatar automatically.

This is important for artist-edited avatar atlases. If you export the official
humanoid avatar as a PNG atlas, edit it externally, and import it back into a
project avatar named `humanoid`, all characters that use the default ruleset
avatar will use the project version. A project avatar named `Human` does not
replace the default `humanoid` avatar by name; it is used only by characters
that explicitly set `avatar = "Human"` or the matching `avatar_id`.

Explicit character and item presentation still wins over default ruleset
presentation. A character with `avatar`, `avatar_id`, `tile_id`, or `source`
does not use the fallback ruleset avatar. Setting an empty `avatar = ""` or
`tile_id = ""` is a deliberate way to prevent inherited default visuals.

## No Backwards Compatibility Requirement

The official ruleset replaces the old ad hoc rules model.

Old projects are migrated toward the new shape by:

- adding the default `[ruleset]` section when missing
- replacing old project rules with the empty **Game / Rules** override template

This is allowed because the goal is to create one coherent default ruleset
instead of preserving every old formula shape forever. The official v1 rules
should prefer explicit tables and dice-like values.

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
`active` event, so state-specific visuals, light, durability, and other behavior
remain defined by the item rather than the character.

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

Ruleset icons live in `[icons]` and are bundled as neutral PNG masks. Item
templates can set `icon = "training_sword"` as a generic fallback. Item display
still prefers explicit tiles, avatar channels, and `visual_template` pixel masks
when present, so hand-shaped pixel item icons remain the primary look.
Icons are shared semantic assets rather than one file per definition: the
official Guard action, for example, deliberately references the existing
`round_shield` icon.

Actions and items may share any semantic icon id, so new content does not
require one-off artwork. Action icons resolve an explicit `ui.icon`, then an
icon matching the required ability, required spell, or action id, then
`[ui.action_icon_fallbacks]` by `healing`, `condition`, action `kind`, and
`default`. Item icons resolve an explicit `icon`/`icon_template`, then
`[ui.item_icon_fallbacks]` by `ruleset_kind` and `default`. Ruleset-backed
project item templates receive the resolved item fallback during sync.

Action particle stages work the same way. Explicit
`[actions.<id>.fx.<stage>]` data wins; otherwise
`[fx.action_fallbacks.<semantic-role>].<stage>` may supply a shared preset.
Healing and condition roles precede the action kind. Attack, spell, stance, and
other mappings are authored by the ruleset rather than hardcoded by official
action id. These fallback tables are optional, and all referenced icon and FX
preset ids are validated.

Ruleset-backed item templates can also carry item script source, authoring text,
tile ids, and lights. The ruleset can bundle the referenced tiles too, including
animated tile frames. This is used for reusable interactive objects such as
`items.tools.torch`: the ruleset creates a normal project item template whose
`use` intent toggles `active`, swaps between the bundled unlit tile and the
bundled four-frame lit animation, enables or disables the point light, and
presents different look/use text for the on and off states. The same item also
uses ruleset durability: while `active`, its `condition` drains in game minutes,
and the default official torch destroys itself at `0%` condition.

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

The official ruleset owns the **Ruleset Palette**.

On load and ruleset sync, Eldiron resolves the effective ruleset `[palette]`
into the project's Ruleset Palette. Rules-owned visuals such as official item
icons, avatar defaults, UI color channels, and generated ruleset assets can rely
on those indices staying stable.

The editable **Art Palette** is separate. It is used for artist-authored tiles,
pixel drawing, tile graphs, palette-index geometry sources, and 3D Paint.

For ruleset-driven projects:

- Ruleset Palette changes should be made by overriding `[palette]` in **Game / Rules**.
- Art Palette changes are made with the Palette Tool and do not alter rules-owned indices.
- palette clear/import actions affect the Art Palette only.
- Art Palette material preset/finish metadata remains editable project render metadata.

## Visual Defaults

The official ruleset can bundle default visual assets.

The global fallback avatar reference is:

```toml
[visuals.defaults]
avatar = "humanoid"
```

The bundled asset lives in the ruleset directory:

```text
crates/ruleset/rulesets/eldiron/v1/assets/humanoid.eldiron_avatar
crates/ruleset/rulesets/eldiron/v1/assets/orc.eldiron_avatar
crates/ruleset/rulesets/eldiron/v1/assets/skeleton.eldiron_avatar
```

Runtime asset loading makes this available to clients. Character visuals can
still provide concrete presentation with values such as `tile_id` or `avatar`.
An explicit project visual wins over the ruleset default, and a project avatar
named `humanoid`, `orc`, or `skeleton` replaces the matching bundled default
avatar for the project. The bundled Skeleton file is a distinct copy of the
humanoid asset intended as the import target for a dedicated Skeleton atlas.

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

When no explicit item icon or tile source is provided, Eldiron uses the default
avatar's idle front frame, extracts the requested channels, recolors them from
the ruleset palette, and uses that shape for inventory, equipped slot, and
ground item previews.

## Runtime Resolution

Runtime systems should use the effective ruleset, not scattered character or
item rule attributes.

That means clients and shared runtime helpers resolve rules by combining:

- the selected bundled ruleset
- the project-level **Game / Rules** override
- concrete character or item identity/state such as race, class, level, and
  equipment

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
restoration in the ruleset instead of in individual scripts or screen widgets.

Resource nodes are separate from inventory materials. For example,
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

The official economy lives in `economy.toml`. Runtime wallets store one integer
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

Recipes live in `recipes.toml` and use the same source of truth as items,
actions, skills, professions, and spells. The official set includes immediate
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
