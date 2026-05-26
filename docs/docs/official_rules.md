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
| Combat kind | `[combat.kinds.fire]` | damage bonuses and reductions |

The current bundled ruleset is assembled from split TOML files under the
`eldiron.official` id.

| Field | Current value |
| --- | --- |
| Ruleset id | `eldiron.official` |
| Name | `Eldiron Official Ruleset` |
| Version | `1.0.0` |
| Schema version | `1` |
| Engine minimum | `0.9.12` |
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
| `MP` / `MAX_MP` | current and maximum mana |
| `ARMOR` | physical protection |
| `RESIST` | general magical protection |
| `FIRE_RESIST` | fire-specific protection |
| `POWER` | general spell power |

Derived stats are table-driven. For example `MAX_HP`, `DMG`, `POWER`, `INIT`,
and `SPEED` can be calculated from level and primary attributes without asking
authors to write formulas on each character.

## Races

Races provide identity, language, visual defaults, attribute defaults, and base
relations.

| Race | Role | Avatar | Languages | Key defaults |
| --- | --- | --- | --- | --- |
| `Human` | balanced default people | `humanoid` | `common` | `HP 10`, all primary attributes `10` |
| `Orc` | strong hostile test race | `humanoid` | `orcish` | `HP 14`, `STR 12`, `DEX 9`, `INT 8`, `WIS 8`, `VIT 12` |

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
</div>

### Warrior

| Area | Rule |
| --- | --- |
| Role | `martial` |
| Primary attributes | `STR`, `VIT` |
| Weapons | sword, axe, mace, spear, bow |
| Armor | cloth, leather, chain, shield |
| Starting health | `HP 16`, `MAX_HP 16` |
| Starter weapon | `training_sword` |
| Starter armor | `padded_armor` |
| Starter clothing | `wool_trousers`, `leather_shoes` |
| Starter inventory | `linen_shirt` |
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
| Starter weapon | `novice_mace` |
| Starter armor | `padded_armor`, `round_shield` |
| Starter clothing | `wool_trousers`, `leather_shoes` |
| Starter inventory | `linen_shirt` |
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
| Starter weapon | `hunting_bow` |
| Starter armor | `leather_vest` |
| Starter clothing | `wool_trousers`, `leather_shoes` |
| Starter inventory | `wooden_arrows`, `linen_shirt` |
| Level 1 abilities | `basic_attack` |

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

Official action distances are resolved before legacy character
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
| `leather_vest` | leather | torso | `2` | torso |
| `chain_shirt` | chain | torso | `3` | torso, arms |
| `round_shield` | shield | shield | `1` | round shield mask |

| Clothing | Family | Slot | Palette color | Avatar channels |
| --- | --- | --- | ---: | --- |
| `linen_shirt` | cloth | torso | `2` | torso, arms |
| `wool_trousers` | cloth | legs | `28` | legs |
| `leather_shoes` | leather | feet | `7` | feet |

| Ammunition | Family | Quantity | Used by | Visual |
| --- | --- | ---: | --- | --- |
| `wooden_arrows` | arrow | `20` | bow | diagonal arrow mask |

Bows consume one matching ammunition item from the attacker's inventory when a
weapon attack resolves. Stackable ammunition decrements its `quantity`; when the
stack reaches zero the inventory slot is emptied.

When an item defines `avatar_channels` and no explicit icon or tile source,
Eldiron derives its preview from the bundled humanoid avatar. Inventory,
equipment, and ground item visuals use the same generated shape.

## Abilities And Spells

Abilities and spells define what exists. Actions define how an actor performs a
gameplay verb. This keeps the current RPG layer compatible with future sandbox
verbs such as harvesting, crafting, lockpicking, stealing, or taming.

| Action | Kind | Target | Cost | Result |
| --- | --- | --- | --- | --- |
| `basic_attack` | attack | hostile entity | - | weapon damage |
| `power_strike` | attack | hostile entity | - | `power_strike` damage |
| `minor_heal` | spell | friendly or self | `3 MP` | `minor_heal` healing |
| `holy_light` | spell | hostile entity | `4 MP` | `holy_light` damage |
| `take` | interaction | ground item | - | move item to inventory |

Action definitions already include a generic `consumes` list, so spells,
crafting, and other sandbox actions can require reagents or materials without a
new hardcoded system.

Abilities are class-owned combat options. Spells add school, cast time, and
damage or healing data. Actions connect those definitions to targets, costs,
cooldowns, results, and FX.

Scripts use `attack()` for the normal weapon attack. Named action buttons or
text commands use `use_action("<id>")`; for example `use_action("power_strike")`
or `use power strike orc` in text play.

| Ability | Kind | Cooldown | Range | Effect |
| --- | --- | ---: | --- | --- |
| `basic_attack` | attack | `1.0` | weapon | normal physical attack |
| `guard` | stance | `3.0` | self | `ARMOR +2` for `2.0` seconds |
| `power_strike` | attack | `4.0` | weapon | `1d8`, bonus `2`, `STR` every 4 |

| Spell | School | Kind | Cost | Cooldown | Range | Roll |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `minor_heal` | restoration | heal | `3 MP` | `4.0` | `5` | `1d6`, bonus `1`, `WIS` every 4 |
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
| Avatar asset | `assets/humanoid.eldiron_avatar` |
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
- Creator integration and rules-aware tools

Expected v1 growth areas:

- more playable classes and enemy roles
- more races and creature templates
- a larger spell and ability catalogue
- crafting professions, reagents, recipes, stations, and item outputs
- loot tables and treasure rules
- conditions such as stunned, burning, poisoned, blessed, and guarded
- armor proficiency, weapon proficiency, and class restrictions
- encounter templates and automatic arena balance tests
- item quality, rarity, value, and repair rules
- richer AI intent rules and disposition changes
- localization-ready rule names, messages, and descriptions
- illustrated guide pages and deeper examples

The long-term goal is not a tiny ruleset. The goal is a real world simulation
that starts simple, stays readable, and grows without returning to scattered
per-character configuration.
