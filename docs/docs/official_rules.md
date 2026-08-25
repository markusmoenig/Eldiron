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
| Icon | `[icons.basic_attack]` | bundled artist-editable RGBA artwork and attribution |
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

The official ruleset maps semantic runtime roles explicitly:

```toml
[attributes.roles]
health = "HP"
max_health = "MAX_HP"
level = "LEVEL"
experience = "EXP"
weapon_damage = "DMG"
armor = "ARMOR"
```

These names are official content, not engine constants. Standalone rulesets can
map the same roles to different attributes or omit progression roles entirely.
Spell and ability costs are typed action maps, so the official `MP` costs use
the same runtime path as a custom `FOCUS`, stamina, or ammunition resource.

Derived stats are calculated on demand from the ruleset. The saved character
attribute remains the `base`, while the formula may reference `level`, another
attribute, or another derived stat:

```toml
[derived_stats.POWER]
formula = "base + floor(max(0, INT - 10) / 4) + floor(max(0, WIS - 10) / 4)"
minimum = 0
```

The official rules define `MAX_HP`, `MAX_MP`, `DMG`, `POWER`, `INIT`, and
`SPEED` this way. Derived values drive combat, action requirements, healing,
resource caps and regeneration, and respawn health. Condition modifiers apply
to formula inputs and then to the resulting stat, without overwriting saved
base attributes.

Mana regeneration is configured by `[resource_regen.MP]` in the official
ruleset. The default restores `1 MP` every `3` real-time seconds while the
character is active, capped by `MAX_MP`.

Class resource growth uses `progression.level.resource_gains`. Each entry names
its current attribute, maximum attribute, and amount gained per level. Cleric,
for example, grows both HP and MP; Warrior and Ranger grow HP. The same generic
list supports custom resources and levels 11–30 without additional field types.

## Races

Races provide identity, language, visual defaults, attribute defaults, and base
relations.

| Race | Role | Avatar | Languages | Key defaults |
| --- | --- | --- | --- | --- |
| `Human` | balanced default people | `humanoid` | `common` | `HP 10`, all primary attributes `10` |
| `Orc` | strong hostile test race | `orc` | `orcish` | `HP 14`, `STR 12`, `DEX 9`, `INT 8`, `WIS 8`, `VIT 12` |
| `Skeleton` | undead skeletal creature | `skeleton` | - | `HP 12`, `VIT 12`, traits `undead`, `skeletal` |

Race names are not hardcoded factions. They are identity defaults that feed
relations and reputation. Race traits are materialized as normal entity
attributes, so custom actions can inspect them through the same public
predicate system.

The bundled `skeleton.eldiron_avatar` currently copies the humanoid frames but
has its own asset id, internal UUID, and name. It is an editable placeholder:
importing a dedicated Skeleton atlas replaces its art without changing race or
project references.

### Disposition And Reputation

Disposition answers a practical AI question: should this character treat that
character as friendly, neutral, or hostile?

| From / Toward | Human | Orc | Skeleton |
| --- | --- | --- | --- |
| Human | friendly | hostile | hostile |
| Orc | hostile | friendly | hostile |
| Skeleton | hostile | hostile | friendly |

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
| Level 4 unlock | `rally` |
| Level 6 unlock | `crushing_blow` |
| Level 8 unlock | `iron_guard` |
| Level 10 unlock | `executioner_strike` |

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
| Starter inventory | `blessed_herb`, `moonwater` |
| Level 1 abilities | `basic_attack`, `guard` |
| Level 1 spells | `minor_heal` |
| Level 2 unlock | `holy_light` |
| Level 4 unlock | `blessing`, `turn_undead` |
| Level 6 unlock | `greater_heal` |
| Level 8 unlock | `smite` |
| Level 10 unlock | `sanctuary` |

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
| Level 2 unlock | `aimed_shot` |
| Level 4 unlock | `quick_step` |
| Level 6 unlock | `piercing_shot` |
| Level 8 unlock | `hunter_focus` |
| Level 10 unlock | `deadly_shot` |

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
| `alchemy` | `INT` | 0-100 | moonwater, ember beads, volatile preparations |
| `ritualism` | `WIS` | 0-100 | warding salt, magical focuses, protective charms |
| `weaponsmithing` | `STR` | 0-100 | metal weapons |
| `armorsmithing` | `STR` | 0-100 | armor and repairs |
| `tailoring` | `DEX` | 0-100 | cloth, leather, dyes, patterns |
| `woodworking` | `DEX` | 0-100 | wooden handles, shields, furniture |

Simple recipes are available immediately. Better skill values create better
outputs instead of blocking the attempt. A character can expose skill points as
attributes such as `skill_fletching = 25`.

Official crafting uses deterministic, use-based advancement. A successful
craft grants `1` point in its recipe skill and another `1` while the crafter is
below the recipe's recommended value. A recipe stops teaching once the crafter
is `20` points above that recommendation, and a skill never exceeds its own
maximum. Failed crafts and missing-material attempts grant nothing. This makes
simple preparations the practice path into advanced ritual work instead of
requiring authored skill points.

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
| `moonwater` | `alchemy` | 12 | 12 | `moonleaf x2` | `moonwater x2` |
| `consecrated_oil` | `restoration` | 20 | 22 | `blessed_herb x1`, `sun_shard x1` | `consecrated_oil x2` |
| `warding_salt` | `ritualism` | 25 | 28 | `grave_dust x2`, `sun_shard x1` | `warding_salt x3` |
| `ember_beads` | `alchemy` | 18 | 20 | `ember_resin x2` | `ember_bead x3` |
| `ritual_censer` | `ritualism` | 35 | 38 | `green_wood x2`, `consecrated_oil x1`, `warding_salt x1` | `ritual_censer x1` |
| `sunward_charm` | `ritualism` | 50 | 55 | `sun_shard x2`, `moonwater x1`, `warding_salt x1` | `sunward_charm x1` |

`profession_hint` marks who usually teaches, sells, or performs the work, and
`class_hint` can mark a class-flavored recipe such as Cleric blessing. The
recipe gate itself is still the actual requirement: `blessed_herb` requires the
`minor_heal` spell, so Herbalism supplies the `wild_herb` and Cleric restoration
turns it into a reagent.

Ritual crafting deliberately forms a small production chain instead of a list
of unrelated conversions. Moonleaf becomes Moonwater; sunlight and blessed
herbs become Consecrated Oil; grave dust and sunlight become Warding Salt; and
Ember Resin becomes Ember Beads. Those prepared reagents are spent by spells,
but they can also be invested in permanent equipment. A Ritual Censer favors
spell power and resistance, while a more demanding Sunward Charm protects
against magic and fire. The same Sun Shards are therefore contested between
immediate spell supplies and long-term equipment. This leaves room for Ultima
Online-style character growth while giving towns useful roles such as
Fletcher, Herbalist, Ritualist, Tailor, and Blacksmith.

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

These declarations are enforced. Equipping a bow or spear occupies both
`main_hand` and `off_hand`. A shield is stored in the `shield` equipment slot
but also occupies `off_hand`, so it cannot be combined with either two-handed
weapon. One-handed weapons can still be combined with a shield.

Each class's `allowed_weapons` and `allowed_armor` lists are also authoritative:
Citizen has no weapon permission; Warrior accepts every current weapon and
armor family; Cleric accepts maces with cloth, leather, chain, and shields; and
Ranger accepts bows, swords, and axes with cloth or leather. A rejected equip
attempt leaves the item where it was.

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
| `ritual_censer` | focus | focus | `POWER +1`, `RESIST +1` | shared mace/censer mask |
| `sunward_charm` | focus | focus | `RESIST +2`, `FIRE_RESIST +1` | shared charm mask |

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

| Reagent | Affinity | Default stack | Used by | Visual |
| --- | --- | ---: | --- | --- |
| `blessed_herb` | restoration / `LO` | `3` | Minor Heal, Greater Heal, ritual recipes | herb sprig mask |
| `moonwater` | life / `VI` | `2` | Minor Heal, Greater Heal, Sanctuary | shared moonwater mask |
| `consecrated_oil` | grace / `YA` | `2` | Holy Light, Blessing, Smite | shared holy-light mask |
| `warding_salt` | ward / `SAR` | `3` | Turn Undead, Sanctuary | shared shield mask |
| `ember_bead` | flame / `FUL` | `3` | Fire Spark, Smite | shared torch/ember mask |

| Material | Family | Quantity | Used by | Visual |
| --- | --- | ---: | --- | --- |
| `green_wood` | wood | `5` | shafts, handles, woodworking | wood shaft mask |
| `feather` | feather | `5` | arrow fletching | feather mask |
| `wild_herb` | herb | `5` | gathered herbalism material | herb sprig mask |
| `moonleaf` | herb | `4` | Moonwater distillation | recolored herb sprig mask |
| `sun_shard` | mineral | `3` | divine oils, wards, charms | shared holy-light mask |
| `grave_dust` | dust | `4` | Warding Salt | shared grave mask |
| `ember_resin` | resin | `4` | Ember Beads | shared torch/ember mask |

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
| `wild_herb_node` | `gather_herbs` | `wild_herb x2` | `300` seconds | authored herb-sprig icon |
| `green_wood_node` | `gather_wood` | `green_wood x3` | `300` seconds | authored wood-shaft icon |
| `bird_nest_node` | `gather_feathers` | `feather x2` | `300` seconds | authored feather/nest icon |
| `moonleaf_patch` | `gather_moonleaf` | `moonleaf x2` | `360` seconds | authored moonleaf icon |
| `sunstone_outcrop` | `mine_sun_shards` | `sun_shard x2` | `480` seconds | authored golden-outcrop icon |
| `old_grave` | `sift_grave_dust` | `grave_dust x2` | `420` seconds | authored tombstone icon |
| `resinous_stump` | `tap_ember_resin` | `ember_resin x2` | `360` seconds | authored ember-resin icon |

| Tool | Worth | Interaction | State | Visual |
| --- | ---: | --- | --- | --- |
| `torch` | `1s` | `use` toggles it on/off | `active` automatically selects the reconstructed world tile and icon state; the script swaps the light and look text, while durability destroys the torch at `0%` | one-frame Off and four-frame On PNG state bundle |

Bows consume one matching ammunition item from the attacker's inventory when a
weapon attack resolves. `hunting_bow` declares `ammunition = "wooden_arrows"`
and `ammunition_quantity = 1`, so the ruleset owns both which item is needed
and how many are spent. Stackable ammunition decrements its `quantity`; when the
stack reaches zero the inventory slot is emptied. `wooden_arrows` therefore
means one inventory stack of arrows, not one single arrow item per slot.

When authored item PNGs are unavailable, an item with `avatar_channels` can
derive a missing-art preview from the bundled humanoid avatar. This generator
is a fallback; official item-id PNGs normally provide inventory and equipment
art without runtime palette remapping.

Some official items are interactive templates rather than passive gear. A torch
contains its own script, authored state text, light definition, and lit/unlit
visual state. Its one-frame Off and four-frame On PNGs are bundled under
`assets/icons/torch/`; Eldiron reconstructs the world tiles from those same
files while preserving stable tile UUIDs. The generic `active` state mapping
selects those tiles, so the script does not contain their IDs. Projects can
therefore place a complete working torch without maintaining separate icon and
tile binaries. Its burn time is rules-owned through `[durability]`: while
`active`, it drains `condition` by `10%` per `60` game minutes, and `on_empty =
"destroy"` removes the burned-out torch.

## Abilities And Spells

Abilities and spells define what exists. Actions define how an actor performs a
gameplay verb. This keeps the current RPG layer compatible with future sandbox
verbs such as harvesting, crafting, lockpicking, stealing, or taming.

### Spellbook

Words of Power are listed directly beside their spells. They are an optional
input vocabulary: a graphical game may show spell icons, a text game may accept
the words, and a rune-based game may draw the same tokens as symbols.

| Spell | Words of Power | MP | Reagents | Effect |
| --- | --- | ---: | --- | --- |
| Minor Heal | `LO VI` | 3 | Blessed Herb x1, Moonwater x1 | restore `1d6 + 1`, scaling with WIS |
| Holy Light | `YA FUL` | 4 | Consecrated Oil x1 | deal `1d6 + 1` arcane damage, scaling with WIS |
| Blessing | `YA` | 5 | Consecrated Oil x1 | grant divine power and resistance |
| Turn Undead | `SAR IR` | 5 | Warding Salt x1 | weaken and repel an undead target |
| Greater Heal | `LO LO` | 7 | Blessed Herb x2, Moonwater x1 | restore `2d6 + 3`, scaling with WIS |
| Smite | `FUL YA` | 6 | Consecrated Oil x1, Ember Bead x1 | deal `2d6 + 2` arcane damage, scaling with WIS |
| Sanctuary | `SAR VI` | 9 | Warding Salt x1, Moonwater x1 | grant strong physical and magical protection |
| Fire Spark | `FUL` | 2 | Ember Bead x1 | deal `1d6` fire damage, scaling with INT |

The six-word lexicon is intentionally small:

| Word | Meaning | Material affinity |
| --- | --- | --- |
| `LO` | restore | herbs |
| `VI` | life | moonleaf and Moonwater |
| `FUL` | flame | Ember Resin and Ember Beads |
| `YA` | grace | Sun Shards and Consecrated Oil |
| `IR` | banish | graves and the dead |
| `SAR` | ward | Warding Salt |

The ingredients echo the spoken construction without enforcing a mechanical
one-token/one-item rule. That keeps each spell memorable while leaving room for
recipes, substitutions, reagent-free settings, or entirely different input
schemes in custom games.

| Action | Kind | Target | Cost | Result |
| --- | --- | --- | --- | --- |
| `basic_attack` | attack | hostile or neutral entity | - | weapon damage |
| `power_strike` | attack | hostile or neutral entity | - | `power_strike` damage |
| `guard` | stance | self | - | apply `guarded` for `2.0s` |
| `rally` | stance | self | - | apply `rallied` for `6.0s` |
| `crushing_blow` | attack | hostile or neutral entity | - | `crushing_blow` damage |
| `iron_guard` | stance | self | - | apply `iron_guarded` for `5.0s` |
| `executioner_strike` | attack | hostile or neutral entity | - | `executioner_strike` damage |
| `aimed_shot` | attack | hostile or neutral entity | - | `aimed_shot` damage |
| `quick_step` | stance | self | - | apply `quick_step` for `4.0s` |
| `piercing_shot` | attack | hostile or neutral entity | - | `piercing_shot` damage |
| `hunter_focus` | stance | self | - | apply `hunter_focus` for `6.0s` |
| `deadly_shot` | attack | hostile or neutral entity | - | `deadly_shot` damage |
| `minor_heal` | spell | friendly or self | `3 MP`, Blessed Herb, Moonwater | `minor_heal` healing |
| `holy_light` | spell | hostile or neutral entity | `4 MP`, Consecrated Oil | `holy_light` damage |
| `blessing` | spell | friendly or self | `5 MP`, Consecrated Oil | apply `blessed` for `8.0s` |
| `turn_undead` | spell | hostile or neutral undead entity | `5 MP`, Warding Salt | apply `turned` for `5.0s` |
| `greater_heal` | spell | friendly or self | `7 MP`, Blessed Herb x2, Moonwater | `greater_heal` healing |
| `smite` | spell | hostile or neutral entity | `6 MP`, Consecrated Oil, Ember Bead | `smite` damage |
| `sanctuary` | spell | friendly or self | `9 MP`, Warding Salt, Moonwater | apply `sanctuary` for `8.0s` |
| `fire_spark` | spell | hostile or neutral entity | `2 MP`, Ember Bead | `fire_spark` damage |
| `take` | interaction | ground item | - | move item to inventory |
| `gather_herbs` | gather | resource node | - | resource output |
| `gather_wood` | gather | resource node | - | resource output |
| `gather_feathers` | gather | resource node | - | resource output |
| `gather_moonleaf` | gather | resource node | `herbalism 5` | `moonleaf x2` |
| `mine_sun_shards` | gather | resource node | `ritualism 5` | `sun_shard x2` |
| `sift_grave_dust` | gather | resource node | `ritualism 10` | `grave_dust x2` |
| `tap_ember_resin` | gather | resource node | `alchemy 5` | `ember_resin x2` |
| `craft_blessed_herb` | craft | self | `1 wild_herb`, `minor_heal` known | `blessed_herb x1` |
| `craft_wooden_arrows` | craft | self | `1 green_wood`, `2 feather` | `wooden_arrows x10` |
| `craft_hunting_bow` | craft | self | `3 green_wood`, recommended `fletching 25` | `hunting_bow x1` |

The optional `words_of_power` scheme binds the sequences in the Spellbook to
ordinary actions. This is an input method, not separate spell logic: knowledge,
targeting, costs, cooldowns, effects, and FX still come from the same action. A
game can render the tokens as words, runes, icons, or buttons, or ignore the
scheme. A Messages command input accepts the phrases directly, including a
named target when needed: `LO VI`, `SAR IR at skeleton`, or the explicit
`invoke words_of_power:YA FUL at orc` form.

Action definitions already include a generic `consumes` list, so spells,
crafting, and other sandbox actions can require reagents or materials without a
new hardcoded system. These costs use stack quantities too: consuming three
arrows, herbs, ore, or reagents subtracts three from a matching stack before it
removes an inventory slot.

Actions can also declare `skill` and `required_skill`. The first gather actions
are open at `required_skill = 0`, but the same mechanism is now available for
higher-tier ore, wood, herbs, locks, traps, and profession actions.

Standalone and heavily customized games can gate an action on their own actor
attributes through `requires.attributes`. Predicates support scalar equality,
numeric minimum/maximum values, and membership in tag-like string lists. For
example an action can require `karma >= 10`, `mode = "stealth"`, and a
`traits` list containing `"undead"` without requiring a class or level system.
`requires.target_attributes` applies the same vocabulary to the selected
entity, enabling actions such as Turn Undead without a Skeleton-specific
engine branch. The official level-4 Cleric action now demonstrates this: it
requires the target's ordinary `traits` list to contain `undead`, then applies
the generic `turned` condition. A living hostile target fails before MP,
cooldown, or particles are consumed.

Those actions can also use `result.modify` for built-in state changes. A single
action may add to numeric actor or target values, set numeric, boolean, or
string attributes, and clamp a result to fixed bounds or another recipient
attribute. This covers verbs such as resting, consuming stamina, changing
karma, or entering a frightened state without writing an action-specific
runtime branch.

Actions can apply or remove reusable conditions. The official `guard` action
applies `conditions.guarded`, whose definition owns its two-second duration,
refresh behavior, tags, and `ARMOR +2` modifier. The server tracks remaining
time and stacks separately from base attributes, so expiration never has to
restore a previously overwritten ARMOR value. Other rulesets can use the same
path for poison, blessings, stances, or curses, including trait-based
immunities, stack-scaled periodic damage/healing/resource effects, lifecycle
events, and apply/active/tick/remove particle phases. Guard demonstrates the
visual lifecycle with a blue-white burst and an aura that follows its owner.
Turned demonstrates a harmful trait-gated condition with `POWER -4`, halved
`SPEED`, and its own divine apply/active/remove particles.
Active condition stacks, remaining time, periodic phase, and stable applying
source survive serialized region state; transient particles are rebuilt rather
than treated as saved gameplay objects.

The three adventuring action bars expose their full level-10 path. Locked
buttons use the same class unlock data to report the required level.

`classes.<id>.unlocks.level_N` is the sole owner of when an ability or spell is
known. Starting loadouts own items only. Ability and spell definitions own
identity and roll data; actions own targets, costs, cooldowns, results, and FX.

Scripts use `attack()` for the normal weapon attack. Named action buttons or
text commands use `use_action("<id>")`; for example `use_action("power_strike")`
or `use power strike orc` in text play. A custom source-tool action may use
`use_action(action_id, target_id, source_item_id)` to select the exact owned
tool instance. Resource actions can also be typed by name, such as `gather
herbs`, `gather wood`, or `gather feathers`, which targets the nearest matching
visible resource node. Successful gathering sends a localized result message
such as `You gather Wild Herb x2`. Recipes can be
typed by name too, such as `craft blessed herb`, `craft wooden arrows`, or
`craft hunting bow`. Container transfers start with simple text commands such
as `open small bag`, `put wild herb in bag`, and `take wild herb from bag`.

| Ability | Kind | Cooldown | Range | Effect |
| --- | --- | ---: | --- | --- |
| `basic_attack` | attack | `1.0` | weapon | normal physical attack |
| `guard` | stance | `3.0` | self | `ARMOR +2` for `2.0` seconds |
| `power_strike` | attack | `4.0` | weapon | `1d8`, bonus `2`, `STR` every 4 |
| `rally` | stance | `10.0` | self | `DMG +2`, `RESIST +1` for `6.0` seconds |
| `crushing_blow` | attack | `6.0` | weapon | `1d10`, bonus `3`, `STR` every 4 |
| `iron_guard` | stance | `12.0` | self | `ARMOR +4`, reduced speed for `5.0` seconds |
| `executioner_strike` | attack | `8.0` | weapon | `2d8`, bonus `4`, `STR` every 3 |
| `aimed_shot` | attack | `4.0` | weapon | `1d8`, bonus `2`, `DEX` every 4 |
| `quick_step` | stance | `9.0` | self | `SPEED +1`, `ARMOR +1` for `4.0` seconds |
| `piercing_shot` | attack | `6.0` | weapon | `1d10`, bonus `3`, `DEX` every 4 |
| `hunter_focus` | stance | `10.0` | self | `DMG +2`, `INIT +2` for `6.0` seconds |
| `deadly_shot` | attack | `8.0` | weapon | `2d6`, bonus `4`, `DEX` every 3 |

| Spell | Words of Power | School | Kind | Cost | Cooldown | Range | Roll |
| --- | --- | --- | --- | ---: | ---: | ---: | --- |
| `minor_heal` | `LO VI` | restoration | heal | `3 MP`, `1 blessed_herb` | `4.0` | `5` | `1d6`, bonus `1`, `WIS` every 4 |
| `holy_light` | `YA FUL` | restoration | damage | `4 MP` | `5.0` | `5` | `1d6`, bonus `1`, `WIS` every 4 |
| `blessing` | `YA` | restoration | support | `5 MP` | `8.0` | `5` | `POWER +2`, `RESIST +2` for `8.0s` |
| `turn_undead` | `SAR IR` | divine | control | `5 MP` | `10.0` | `5` | undead target only; `POWER -4`, `SPEED ×0.5` for `5.0s` |
| `greater_heal` | `LO LO` | restoration | heal | `7 MP`, `1 blessed_herb` | `7.0` | `5` | `2d6`, bonus `3`, `WIS` every 4 |
| `smite` | `FUL YA` | divine | damage | `6 MP` | `7.0` | `6` | `2d6`, bonus `2`, `WIS` every 4 |
| `sanctuary` | `SAR VI` | restoration | support | `9 MP` | `12.0` | `5` | `ARMOR +3`, `RESIST +4` for `8.0s` |
| `fire_spark` | `FUL` | fire | damage | `2 MP` | `3.0` | `6` | `1d6`, bonus `0`, `INT` every 4 |

Spell FX use semantic presets from `fx.toml`. The ruleset describes the visual
intent, and the engine maps that to procedural particles and lighting.
Explicit action stages override the official semantic fallbacks. Attacks use
`hit_burst` for an otherwise unspecified impact; healing uses `rising_motes`;
condition actions use `holy_glow` while casting; and conditions share
apply/active/tick/remove defaults. A different ruleset may replace or omit
these mappings without engine changes.

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

Minor quests award `25` XP, major quests award `100` XP, and kill XP starts at
`25` per defender level.

The current maximum level is `10`. Adventuring classes gain their core kit at
level 1 and new actions at levels 2, 4, 6, 8, and 10; odd levels still improve
class resources and primary attributes. Extending the same rules to level 30
only requires additional XP rows and `unlocks.level_N` tables—no representation
or runtime branch change. The runtime treats `max_level` as authoritative for
both explicit XP tables and the optional `xp_for_level` formula form.

## Visual Defaults

Rules and visuals meet in play, so the ruleset provides a consistent default
visual layer.

| Visual rule | Value |
| --- | --- |
| Default avatar | `humanoid` |
| Avatar assets | `assets/humanoid.eldiron_avatar`, `assets/orc.eldiron_avatar`, `assets/skeleton.eldiron_avatar` |
| Ruleset Palette | fixed rules-owned mood palette based on Lospec's "31" palette |
| Explicit override | project `tile_id`, `avatar`, or empty visual fields win |

On load, Eldiron resolves the effective ruleset palette into the project's
Ruleset Palette. This keeps avatar channels, UI color defaults, and generated
missing-art fallbacks on stable rules-owned indices. Authored icon PNGs retain
their own RGBA colors and are not palette-remapped at runtime.

The editable Art Palette remains separate for tiles, drawing, palette-index
geometry sources, and 3D Paint.

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
- more conditions such as stunned, burning, poisoned, and blessed, using the
  existing periodic-effect, lifecycle-event, immunity, and particle vocabulary
- armor proficiency, weapon proficiency, and class restrictions
- encounter templates and automatic arena balance tests
- rarity, value, repair rules, and deeper quality/condition effects
- richer AI intent rules and disposition changes
- localization-ready rule names, messages, and descriptions
- illustrated guide pages and deeper examples

The long-term goal is not a tiny ruleset. The goal is a real world simulation
that starts simple, stays readable, and grows without returning to scattered
per-character configuration.
