---
title: "Rules"
sidebar_position: 6.5
---

**Rules** define project-wide gameplay formulas.

You can change them in the creator via the **Game / Rules** item in the project tree.

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

## Combat Rules

The first system using rules is damage calculation.

### Formula Syntax

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

Available variables in combat formulas:

- `value`: The incoming base amount.
- `attacker.<attr>`: Reads an attacker attribute from data.
- `defender.<attr>`: Reads a defender attribute from data.
- `weapon.<attr>` / `attacker.weapon.<attr>`: Sum of the attacker's equipped weapon-slot item attributes.
- `defender.weapon.<attr>`: Sum of the defender's equipped weapon-slot item attributes.
- `source.<attr>` / `attacker.source.<attr>`: Attribute of the actual weapon or spell item that caused this hit, when available.
- `equipped.<attr>` / `attacker.equipped.<attr>`: Sum of all equipped attacker item attributes.
- `defender.equipped.<attr>`: Sum of all equipped defender item attributes.
- `armor.<attr>`: Sum of the defender's non-weapon equipped item attributes.
- `attacker.armor.<attr>`: Sum of the attacker's non-weapon equipped item attributes.
- `defender.armor.<attr>`: Sum of the defender's non-weapon equipped item attributes.

The weapon and armor groups use the configured slot lists from `Game -> Settings`:

- `game.weapon_slots`
- `game.gear_slots`

Examples:

```toml
[combat]
incoming_damage = "value + attacker.weapon.DMG - defender.armor.ARMOR"
```

```toml
[combat.kinds.physical]
incoming_damage = "value + attacker.STR + attacker.weapon.DMG - defender.equipped.ARMOR"
```

```toml
[combat.kinds.physical]
incoming_damage = "value + source.DMG - defender.armor.ARMOR"
```

If `combat.kinds.<kind>.incoming_damage` exists, it overrides the base combat formula for that damage kind.

## Combat Messages

You can also define automatic combat messages in rules so every monster does not need its own `take_damage` message script.

```toml
[combat.messages]
incoming_key = "combat.damage.incoming"
incoming_category = "warning"
outgoing_key = "combat.damage.outgoing"
outgoing_category = "system"
```

The message key is looked up in **Game / Locales** using the active locale from `Game -> Settings`.

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

Message categories:

- `incoming_category`
- `outgoing_category`

These use the same category string you already use with `message(...)`, for example `system`, `warning`, or custom categories styled by your game.

If you do not want localization for a rule message, you can still use literal `incoming` / `outgoing` strings instead of `incoming_key` / `outgoing_key`.

Automatic combat messages are only sent when a player is involved:

- `incoming`: only if the defender is a player
- `outgoing`: only if the attacker is a player

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

- generated effects from `Game / Audio FX`
- regular audio assets loaded through the audio asset system

Kind overrides work the same way as formula overrides:

```toml
[combat.kinds.fire.audio]
outgoing_fx = "fire_cast"
```

Spells are already connected to this system through `spell_kind`:

- spell items default to `spell_kind = "spell"`
- changing `spell_kind` to `fire`, `ice`, or another custom kind uses the matching `combat.kinds.<kind>` rule path
- the same kind drives damage formulas, combat messages, and combat audio

Weapon and spell items can override the rules-based audio directly with item attributes:

- `attack_fx`
- `attack_bus`
- `attack_gain`
- `hit_fx`
- `hit_bus`
- `hit_gain`

These item-level values take precedence over the global rules audio. This lets one sword, bow, or spell item use its own sound without changing the shared combat defaults.

Combat audio is only played when a player is involved:

- `incoming_fx`: only if the defender is a player
- `outgoing_fx`: only if the attacker is a player

Damage kinds are passed through the runtime payload:

- `deal_damage(...)` defaults to `physical`
- spells default to `spell`
- custom kinds like `fire`, `ice`, or `poison` can override the base rule

After the server resolves the final amount, `take_damage` receives:

- `amount`: final incoming damage
- `from_id`: attacker id
- `damage_kind`: kind string
- `source_item_id`: weapon or spell item id when available
- `attacker_name`: resolved attacker name

The server applies the final damage automatically after `take_damage` returns.

For general localization and built-in `system.*` keys, see [Localization](./localization).
