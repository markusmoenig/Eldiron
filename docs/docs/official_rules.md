---
title: "Official Rules"
sidebar_position: 6.4
---

The **Eldiron Official Ruleset** is the default fantasy RPG ruleset bundled
with Eldiron. It gives characters, races, classes, weapons, armor, spells,
progression, visuals, and world simulation one shared meaning.

![Black-and-white rulebook scene with adventurers, dice, and an open rules tome](/img/rules/rulebook-hero-ink.png)

:::caution Work In Progress
This is a preview of the official ruleset direction for the next release line.
It is not the final v1 scope. Version 1 is expected to grow beyond the current
draft with more classes, races, spells, equipment, crafting, conditions, loot,
encounter tools, localization, and balancing support.
:::

<div class="rules-hero-spread">
  <div class="rules-hero-card rules-hero-card-main">
    <span class="rules-kicker">Official Ruleset</span>
    <h2>One Rulebook For The World</h2>
    <p>
      A Warrior, an Orc, a training sword, a healing spell, and a leather vest
      should all mean something before a project author writes custom scripts.
      The official ruleset is that shared meaning.
    </p>
  </div>
  <div class="rules-dice-card" aria-label="Dice examples">
    <span class="rules-die">d3</span>
    <span class="rules-die">d6</span>
    <span class="rules-die">d8</span>
    <span class="rules-die rules-die-large">d20</span>
  </div>
  <div class="rules-sheet-card">
    <strong>Level 1 Warrior</strong>
    <span>Human</span>
    <span>HP 16 / 16</span>
    <span>STR 12</span>
    <span>Training Sword</span>
  </div>
</div>

## Design Goal

Eldiron should feel easier to author because the ordinary RPG rules are already
there. A project should not need custom attributes for every weapon cooldown,
every spell range, every starter loadout, every faction decision, or every item
preview.

When a project says:

- this character is a `Warrior`
- this enemy is an `Orc`
- this item is a `training_sword`
- this spell is `minor_heal`

the official ruleset should answer the obvious questions: What are its stats?
What can it equip? How far can it attack? What does it roll? How long is the
cooldown? What does it look like? Who is it hostile toward?

Projects can still customize the answers. The official ruleset is the default
layer; **Game / Rules** is the project override layer.

## How To Read The Rules

The official ruleset is a tabletop-style rulebook backed by TOML. The guide
explains how the rules play. The TOML is the source of truth that Creator,
clients, runtime systems, and tools read.

| Rulebook term | In Eldiron TOML | Used by |
| --- | --- | --- |
| Race | `[races.Human]`, `[races.Orc]` | identity, relations, visual defaults |
| Class | `[classes.Warrior]` | stats, equipment, abilities, loadouts |
| Weapon | `[items.weapons.training_sword]` | damage, cooldown, range, visuals |
| Spell | `[spells.minor_heal]` | cost, range, cast time, effect |
| Intent | `[intents.attack]` | allowed actions and distances |
| Icon | `[icons.basic_attack]` | bundled UI/item masks and attribution |
| Combat kind | `[combat.kinds.fire]` | damage bonuses and reductions |

The current bundled ruleset is assembled from split TOML files under the
`eldiron.official` id.

Ruleset timing values are measured in seconds. Cooldowns, spell durations, FX
durations, resource respawns, corpse lifetimes, and NPC respawn delays all use
seconds so combat and abilities remain easy to tune. Script scheduling commands
such as `notify_in` and `block_events` are different: they use in-game minutes
because they schedule world-clock events.

| Field | Current value |
| --- | --- |
| Ruleset id | `eldiron.official` |
| Name | `Eldiron Official Ruleset` |
| Version | `1.0.0` |
| Schema version | `1` |
| Engine minimum | `0.91.0` |
| Status | `draft` |

## Dice

Eldiron uses dice notation for readable random values.

<div class="rules-callout-grid">
  <div class="rules-callout">
    <span class="rules-die-inline">1d6</span>
    <p>Roll one six-sided die. Result: 1 to 6.</p>
  </div>
  <div class="rules-callout">
    <span class="rules-die-inline">1d8</span>
    <p>Roll one eight-sided die. Result: 1 to 8.</p>
  </div>
  <div class="rules-callout">
    <span class="rules-die-inline">d20</span>
    <p>Used for critical checks and future resolution rules.</p>
  </div>
</div>

A ruleset damage entry is intentionally readable:

```toml
[items.weapons.training_sword.damage]
roll = "1d6"
bonus = 1
bonus_attribute = "STR"
bonus_every = 4
damage_kind = "physical"
```

Read it as:

1. Roll `1d6`.
2. Add the flat `bonus`.
3. Add one more bonus for every 4 points in `STR`.
4. Treat the result as `physical` damage.

So a Warrior with `STR = 12` using a training sword rolls `1d6`, adds `1`, and
then gains `3` more from Strength.

The TOML stays explicit. Tools can explain it, calculate it, and test it without
requiring hidden formulas in character data.

## Character Sheet

Every character starts from shared attributes. Race and class then shape those
attributes into a playable role.

| Group | Attributes |
| --- | --- |
| Primary | `STR`, `DEX`, `INT`, `WIS`, `VIT` |
| Resources | `HP`, `MAX_HP`, `MP`, `MAX_MP` |
| Combat | `DMG`, `POWER`, `ARMOR`, `RESIST`, `FIRE_RESIST`, `INIT`, `SPEED` |
| Progression | `LEVEL`, `EXP` |

| Attribute | Meaning |
| --- | --- |
| `STR` | physical force, melee scaling, carrying future heavy actions |
| `DEX` | agility, initiative, future ranged and avoidance hooks |
| `INT` | arcane spell scaling and knowledge hooks |
| `WIS` | divine spell scaling, resolve, and perception hooks |
| `VIT` | toughness and health scaling |
| `HP` / `MAX_HP` | current and maximum health |
| `MP` / `MAX_MP` | current and maximum mana; MP regenerates over time |
| `ARMOR` | physical protection |
| `RESIST` | general magical protection |
| `FIRE_RESIST` | fire-specific protection |
| `POWER` | general spell power |

Derived stats are table-driven. For example `MAX_HP`, `DMG`, `POWER`, `INIT`,
and `SPEED` can be calculated from level and primary attributes without asking
authors to write formulas on each character.

Mana regeneration is configured by `[resource_regen.MP]` in the official
ruleset. The default restores `1 MP` every `3` real-time seconds while the
character is active, capped by `MAX_MP`.

## Races

Races provide identity, language, visual defaults, attribute defaults, and base
relations.

| Race | Role | Avatar | Languages | Key defaults |
| --- | --- | --- | --- | --- |
| `Human` | balanced default people | `humanoid` | `common` | `HP 10`, all primary attributes `10` |
| `Orc` | strong hostile test race | `orc` | `orcish` | `HP 14`, `STR 12`, `DEX 9`, `INT 8`, `WIS 8`, `VIT 12` |

Race names are not hardcoded factions. They are identity defaults that feed
relations and reputation.

### Disposition And Reputation

Disposition answers a practical AI question: should this character treat that
character as friendly, neutral, or hostile?

| From / Toward | Human | Orc |
| --- | --- | --- |
| Human | friendly | hostile |
| Orc | hostile | friendly |

Reputation starts at `0`, which means the race relation is used as-is.

| Reputation | Disposition |
| ---: | --- |
| `-50` or lower | hostile |
| `0` | normal relation |
| `50` or higher | friendly |

Scripts should use `is_hostile(entity_id)` or `disposition_of(entity_id)` when
they need an AI decision. They should not inspect custom alignment numbers.

## Classes

Classes are the main playable role definitions. A class controls attributes,
equipment permissions, abilities, spells, progression hooks, and starting
loadout.

![Black-and-white rulebook plate of a Warrior and Cleric](/img/rules/classes-ink.png)

<div class="rules-class-grid">
  <div class="rules-class-card">
    <span class="rules-kicker">Martial</span>
    <h3>Warrior</h3>
    <p>Durable weapon user. Strong opening class for melee, armor, shields, and basic combat tests.</p>
    <ul>
      <li>Primary: STR, VIT</li>
      <li>HP 16 / 16</li>
      <li>Training Sword</li>
      <li>Padded Armor</li>
    </ul>
  </div>
  <div class="rules-class-card">
    <span class="rules-kicker">Divine</span>
    <h3>Cleric</h3>
    <p>Armored support caster. Establishes mana, healing, spell unlocks, and divine scaling.</p>
    <ul>
      <li>Primary: WIS, VIT</li>
      <li>HP 14 / 14, MP 8 / 8</li>
      <li>Novice Mace</li>
      <li>Minor Heal</li>
    </ul>
  </div>
  <div class="rules-class-card">
    <span class="rules-kicker">Ranged</span>
    <h3>Ranger</h3>
    <p>Mobile hunter. Establishes ranged weapon range, bow damage, ammunition, and DEX scaling.</p>
    <ul>
      <li>Primary: DEX, VIT</li>
      <li>HP 14 / 14</li>
      <li>Hunting Bow</li>
      <li>Wooden Arrows</li>
    </ul>
  </div>
  <div class="rules-class-card">
    <span class="rules-kicker">Civilian</span>
    <h3>Citizen</h3>
    <p>Settlement baseline for vendors, crafters, trainers, and other non-adventuring NPCs.</p>
    <ul>
      <li>Primary: VIT</li>
      <li>HP 10 / 10</li>
      <li>No combat abilities</li>
      <li>Profession services</li>
    </ul>
  </div>
</div>

### Warrior

| Area | Rule |
| --- | --- |
| Role | `martial` |
| Primary attributes | `STR`, `VIT` |
| Weapons | sword, axe, mace, spear, bow |
| Armor | cloth, leather, chain, shield |
| Starting health | `HP 16`, `MAX_HP 16` |
| Starter money | `5s` |
| Starter weapon | `training_sword` |
| Starter armor | `padded_armor` |
| Starter clothing | `wool_trousers`, `leather_shoes` |
| Starter inventory | none |
| Level 1 abilities | `basic_attack`, `guard` |
| Level 2 unlock | `power_strike` |

### Cleric

| Area | Rule |
| --- | --- |
| Role | `divine` |
| Primary attributes | `WIS`, `VIT` |
| Weapons | mace |
| Armor | cloth, leather, chain, shield |
| Starting health | `HP 14`, `MAX_HP 14` |
| Starting mana | `MP 8`, `MAX_MP 8` |
| Starter money | `5s` |
| Starter weapon | `novice_mace` |
| Starter armor | `cleric_vestments`, `round_shield` |
| Starter clothing | `wool_trousers`, `leather_shoes` |
| Starter inventory | `blessed_herb` |
| Level 1 abilities | `basic_attack`, `guard` |
| Level 1 spells | `minor_heal` |
| Level 2 unlock | `holy_light` |

### Ranger

| Area | Rule |
| --- | --- |
| Role | `ranged` |
| Primary attributes | `DEX`, `VIT` |
| Weapons | bow, sword, axe |
| Armor | cloth, leather |
| Starting health | `HP 14`, `MAX_HP 14` |
| Starter money | `5s` |
| Starter weapon | `hunting_bow` |
| Starter armor | `leather_vest` |
| Starter clothing | `wool_trousers`, `leather_shoes` |
| Starter inventory | `wooden_arrows` |
| Level 1 abilities | `basic_attack` |

### Citizen

| Area | Rule |
| --- | --- |
| Role | `civilian` |
| Primary attributes | `VIT` |
| Weapons | none by default |
| Armor | cloth, leather |
| Starting health | `HP 10`, `MAX_HP 10` |
| Starter clothing | `linen_shirt`, `wool_trousers`, `leather_shoes` |
| Combat abilities | none |
| Professions | separate from class |

Citizens are the default class for settlement NPCs. A blacksmith, merchant, or
herbalist does not need to be a Warrior just to exist in the world. Use class
for the character's combat baseline and profession for their economic or social
role:

```toml
race = "Human"
class = "Citizen"
profession = "Blacksmith"
```

An armed town guard can still be a `Warrior` with `profession = "Guard"` or a
future guard service role. The important split is that class answers "how does
this character survive conflict?" while profession answers "what do they do in
the settlement economy?"

### Professions

Professions define services and future crafting families. They are not combat
classes.

| Profession | Role |
| --- | --- |
| `Merchant` | buys and sells goods |
| `Blacksmith` | metal weapons, armor, repairs, forge recipes |
| `Tailor` | cloth clothing, light armor, dyes, patterns |
| `Herbalist` | wild herbs, gathering, herb trade |
| `Fletcher` | arrows, bows, shafts, and ranged supplies |
| `Innkeeper` | rest, food, rooms, rumors |
| `Trainer` | ability, recipe, and skill unlocks |

Professions do not cap what a player can learn. They describe social and
economic identity: who teaches, trades, repairs, or specializes in a settlement.
Crafting power comes from skills and recipe gates, so a Ranger with high
Fletching can naturally become better at bows and arrows without being locked
into a hard profession slot.

### Crafting Skills

Official v1 starts with open sandbox crafting: there is no fixed two-profession
limit. Recipes name a skill, a recommended skill value, a difficulty, and the
attribute that naturally supports that work.

| Skill | Attribute | Range | Early use |
| --- | --- | --- | --- |
| `fletching` | `DEX` | 0-100 | arrows, bows, shafts |
| `herbalism` | `WIS` | 0-100 | wild herbs and preparation |
| `restoration` | `WIS` | 0-100 | blessings, restoration reagents |
| `weaponsmithing` | `STR` | 0-100 | metal weapons |
| `armorsmithing` | `STR` | 0-100 | armor and repairs |
| `tailoring` | `DEX` | 0-100 | cloth, leather, dyes, patterns |
| `woodworking` | `DEX` | 0-100 | wooden handles, shields, furniture |

Simple recipes are available immediately. Better skill values create better
outputs instead of blocking the attempt. A character can expose skill points as
attributes such as `skill_fletching = 25`.

Crafted items use two numeric percentages:

- `quality`: `1..100`, how well the item was made
- `condition`: `1..100`, current wear or damage

Crafted items start with `condition = 100`. Their `quality` is calculated from
recipe difficulty, recommended skill, the crafter's matching skill, and the
supporting attribute. Weapon damage scales by both quality and condition.

### Recipes

Recipes transform inventory stacks into item outputs. They use the same item
templates as loot, shops, class loadouts, spell reagents, and text look paths.

| Recipe | Skill | Recommended | Difficulty | Consumes | Produces |
| --- | --- | --- | --- | --- | --- |
| `wooden_arrows` | `fletching` | 10 | 10 | `green_wood x1`, `feather x2` | `wooden_arrows x10` |
| `blessed_herb` | `restoration` | 8 | 8 | `wild_herb x1` | `blessed_herb x1` |
| `hunting_bow` | `fletching` | 25 | 35 | `green_wood x3` | `hunting_bow x1` |

`profession_hint` marks who usually teaches, sells, or performs the work, and
`class_hint` can mark a class-flavored recipe such as Cleric blessing. The
recipe gate itself is still the actual requirement: `blessed_herb` requires the
`minor_heal` spell, so Herbalism supplies the `wild_herb` and Cleric restoration
turns it into a reagent. This leaves room for Ultima Online-style character
growth while still giving towns useful roles such as Fletcher, Herbalist,
Tailor, and Blacksmith.

Setting `LEVEL` on an authored character applies class progression during
spawn/load. For example, a level 2 Cleric receives the Cleric level gains and
level 2 spell unlocks from the ruleset. Explicit character overrides, such as a
custom wounded `HP = 1`, are preserved.

Class starting loadouts are applied only when a character does not define its
own startup item attributes. This keeps the official defaults useful while still
allowing special templates.

## Intents

Intent rules are fed by actions. For example, the `attack` intent resolves to
the `basic_attack` action, and `take` resolves to the `take` action. This keeps
buttons, scripts, and later sandbox tools on the same rules path.

| Intent | Rule |
| --- | --- |
| `attack` | target must resolve to a hostile disposition |
| `attack` distance | comes from equipped weapon range, fallback `1.5` tiles |
| `take` | target must be an item, distance `1.5` tiles |
| `use` | distance `2` tiles |

Official action distances are resolved before per-character
`[intent_distance]` values. A single Attack button can therefore serve melee
and ranged weapons: swords and maces use melee range, bows use the bow category
range. In 2D directional play, choosing Attack and pressing a direction scans
that lane out to the equipped weapon range, so a bow can target an enemy several
tiles away without needing a separate ranged-attack intent.

Attack cooldown is rules-owned. A character script should call `attack()` for a
normal weapon or unarmed attack. The runtime uses the equipped weapon cooldown
and falls back to the `basic_attack` action cooldown.
When damage resolves, the target receives the `damaged` event with the final
`amount`, `attacker_id`, damage `kind`, and `source_item_id` payload fields.
Ruleset spell damage uses this same `damaged` event path, so NPC reactions do
not need separate weapon and spell handlers.

## Combat

Combat is meant to be easy to explain and easy to test.

![Black-and-white combat plate with dice, sword, armor, shield, and orc marker](/img/rules/combat-dice-ink.png)

<div class="rules-flow">
  <div><strong>1</strong><span>Choose target</span></div>
  <div><strong>2</strong><span>Call attack()</span></div>
  <div><strong>3</strong><span>Roll weapon dice</span></div>
  <div><strong>4</strong><span>Apply damage kind</span></div>
  <div><strong>5</strong><span>Reduce by armor</span></div>
  <div><strong>6</strong><span>Apply final damage</span></div>
</div>

| Combat default | Value |
| --- | --- |
| Damage kind | `physical` |
| Unarmed damage | `1d3`, plus `STR` bonus every 4 points |
| Attack cooldown | `1.0` seconds |
| Cast cooldown | `1.5` seconds |
| Global cooldown | `0.5` seconds |
| Critical roll | `20` |
| Critical multiplier | `1.5` |

### Example Attack

A level 1 Human Warrior attacks an Orc with a training sword.

| Step | Result |
| --- | --- |
| Weapon | `training_sword` |
| Damage roll | `1d6` |
| Flat bonus | `+1` |
| Strength bonus | `STR 12`, so `+3` from `bonus_every = 4` |
| Damage kind | `physical` |
| Cooldown | `1.0` seconds |

If the die rolls `4`, the attack starts at `4 + 1 + 3 = 8` physical damage.
The Orc's physical reduction then comes from `ARMOR` and equipped armor before
the final amount is applied.

### Damage Kinds

| Kind | Meaning | Reduction |
| --- | --- | --- |
| `physical` | mundane weapon and body damage | `ARMOR` plus equipped armor `ARMOR` |
| `arcane` | unshaped magical force | `RESIST` plus equipped armor `RESIST` |
| `spell` | compatibility name for arcane force | `RESIST` plus equipped armor `RESIST` |
| `fire` | heat, flame, and burning magic | `FIRE_RESIST` plus equipped armor `FIRE_RESIST` |

## Weapons

Weapons define category, slot, cooldown, damage kind, visual data, and dice
damage.

| Weapon | Category | Slot | Cooldown | Damage | Visual |
| --- | --- | --- | ---: | --- | --- |
| `training_sword` | sword | `main_hand` | `1.0` | `1d6`, bonus `1`, `STR` every 4 | wooden diagonal sword mask |
| `hand_axe` | axe | `main_hand` | `1.2` | `1d8`, bonus `1`, `STR` every 4 | diagonal axe mask |
| `novice_mace` | mace | `main_hand` | `1.15` | `1d6`, bonus `0`, `STR` every 4 | diagonal mace mask |
| `hunting_bow` | bow | `main_hand` | `1.5` | `1d6`, bonus `0`, `DEX` every 4 | diagonal bow mask |
| `training_spear` | spear | `main_hand` | `1.25` | `1d6`, bonus `1`, `STR` every 4 | diagonal spear mask |

Weapon categories add shared behavior.

| Category | Hands | Base cooldown | Range |
| --- | --- | ---: | ---: |
| sword | one-handed | `1.0` | default melee |
| axe | one-handed | `1.2` | default melee |
| mace | one-handed | `1.15` | default melee |
| spear | two-handed | `1.25` | `2` |
| bow | two-handed | `1.5` | `6` |

## Armor And Clothing

The current armor model follows broad material families: cloth, leather, chain,
and shield. This keeps equipment readable and gives crafting professions a
natural future path.

![Black-and-white equipment plate showing weapons, armor, boots, trousers, and clothing](/img/rules/equipment-ink.png)

| Armor | Family | Slot | Armor | Avatar channels |
| --- | --- | --- | ---: | --- |
| `padded_armor` | cloth | torso | `1` | torso, arms |
| `cleric_vestments` | cloth | torso | `1` | torso, arms |
| `leather_vest` | leather | torso | `2` | torso |
| `chain_shirt` | chain | torso | `3` | torso, arms |
| `round_shield` | shield | shield | `1` | round shield mask |

| Clothing | Family | Slot | Worth | Avatar channels |
| --- | --- | --- | ---: | --- |
| `linen_shirt` | cloth | torso | `5c` | torso, arms |
| `wool_trousers` | cloth | legs | `6c` | legs |
| `leather_shoes` | leather | feet | `8c` | feet |

| Container | Family | Slots | Visual |
| --- | --- | ---: | --- |
| `small_bag` | bag | `6` | pouch mask |
| `loot_corpse` | corpse | `8+` | tombstone mask |

Containers open as floating UI panels. They use procedural UI templates by
default; a template defines columns, slot size, padding, gap, title, and palette
colors. Projects can later skin the same template with tile ids for frame
corners, edges, center fill, and slots without turning every bag into a custom
screen.

Dead characters that call `drop_items("")` create a lootable corpse container
under the official rules. The corpse expands to fit the carried loot and uses
the same open, click-to-take, drag, and text transfer paths as bags. Empty
corpses despawn by default so cleaned-out tombstones do not remain on the map.
Non-empty corpses also have a lifetime. For respawning NPCs, the corpse
disappears shortly before the NPC returns, using
`despawn_before_respawn_seconds`; other corpses use `despawn_seconds`.

NPCs respawn by default. `[respawn.npc]` defines the delay, restores health to
full, restores the NPC's startup loadout and behavior state, and removes that
NPC's corpse when it returns. Player death remains script-controlled so games
can decide whether the player wakes at a shrine, returns to town, loses money,
keeps a tombstone, or follows another custom death loop. Individual NPCs can
override the timer with `respawn_seconds` or disable automatic respawn with
`respawn = false`.

| Ammunition | Family | Quantity | Used by | Visual |
| --- | --- | ---: | --- | --- |
| `wooden_arrows` | arrow | `20` | bow, 1 per attack | diagonal arrow mask |

| Reagent | Family | Quantity | Used by | Visual |
| --- | --- | ---: | --- | --- |
| `blessed_herb` | herb | `3` | Cleric restoration reagent | herb sprig mask |

| Material | Family | Quantity | Used by | Visual |
| --- | --- | ---: | --- | --- |
| `green_wood` | wood | `5` | shafts, handles, woodworking | wood shaft mask |
| `feather` | feather | `5` | arrow fletching | feather mask |
| `wild_herb` | herb | `5` | gathered herbalism material | herb sprig mask |

## Economy

Eldiron uses a classic copper, silver, and gold economy. All prices and rewards
are stored as integer base units, where copper is the base:

| Currency | Symbol | Base value |
| --- | --- | ---: |
| Copper | `c` | `1` |
| Silver | `s` | `10` |
| Gold | `g` | `100` |

The UI formats base values compactly. `125` copper is shown as `1g 2s 5c`.
This keeps the rules and tools simple while still presenting familiar RPG
money to players.

New player characters start with `50` copper, displayed as `5s`, unless the
character explicitly defines another `wealth` value.

| Money item | Adds |
| --- | ---: |
| `copper_coin` | `1c` |
| `silver_coin` | `1s` |
| `gold_coin` | `1g` |

Money items are marked `monetary = true`. Taking one adds its value directly to
the actor's wallet instead of placing the coin item in inventory. Normal item
`worth`, shop prices, loot rewards, and character `wealth` are all measured in
base copper units.

Money loot can use the same item template with a different value. For example,
a dropped purse can set `monetary = true`, `currency = "silver"`, and
`amount = 5` to add `5s` when taken. `worth = 50` is the equivalent raw base
value and is useful for tools and display.

Resource nodes are the world objects that produce materials. They are distinct
from inventory items.

| Resource node | Action | Produces | Respawn | Visual |
| --- | --- | --- | ---: | --- |
| `wild_herb_node` | `gather_herbs` | `wild_herb x2` | `300` seconds | herb sprig mask |
| `green_wood_node` | `gather_wood` | `green_wood x3` | `300` seconds | wood shaft mask |
| `bird_nest_node` | `gather_feathers` | `feather x2` | `300` seconds | feather/nest mask |

| Tool | Worth | Interaction | State | Visual |
| --- | ---: | --- | --- | --- |
| `torch` | `1s` | `use` toggles it on/off | swaps light, tile, and look text; while lit it loses `condition` over game minutes and destroys itself at `0%` | one bundled unlit tile, one bundled four-frame lit tile |

Bows consume one matching ammunition item from the attacker's inventory when a
weapon attack resolves. `hunting_bow` declares `ammunition = "wooden_arrows"`
and `ammunition_quantity = 1`, so the ruleset owns both which item is needed
and how many are spent. Stackable ammunition decrements its `quantity`; when the
stack reaches zero the inventory slot is emptied. `wooden_arrows` therefore
means one inventory stack of arrows, not one single arrow item per slot.

When an item defines `avatar_channels` and no explicit icon or tile source,
Eldiron derives its preview from the bundled humanoid avatar. Inventory,
equipment, and ground item visuals use the same generated shape.

Some official items are interactive templates rather than passive gear. A torch
contains its own script, authored state text, light definition, and lit/unlit
visual state. Its unlit tile and animated lit tile are bundled with the official
rules, so projects can place a complete working torch without rebuilding that
behavior by hand. Its burn time is rules-owned through `[durability]`: while
`active`, it drains `condition` by `10%` per `60` game minutes, and `on_empty =
"destroy"` removes the burned-out torch.

## Abilities And Spells

Abilities and spells define what exists. Actions define how an actor performs a
gameplay verb. This keeps the current RPG layer compatible with future sandbox
verbs such as harvesting, crafting, lockpicking, stealing, or taming.

| Action | Kind | Target | Cost | Result |
| --- | --- | --- | --- | --- |
| `basic_attack` | attack | hostile or neutral entity | - | weapon damage |
| `power_strike` | attack | hostile or neutral entity | - | `power_strike` damage |
| `minor_heal` | spell | friendly or self | `3 MP`, `1 blessed_herb` | `minor_heal` healing |
| `holy_light` | spell | hostile or neutral entity | `4 MP` | `holy_light` damage |
| `take` | interaction | ground item | - | move item to inventory |
| `gather_herbs` | gather | resource node | - | resource output |
| `gather_wood` | gather | resource node | - | resource output |
| `gather_feathers` | gather | resource node | - | resource output |
| `craft_blessed_herb` | craft | self | `1 wild_herb`, `minor_heal` known | `blessed_herb x1` |
| `craft_wooden_arrows` | craft | self | `1 green_wood`, `2 feather` | `wooden_arrows x10` |
| `craft_hunting_bow` | craft | self | `3 green_wood`, recommended `fletching 25` | `hunting_bow x1` |

Action definitions already include a generic `consumes` list, so spells,
crafting, and other sandbox actions can require reagents or materials without a
new hardcoded system. These costs use stack quantities too: consuming three
arrows, herbs, ore, or reagents subtracts three from a matching stack before it
removes an inventory slot.

Actions can also declare `skill` and `required_skill`. The first gather actions
are open at `required_skill = 0`, but the same mechanism is now available for
higher-tier ore, wood, herbs, locks, traps, and profession actions.

Class action bars expose five demo slots in Hideout2D. Warrior gets martial
combat plus simple field gathering and arrow crafting. Cleric gets attack,
healing, holy damage, herb gathering, and herb blessing. Ranger gets attack,
wood and feather gathering, arrow fletching, and bow crafting.

Abilities are class-owned combat options. Spells add school, cast time, and
damage or healing data. Actions connect those definitions to targets, costs,
cooldowns, results, and FX.

Scripts use `attack()` for the normal weapon attack. Named action buttons or
text commands use `use_action("<id>")`; for example `use_action("power_strike")`
or `use power strike orc` in text play. Resource actions can also be typed by
name, such as `gather herbs`, `gather wood`, or `gather feathers`, which targets
the nearest matching visible resource node. Successful gathering sends a
localized result message such as `You gather Wild Herb x2`. Recipes can be
typed by name too, such as `craft blessed herb`, `craft wooden arrows`, or
`craft hunting bow`. Container transfers start with simple text commands such
as `open small bag`, `put wild herb in bag`, and `take wild herb from bag`.

| Ability | Kind | Cooldown | Range | Effect |
| --- | --- | ---: | --- | --- |
| `basic_attack` | attack | `1.0` | weapon | normal physical attack |
| `guard` | stance | `3.0` | self | `ARMOR +2` for `2.0` seconds |
| `power_strike` | attack | `4.0` | weapon | `1d8`, bonus `2`, `STR` every 4 |

| Spell | School | Kind | Cost | Cooldown | Range | Roll |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `minor_heal` | restoration | heal | `3 MP`, `1 blessed_herb` | `4.0` | `5` | `1d6`, bonus `1`, `WIS` every 4 |
| `holy_light` | restoration | damage | `4 MP` | `5.0` | `5` | `1d6`, bonus `1`, `WIS` every 4 |
| `fire_spark` | fire | damage | `2 MP` | `3.0` | `6` | `1d6`, bonus `0`, `INT` every 4 |

Spell FX use semantic presets from `fx.toml`. The ruleset describes the visual
intent, and the engine maps that to procedural particles and lighting.

| Spell | Cast FX | Travel FX | Impact FX |
| --- | --- | --- | --- |
| `minor_heal` | `rising_motes` | - | `rising_motes` |
| `holy_light` | `holy_glow` | `holy_bolt` | `hit_burst` |
| `fire_spark` | - | `ember_trail` | `fire_burst` |

| FX Preset | Description |
| --- | --- |
| `hit_burst` | short impact burst from the target center |
| `rising_motes` | soft particles across the tile, moving upward |
| `holy_glow` | warm divine aura around caster or target |
| `holy_bolt` | focused holy projectile with trailing glow |
| `fire_burst` | hot impact explosion with sparks and smoke |
| `flame_patch` | small burning area on the tile |
| `ember_trail` | embers behind a moving fire spell |

## Progression

Progression uses explicit tables so balancing is visible.

| Level | Required XP |
| ---: | ---: |
| 2 | 100 |
| 3 | 250 |
| 4 | 450 |
| 5 | 700 |
| 10 | 2700 |
| 20 | 10450 |

Minor quests award `25` XP, major quests award `100` XP, and kill XP starts at
`25` per defender level.

The current maximum level is `20`. Level-up rewards and ability unlocks are
class-owned.

## Visual Defaults

Rules and visuals meet in play, so the ruleset provides a consistent default
visual layer.

| Visual rule | Value |
| --- | --- |
| Default avatar | `humanoid` |
| Avatar assets | `assets/humanoid.eldiron_avatar`, `assets/orc.eldiron_avatar` |
| Palette | single 31-color mood palette based on Lospec's "31" palette |
| Explicit override | project `tile_id`, `avatar`, or empty visual fields win |

On load, Eldiron copies the effective ruleset palette into the project palette.
That keeps internal painting, item previews, avatar channels, and generated
icons on one color source.

## Tools And Testing

The official ruleset is structured so tools can answer practical authoring
questions without guessing:

- Which items does a level 1 Warrior start with?
- Can a Cleric equip this weapon?
- What is the attack range of this weapon?
- How much XP is needed for the next level?
- What damage can this weapon or spell roll?
- Which race relation makes this target hostile?
- How long until this character can attack again?
- What happens if this Orc fights this Warrior 100 times?

The same TOML should serve gameplay, Creator UI, terminal tools, console tools,
validation, and automated arena tests.

## Work In Progress Roadmap

The current draft is a playable slice, not the final v1 promise.

Already present or underway:

- Human and Orc race baselines
- Warrior and Cleric class baselines
- default race relations and reputation thresholds
- attack, take, and use intent rules
- weapon and unarmed dice damage
- physical, arcane, spell, and fire damage kinds
- cloth, leather, chain, and shield armor families
- starter weapons, armor, clothing, abilities, and spells
- default humanoid avatar and rules-owned palette
- stackable materials, reagents, ammunition, and first crafting recipes
- skill-gated crafting with open profession growth
- first item container, `small_bag`, with text transfer commands
- Creator integration and rules-aware tools

Expected v1 growth areas:

- more playable classes and enemy roles
- more races and creature templates
- a larger spell and ability catalogue
- larger crafting professions, reagents, recipes, stations, and item outputs
- container popups, bags, chests, corpses, and loot transfer UI
- loot tables and treasure rules
- conditions such as stunned, burning, poisoned, blessed, and guarded
- armor proficiency, weapon proficiency, and class restrictions
- encounter templates and automatic arena balance tests
- rarity, value, repair rules, and deeper quality/condition effects
- richer AI intent rules and disposition changes
- localization-ready rule names, messages, and descriptions
- illustrated guide pages and deeper examples

The long-term goal is not a tiny ruleset. The goal is a real world simulation
that starts simple, stays readable, and grows without returning to scattered
per-character configuration.
