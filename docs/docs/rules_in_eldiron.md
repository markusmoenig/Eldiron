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
      actions.toml
      abilities_spells.toml
      races_classes.toml
      README.md
      assets/
        humanoid.eldiron_avatar
        orc.eldiron_avatar
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
embeds the bundled `humanoid` and `orc` avatar assets.

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
[actions.basic_attack]
target = "hostile_or_neutral_entity"
range = "weapon"
cooldown = 1.0
```

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

The global fallback avatar reference is:

```toml
[visuals.defaults]
avatar = "humanoid"
```

The bundled asset lives in the ruleset directory:

```text
crates/ruleset/rulesets/eldiron/v1/assets/humanoid.eldiron_avatar
crates/ruleset/rulesets/eldiron/v1/assets/orc.eldiron_avatar
```

Runtime asset loading makes this available to clients. Character visuals can
still provide concrete presentation with values such as `tile_id` or `avatar`.
An explicit project visual wins over the ruleset default, and a project avatar
named `humanoid` or `orc` replaces the matching bundled default avatar for the
project.

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
actions, skills, professions, and spells. The first recipes are intentionally
small:

- `wooden_arrows`: consumes `green_wood x1` and `feather x2`, produces `wooden_arrows x10`
- `blessed_herb`: requires `minor_heal`, consumes `wild_herb x1`, produces `blessed_herb x1`
- `hunting_bow`: recommends `skill_fletching = 25`, consumes `green_wood x3`, produces `hunting_bow x1`

Recipe execution consumes input stack quantities and merges output stack
quantities into existing inventory slots when possible. This is the same economy
path that later shops, gathering nodes, crafting stations, and profession
services can use.

The text command path can craft known recipes by name:

```text
craft wooden arrows
craft hunting bow
craft blessed herb
```

Recipes can also be exposed through rules actions such as
`rules.craft_blessed_herb`, `rules.craft_wooden_arrows`, and
`rules.craft_hunting_bow`. This lets screen command slots trigger the same
recipe path as text commands and scripts while keeping recipes as the source of
truth for materials, spell gates, skill targets, and outputs.

Recipes can still use `required_skill` for hard gates, but ordinary crafting is
better modeled through output quality. `recommended_skill`, `difficulty`, and a
supporting attribute such as `DEX` or `WIS` set crafted item `quality` from
`1..100`; crafted items start at `condition = 100`. Weapon damage scales by item
quality and condition, so a new Ranger can craft immediately but starts with
rougher gear.

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
