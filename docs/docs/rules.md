---
title: "Rules"
sidebar_position: 6.5
---

**Rules** define project-wide gameplay formulas.

You can change them in the creator via the **Game / Rules** item in the project tree.

Rules are where the shared game math lives. Instead of repeating the same combat formulas, combat messages, or combat sound logic in every character script, you define them once here and let the engine apply them consistently.

## What Rules Are For

Think of rules as the **global gameplay math layer**.

Scripts should usually decide things like:

- when an NPC attacks
- when it runs away
- when it starts or stops tracking a target
- what event should happen next

Rules should usually decide things like:

- how much damage a hit really does
- how armor reduces damage
- how spells differ from physical attacks
- which combat message should be shown
- which combat sound should play

That keeps character scripts smaller and avoids copying the same combat logic into every NPC.

## Format

Rules use **TOML**.

```toml
[combat]
incoming_damage = "value + attacker.STR - defender.ARMOR"

[combat.messages]
incoming_key = "combat.damage.incoming"
incoming_category = "warning"
outgoing_key = "combat.damage.outgoing"
outgoing_category = "system"

[combat.audio]
incoming_fx = "hit"
outgoing_fx = "attack"

[combat.kinds.physical]
incoming_damage = "value + attacker.STR - defender.ARMOR"

[combat.kinds.spell]
incoming_damage = "value + attacker.INT - defender.RESIST"

[combat.kinds.fire]
incoming_damage = "value + attacker.INT - defender.FIRE_RESIST"
```

## Mental Model

Right now, the normal damage flow looks like this:

1. A script decides to attack and calls `deal_damage(...)`.
2. That call provides a base `value`.
3. The engine resolves:
   - attacker
   - defender
   - damage kind
   - source item, if there is one
4. The matching combat rule calculates the **final incoming damage**.
5. The `take_damage` event runs as the reaction hook.
6. The server applies the final damage automatically.

So:

- **scripts** decide that an attack happens
- **rules** decide what that attack means mathematically
- `take_damage` is for reaction logic, not for repeating combat math

## Formula Syntax

Rules formulas support:

- numbers like `1`, `2.5`, `10.0`
- variables like `value`, `attacker.STR`, `defender.ARMOR`, `source.DMG`
- `+`, `-`, `*`, `/`
- parentheses: `( ... )`
- unary `+` and `-`

Supported helper functions:

- `min(a, b)`
- `max(a, b)`
- `clamp(value, min, max)`
- `abs(x)`
- `floor(x)`
- `ceil(x)`
- `round(x)`

Example:

```toml
[combat]
incoming_damage = "(value + attacker.STR + source.DMG) - defender.armor.ARMOR"
```

The engine already clamps final damage to `>= 0`, so you usually do not need to wrap formulas in `max(0, ...)`.

## Combat Values

Available values in combat formulas:

- `value`: the incoming base amount passed into the rules system
- `attacker.<attr>`: reads an attacker attribute
- `defender.<attr>`: reads a defender attribute
- `weapon.<attr>` / `attacker.weapon.<attr>`: sum of the attacker's equipped weapon-slot item attributes
- `defender.weapon.<attr>`: sum of the defender's equipped weapon-slot item attributes
- `source.<attr>` / `attacker.source.<attr>`: attribute of the actual weapon or spell item that caused this hit, when available
- `equipped.<attr>` / `attacker.equipped.<attr>`: sum of all equipped attacker item attributes
- `defender.equipped.<attr>`: sum of all equipped defender item attributes
- `armor.<attr>`: sum of the defender's non-weapon equipped item attributes
- `attacker.armor.<attr>`: sum of the attacker's non-weapon equipped item attributes
- `defender.armor.<attr>`: sum of the defender's non-weapon equipped item attributes

The weapon and armor groups use the configured slot lists from **Game / Settings**:

- `game.weapon_slots`
- `game.gear_slots`

### Difference Between `weapon.*` and `source.*`

This is important:

- `weapon.<attr>` means the **sum of all equipped weapons** in the configured weapon slots
- `source.<attr>` means the **actual item that caused this hit**

So:

- use `weapon.<attr>` when you want a total from all equipped weapons
- use `source.<attr>` when you want the sword, bow, or spell item that was actually used

## Worked Examples

### Example 1: Basic Physical Damage

```toml
[combat]
incoming_damage = "value + attacker.STR - defender.armor.ARMOR"
```

If:

- `value = 2`
- `attacker.STR = 3`
- `defender.armor.ARMOR = 1`

then:

- final damage = `2 + 3 - 1 = 4`

### Example 2: Weapon Damage from the Actual Source Item

```toml
[combat.kinds.physical]
incoming_damage = "value + source.DMG - defender.armor.ARMOR"
```

If:

- `value = 1`
- the actual sword used has `DMG = 4`
- `defender.armor.ARMOR = 2`

then:

- final damage = `1 + 4 - 2 = 3`

This is usually a better formula than `attacker.weapon.DMG` if you want the hit to depend on the weapon that was actually used.

### Example 3: Sum of Equipped Weapons

```toml
[combat]
incoming_damage = "value + attacker.weapon.DMG - defender.armor.ARMOR"
```

If the attacker has:

- main hand weapon with `DMG = 4`
- off hand weapon with `DMG = 2`

then:

- `attacker.weapon.DMG = 6`

This is useful if your game really wants the total from all equipped weapons. If not, use `source.DMG` instead.

### Example 4: Spell Damage by Kind

```toml
[combat.kinds.spell]
incoming_damage = "value + attacker.INT - defender.RESIST"

[combat.kinds.fire]
incoming_damage = "value + attacker.INT + source.POWER - defender.FIRE_RESIST"
```

If a spell item has:

```toml
spell_kind = "fire"
POWER = 3
```

then the engine uses the `fire` formula instead of the generic `spell` formula.

## Damage Kinds

Kinds let you branch combat rules by damage type.

Common examples:

- `physical`
- `spell`
- `fire`
- `ice`
- `poison`

Behavior:

- `deal_damage(...)` defaults to `physical`
- spells default to `spell`
- custom kinds like `fire` or `ice` can override the base rule

If `combat.kinds.<kind>.incoming_damage` exists, it overrides the base combat formula for that kind.

Spells are already connected to this system through `spell_kind`:

- spell items default to `spell_kind = "spell"`
- changing `spell_kind` to `fire`, `ice`, or another custom kind uses the matching `combat.kinds.<kind>` rule path
- the same kind drives damage formulas, combat messages, and combat audio

## Combat Messages

You can also define automatic combat messages in rules so every monster does not need its own `take_damage` message script.

```toml
[combat.messages]
incoming_key = "combat.damage.incoming"
incoming_category = "warning"
outgoing_key = "combat.damage.outgoing"
outgoing_category = "system"
```

The message key is looked up in **Game / Locales** using the active locale from **Game / Settings**.

```toml
[game]
locale = "en"
```

Example locale entries:

```toml
[en]
combat.damage.incoming = "{attacker} hits you for {amount} damage"
combat.damage.outgoing = "You hit {defender} for {amount} damage"
```

Supported placeholders inside locale strings:

- `{attacker}`
- `{defender}`
- `{amount}`
- `{kind}`
- `{from_id}`
- `{target_id}`

These messages are only sent when a player is involved:

- `incoming`: only if the defender is a player
- `outgoing`: only if the attacker is a player

If you do not want localization for a rule message, you can still use literal `incoming` / `outgoing` strings instead of `incoming_key` / `outgoing_key`.

## Combat Audio

Rules can also trigger built-in or file-based audio clips during combat.

```toml
[combat.audio]
incoming_fx = "hit"
incoming_bus = "sfx"
incoming_gain = 1.0
outgoing_fx = "attack"
outgoing_bus = "sfx"
outgoing_gain = 1.0
```

These names are played through the normal audio system, so they can point to either:

- generated effects from **Audio FX**
- regular audio assets loaded through the audio asset system

Kind overrides work the same way as formula overrides:

```toml
[combat.kinds.fire.audio]
outgoing_fx = "fire_cast"
```

Weapon and spell items can override the rules-based audio directly with item attributes:

- `attack_fx`
- `attack_bus`
- `attack_gain`
- `hit_fx`
- `hit_bus`
- `hit_gain`

These item-level values take precedence over the global rules audio.

Combat audio is only played when a player is involved:

- `incoming_fx`: only if the defender is a player
- `outgoing_fx`: only if the attacker is a player

## What `take_damage` Receives

After the server resolves the final amount, `take_damage` receives:

- `amount`: final incoming damage
- `from_id`: attacker id
- `damage_kind`: kind string
- `source_item_id`: weapon or spell item id when available
- `attacker_name`: resolved attacker name

The server applies the final damage automatically after `take_damage` returns.

So the usual pattern is:

- keep combat math in rules
- use `take_damage` for reaction logic like fleeing, counterattacks, or custom behavior

## Recommended Pattern Right Now

For the current system, this is the intended split:

- use scripts to decide **when** to attack
- use `deal_damage(...)` to send a base amount into the combat system
- use rules to calculate the final damage
- use `take_damage` to react to the hit

This means the following is usually a good script shape:

```eldrin
if event == "attack" {
    let damage = random(1, 3)
    deal_damage(target(), damage)
}
```

and the detailed math should live in rules, not in every NPC script.

Future releases may add higher-level combat helpers on top of this, but the rules system already defines the shared math layer today.

For general localization and built-in `system.*` keys, see [Localization](./localization).
