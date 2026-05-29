---
title: "Rules In Eldiron"
sidebar_position: 6.5
---

This page explains how the official rules are applied inside Eldiron.

For the player-facing rulebook, see [Official Rules](./official_rules). This
page is about storage, embedding, project selection, Creator behavior, runtime
resolution, project-level rule overrides, and rule testing.

## Source Of Truth

The official ruleset lives at the top level of the repository:

```text
rulesets/
  manifest.toml
  eldiron/
    v1/
      ruleset.toml
      identity.toml
      attributes.toml
      progression.toml
      combat.toml
      messages.toml
      locales.toml
      equipment.toml
      fx.toml
      actions.toml
      abilities_spells.toml
      races_classes.toml
      README.md
      assets/
        humanoid.eldiron_avatar
```

This location is intentional. The ruleset is not owned by Creator only. It must
be available to:

- Creator
- graphical clients
- terminal clients
- shared runtime code
- calculators
- automatic arena tools
- tests
- documentation generators

The current built-in ruleset is `eldiron.official` version `1.0.0`.

## Compile-Time Embedding

Official rulesets are embedded at compile time by shared code.

The shared ruleset module includes all official v1 TOML parts with
`include_str!`, joins them into one effective official TOML source, and also
embeds the bundled `humanoid` avatar asset.

This lets every binary built from the repository access the same official
ruleset without each app carrying its own private copy.

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

## No Backwards Compatibility Requirement

The official ruleset replaces the old ad hoc rules model.

Old projects are migrated toward the new shape by:

- adding the default `[ruleset]` section when missing
- replacing old project rules with the empty **Game / Rules** override template

This is allowed because the goal is to create one coherent default ruleset
instead of preserving every old formula shape forever. The official v1 rules
should prefer explicit tables and dice-like values.

## Character Defaults

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
ruleset skills such as `fletching`, `herbalism`, or `restoration`. A character
can carry skill points with attributes like `skill_fletching = 25`, while simple
recipes can require `0` skill and be available immediately. Recipes can also
require known spells, such as `blessed_herb` requiring `minor_heal`.

If a character does not define `start_equipped_items`,
`startup_equipped_items`, or `add_equip_items`, the class loadout supplies
equipped weapons, armor, and clothing. If a character does not define
`start_items`, `startup_items`, or `add_items`, the class loadout supplies its
starting inventory.

Explicit character startup item attributes always override the class loadout.

## Intent Rules

Common intent policy belongs in the effective ruleset.

For example, the official attack rule is declarative:

```toml
[intents.attack]
allowed_dispositions = ["hostile"]
deny_message = "{system.cant_do_that}"

[intents.attack.distance]
source = "weapon_range"
fallback = 1.5
```

The runtime resolves the target disposition from race relations and reputation.
Reputation defaults to `0`, which means normal: keep the base race relation.
Rules should use structured keys that tools can validate.

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

Ruleset item ids are stable. Startup loadouts can reference `training_sword` or
`padded_armor` even when the visible item name is `Training Sword` or
`Padded Armor`.

## Palette Ownership

The official ruleset owns the game palette.

On load and ruleset sync, Eldiron copies the effective ruleset `[palette]` into
the project palette. This keeps existing painting, avatar, tile, item, and
rendering systems on one active palette instead of maintaining two palettes at
runtime.

For ruleset-driven projects:

- palette clear/import actions are disabled
- palette color picker and hex color edits are disabled
- palette material attributes remain editable project render metadata
- the palette sidebar shows only the colors present in the active ruleset

Palette changes should be made by overriding `[palette]` in **Game / Rules**.

## Visual Defaults

The official ruleset can bundle default visual assets.

The current default avatar reference is:

```toml
[visuals.defaults]
avatar = "humanoid"
```

The bundled asset lives in the ruleset directory:

```text
rulesets/eldiron/v1/assets/humanoid.eldiron_avatar
```

Runtime asset loading makes this available to clients. Character visuals can
still provide concrete presentation with values such as `tile_id` or `avatar`.
An explicit project visual wins over the ruleset default.

Ruleset items can also define `avatar_channels`:

```toml
[items.clothing.linen_shirt]
color = 2
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
- `rules list`: list races, classes, professions, skills, recipes, weapons, armor, spells, and abilities
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
`minor_heal` consumes `1 blessed_herb` only after target, range, MP, and effect
checks pass.

Resource nodes are separate from inventory materials. For example,
`wild_herb_node` is a placed world item with `static = true`, `resource_id =
"wild_herb_node"`, `respawn = 300`, and `amount = 2`. Gathering it with
`gather_herbs` adds `wild_herb x2` to the actor's inventory, hides the node, and
lets it become visible again after its respawn timer. It also sends a localized
success message such as `You gather Wild Herb x2`. `green_wood_node` works the
same way for `gather_wood`, producing `green_wood x3`, while `bird_nest_node`
uses `gather_feathers` to produce `feather x2`. The text command path can use
the same action with:

```text
gather herbs
gather wood
gather feathers
craft blessed herb
craft wooden arrows
craft hunting bow
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

## Recipes

Recipes live in `recipes.toml` and use the same source of truth as items,
actions, skills, professions, and spells. The first recipes are intentionally
small:

- `wooden_arrows`: consumes `green_wood x1` and `feather x2`, produces `wooden_arrows x10`
- `blessed_herb`: requires `minor_heal`, consumes `wild_herb x1`, produces `blessed_herb x1`
- `hunting_bow`: requires `skill_fletching = 25`, consumes `green_wood x3`, produces `hunting_bow x1`

Recipe execution consumes input stack quantities and merges output stack
quantities into existing inventory slots when possible. This is the same economy
path that later shops, gathering nodes, crafting stations, quality rules, and
profession services can use.

The text command path can craft known recipes by name:

```text
craft wooden arrows
craft hunting bow
craft blessed herb
```

Recipes use `required_skill` for hard gates and `difficulty` for balancing. The
current runtime enforces required skill and required spells before consuming
materials. The rules also name a supporting attribute, such as `DEX` for
Fletching or `WIS` for Herbalism and Restoration, so future quality and success
systems can use both practice and natural aptitude.

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
